import json
import os
import subprocess
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory


class CiMsvcOptInTest(unittest.TestCase):
    def test_runtime_flag_requires_windows_and_an_operator_opt_in(self):
        root = Path(__file__).resolve().parents[2]
        for platform, accepted, expected in [
            ("Windows", "", False),
            ("Windows", "0", False),
            ("Windows", "1", True),
            ("Linux", "1", False),
        ]:
            with self.subTest(platform=platform, accepted=accepted):
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
                    environment = dict(os.environ)
                    environment.update(
                        {
                            "RUNNER_OS": platform,
                            "BUILDBUDDY_API_KEY": "",
                            "CODEX_BAZEL_BIN": str(executable),
                            "CODEX_BAZEL_WINDOWS_PATH": str(folder),
                            "CODEX_CI_MSVC_RUNTIME_LICENSE_ACCEPTED": accepted,
                            "CAPTURED_ARGUMENTS": str(arguments),
                        }
                    )
                    completed = subprocess.run(
                        [
                            "bash",
                            str(root / ".github/scripts/run-bazel-ci.sh"),
                            "--",
                            "build",
                            "--",
                            "//codex-rs/cli:codex",
                        ],
                        cwd=root,
                        env=environment,
                        text=True,
                        capture_output=True,
                    )
                    self.assertEqual(completed.returncode, 0, completed.stderr)
                    flags = json.loads(arguments.read_text())
                    self.assertEqual(
                        "--repo_env=BAZEL_MSVC_RUNTIME_VISUAL_STUDIO_EULA=1" in flags,
                        expected,
                    )


if __name__ == "__main__":
    unittest.main()
