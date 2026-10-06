import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class PreStopTest(unittest.TestCase):
    def run_hook(self, response):
        with tempfile.TemporaryDirectory() as directory:
            curl = Path(directory) / "curl"
            curl.write_text('#!/bin/sh\nprintf "%s" "$FAKE_STATUS"\n')
            curl.chmod(0o755)
            env = dict(os.environ, PATH=directory + ':' + os.environ['PATH'],
                       FAKE_STATUS=response, ZO_ROOT_USER_EMAIL='test@example.com',
                       ZO_ROOT_USER_PASSWORD='test-only', ZO_INGESTER_DRAIN_TIMEOUT_SECS='1')
            return subprocess.run(['sh', str(Path(__file__).with_name('prestop.sh'))],
                                  env=env, capture_output=True, text=True, timeout=10)

    def test_ready_succeeds(self):
        self.assertEqual(self.run_hook('{"readyForShutdown":true,"error":null}').returncode, 0)

    def test_error_is_not_success(self):
        result = self.run_hook('{"readyForShutdown":false,"error":"OSS unavailable"}')
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn('test-only', result.stderr)

    def test_nonempty_queue_times_out(self):
        self.assertNotEqual(self.run_hook('{"readyForShutdown":false,"error":null}').returncode, 0)


if __name__ == '__main__':
    unittest.main()
