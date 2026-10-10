import json
import os
import subprocess
import sys
import unittest
from pathlib import Path
from tempfile import TemporaryDirectory


class WindowsBazelAbiTest(unittest.TestCase):
    def test_keyless_cross_build_keeps_gnullvm_targets_and_msvc_host_tools_separate(
        self,
    ):
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
            environment = dict(os.environ)
            environment.update(
                {
                    "RUNNER_OS": "Windows",
                    "BUILDBUDDY_API_KEY": "",
                    "CODEX_BAZEL_BIN": str(executable),
                    "CODEX_BAZEL_WINDOWS_PATH": str(folder),
                    "CAPTURED_ARGUMENTS": str(arguments),
                }
            )
            completed = subprocess.run(
                [
                    "bash",
                    str(root / ".github/scripts/run-bazel-ci.sh"),
                    "--windows-cross-compile",
                    "--",
                    "test",
                    "--",
                    "//codex-rs/utils/rustls-provider:rustls-provider-preinstalled-test",
                ],
                cwd=root,
                env=environment,
                text=True,
                capture_output=True,
            )
            self.assertEqual(completed.returncode, 0, completed.stderr)
            flags = json.loads(arguments.read_text())
            self.assertIn("--platforms=//:windows_x86_64_gnullvm", flags)
            self.assertIn("--host_platform=//:local_windows_msvc", flags)
            self.assertIn("--repo_env=BAZEL_DO_NOT_DETECT_CPP_TOOLCHAIN=0", flags)
            self.assertIn(
                "--extra_toolchains=//.github/fork:native_windows_msvc", flags
            )


if __name__ == "__main__":
    unittest.main()
