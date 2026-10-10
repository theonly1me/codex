import os
import subprocess
from pathlib import Path

PINNED_REF = "6d69bf8871bb088c3bdf5b5526b03dce633cdb88"


def execute(arguments, environment, project):
    result = subprocess.run(
        arguments,
        env=environment,
        cwd=project,
        text=True,
        capture_output=True,
        timeout=60,
    )
    if result.returncode:
        raise RuntimeError(f"{arguments}: {result.stderr}\n{result.stdout}")
    return result.stdout


def configure(root, model):
    home = root / "codex-home"
    project = root / "project"
    actual_home = root / "actual-home"
    for path in [home, project, actual_home]:
        path.mkdir()
    environment = dict(os.environ, CODEX_HOME=str(home), HOME=str(actual_home))
    home.joinpath("config.toml").write_text(
        f'''model = "gpt-6.1-sol"
model_provider = "mods-smoke"
model_catalog_json = "{Path(__file__).resolve().parents[2] / "codex-rs/models-manager/models.json"}"
approval_policy = "never"
sandbox_mode = "workspace-write"
[features]
plugins = true
remote_plugin = false
[model_providers.mods-smoke]
name = "Isolated mods verification"
base_url = "{model.url}"
wire_api = "responses"
requires_openai_auth = false
supports_websockets = false
'''
    )
    execute(["git", "init", "--quiet"], environment, project)
    project.joinpath("value.txt").write_text("one\n")
    execute(["git", "add", "value.txt"], environment, project)
    execute(
        [
            "git",
            "-c",
            "user.name=Mods smoke",
            "-c",
            "user.email=mods@example.invalid",
            "commit",
            "-m",
            "test(mods): add initial fixture",
        ],
        environment,
        project,
    )
    return home, project, actual_home, environment
