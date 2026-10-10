import json
import os
import subprocess
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory


class HostedBazelTestResourcesTest(unittest.TestCase):
    def test_hosted_local_tests_limit_parallelism_without_excluding_tests(self):
        root = Path(__file__).resolve().parents[2]
        with TemporaryDirectory() as temporary:
            folder = Path(temporary)
            arguments = folder / "arguments.json"
            executable = folder / "bazel"
            executable.write_text(
                f"#!{sys.executable}\n"
                "import json, os, sys\n"
                "from pathlib import Path\n"
                "Path(os.environ['CAPTURED_ARGUMENTS']).write_text(json.dumps(sys.argv[1:]))\n"
            )
            executable.chmod(0o755)
            for runner in ["Linux", "macOS"]:
                environment = dict(os.environ)
                environment.update(
                    {
                        "RUNNER_OS": runner,
                        "RUNNER_ENVIRONMENT": "github-hosted",
                        "BUILDBUDDY_API_KEY": "",
                        "CODEX_BAZEL_BIN": str(executable),
                        "CAPTURED_ARGUMENTS": str(arguments),
                    }
                )
                completed = subprocess.run(
                    [
                        "bash",
                        str(root / ".github/scripts/run-bazel-ci.sh"),
                        "--",
                        "test",
                        "--",
                        "//...",
                    ],
                    cwd=root,
                    env=environment,
                    text=True,
                    capture_output=True,
                )
                self.assertEqual(completed.returncode, 0, completed.stderr)
                flags = json.loads(arguments.read_text())
                self.assertIn("--local_test_jobs=2", flags)
                self.assertIn("--test_env=RUST_TEST_THREADS=1", flags)
                self.assertIn("//...", flags)
                self.assertFalse(
                    any("CODEX_BAZEL_TEST_SKIP_FILTERS" in flag for flag in flags)
                )


if __name__ == "__main__":
    unittest.main()
