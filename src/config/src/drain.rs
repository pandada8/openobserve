use std::sync::{LazyLock, Mutex};

static GATE: LazyLock<Mutex<Gate>> = LazyLock::new(|| Mutex::new(Gate::default()));

#[derive(Default)]
struct Gate {
    draining: bool,
    sealed: bool,
    requests: usize,
    writes: usize,
}

pub struct Admission {
    write: bool,
}

impl Drop for Admission {
    fn drop(&mut self) {
        let mut gate = GATE.lock().unwrap();
        if self.write {
            gate.writes -= 1;
        } else {
            gate.requests -= 1;
        }
    }
}

pub fn admit(write: bool) -> Option<Admission> {
    let mut gate = GATE.lock().unwrap();
    if (write && gate.sealed) || (!write && gate.draining) {
        return None;
    }
    if write {
        gate.writes += 1;
    } else {
        gate.requests += 1;
    }
    Some(Admission { write })
}

pub fn begin() {
    GATE.lock().unwrap().draining = true;
}

pub fn is_draining() -> bool {
    GATE.lock().unwrap().draining
}

pub fn seal_if_idle() -> bool {
    let mut gate = GATE.lock().unwrap();
    if !gate.draining || gate.requests != 0 {
        return false;
    }
    gate.sealed = true;
    gate.writes == 0
}

pub fn counts() -> (usize, usize) {
    let gate = GATE.lock().unwrap();
    (gate.requests, gate.writes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_requests_finish_before_writers_are_sealed() {
        let request = admit(false).unwrap();
        begin();
        assert!(admit(false).is_none());
        assert!(!seal_if_idle());
        let write = admit(true).unwrap();
        drop(request);
        assert!(!seal_if_idle());
        assert!(admit(true).is_none());
        drop(write);
        assert!(seal_if_idle());
        assert_eq!(counts(), (0, 0));
    }
}
