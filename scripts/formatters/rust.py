import os
import subprocess
from pathlib import Path


def commands(repository_root: Path, *, check: bool) -> tuple[tuple[str, ...], ...]:
    files = subprocess.check_output(
        [
            "git",
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
            "--",
            "*.rs",
        ],
        cwd=repository_root,
    ).split(b"\0")
    workspace = repository_root / "codex-rs"
    paths = sorted(
        {
            os.path.relpath(repository_root / os.fsdecode(path), workspace)
            for path in files
            if path and (repository_root / os.fsdecode(path)).is_file()
        }
    )
    arguments = [
        "rustfmt",
        "--edition",
        "2024",
        "--config-path",
        str(workspace / "rustfmt.toml"),
        "--config",
        "imports_granularity=Item,skip_children=true",
    ]
    if check:
        arguments.append("--check")
    batches: list[tuple[str, ...]] = []
    batch: list[str] = []
    size = 0
    for path in paths:
        path_size = len(os.fsencode(path)) + 3
        if batch and size + path_size > 16_000:
            batches.append((*arguments, *batch))
            batch = []
            size = 0
        batch.append(path)
        size += path_size
    if batch:
        batches.append((*arguments, *batch))
    return tuple(batches)
