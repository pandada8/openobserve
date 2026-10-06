use std::{
    io,
    path::Path,
    sync::{LazyLock, Mutex},
    time::Duration,
};

use serde::Serialize;

static RUNNING: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static STATUS: LazyLock<Mutex<DrainStatus>> = LazyLock::new(|| Mutex::new(DrainStatus::default()));

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DrainStatus {
    pub is_draining: bool,
    pub memory_flushed: bool,
    pub ready_for_shutdown: bool,
    pub pending_files: usize,
    pub pending_segments: usize,
    pub active_requests: usize,
    pub active_writes: usize,
    pub error: Option<String>,
}

pub fn status() -> DrainStatus {
    let mut status = STATUS.lock().unwrap().clone();
    status.is_draining = config::drain::is_draining();
    (status.active_requests, status.active_writes) = config::drain::counts();
    status
}

pub async fn run() -> Result<(), String> {
    config::drain::begin();
    let _lock = RUNNING.lock().await;
    if status().ready_for_shutdown {
        return Ok(());
    }
    let timeout = Duration::from_secs(config::get_config().common.ingester_drain_timeout_secs);
    let result = tokio::time::timeout(timeout, drain())
        .await
        .unwrap_or_else(|_| {
            Err("Drain timed out; preserve PVC and retry drain before scaling down".to_string())
        });
    STATUS.lock().unwrap().error = result.as_ref().err().cloned();
    result
}

async fn drain() -> Result<(), String> {
    while !config::drain::seal_if_idle() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    crate::flush_all().await.map_err(|e| e.to_string())?;
    STATUS.lock().unwrap().memory_flushed = true;
    loop {
        crate::flush_all().await.map_err(|e| e.to_string())?;
        let immutable = crate::get_immutables_cache_stats().await.0;
        let persisting = crate::get_processing_tables_cache_stats().await.0;
        let segments = crate::get_segment_index_stats().await.1;
        let root = config::get_config().common.data_wal_dir.clone();
        let files = tokio::task::spawn_blocking(move || count_pending(Path::new(&root)))
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        {
            let mut status = STATUS.lock().unwrap();
            status.pending_files = files;
            status.pending_segments = segments;
            if immutable == 0 && persisting == 0 && segments == 0 && files == 0 {
                status.ready_for_shutdown = true;
                return Ok(());
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

fn count_pending(root: &Path) -> io::Result<usize> {
    std::fs::read_dir(root)?;
    let mut count = 0;
    for directory in ["logs", "files", "pack"] {
        count += count_files(&root.join(directory))?;
    }
    Ok(count)
}

fn count_files(path: &Path) -> io::Result<usize> {
    let entries = match std::fs::read_dir(path) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
        Err(e) => return Err(e),
    };
    let mut count = 0;
    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            count += count_files(&entry.path())?;
        } else {
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unfinished_and_unknown_files_block_completion() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(count_pending(root.path()).unwrap(), 0);
        for name in [
            "logs/1.wal",
            "files/1.par",
            "files/2.parquet",
            "pack/1.pack.tmp",
        ] {
            let path = root.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"pending").unwrap();
        }
        assert_eq!(count_pending(root.path()).unwrap(), 4);
    }
}
