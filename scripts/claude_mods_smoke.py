import argparse
import json
import os
import subprocess
import tempfile
from pathlib import Path

from claude_mods.model import ModelServer
from claude_mods.individual import verify_individual
from claude_mods.controls import verify_controls
from claude_mods.scenarios import check_catalog, check_native_turn
from claude_mods.guards import check_scope_guard
from claude_mods.behavior import verify_work_reports, verify_persistence
from claude_mods.transport import AppServer


from claude_mods.fixtures import PINNED_REF, configure, execute


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--mods-ref", default=PINNED_REF)
    parser.add_argument("--mods-checkout", type=Path)
    parser.add_argument(
        "--binary",
        type=Path,
        default=Path(__file__).resolve().parents[1] / "codex-rs/target/debug/codex",
    )
    parser.add_argument("--output", type=Path)
    arguments = parser.parse_args()
    if arguments.mods_ref != PINNED_REF:
        raise ValueError(f"This compatibility suite targets {PINNED_REF}")
    if arguments.output:
        arguments.output.write_text(
            json.dumps({"status": "running", "modsRef": arguments.mods_ref}) + "\n"
        )
    checks = []
    model = ModelServer()
    try:
        fixture_parent = Path(__file__).resolve().parents[1] / ".tmp"
        fixture_parent.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(
            prefix="codex-mods-smoke-", dir=fixture_parent
        ) as temporary:
            root = Path(temporary)
            checkout = arguments.mods_checkout
            if checkout is None:
                checkout = root / "mods"
                subprocess.run(["git", "init", "--quiet", str(checkout)], check=True)
                subprocess.run(
                    [
                        "git",
                        "-C",
                        str(checkout),
                        "fetch",
                        "--depth=1",
                        "https://github.com/theonly1me/claude-code-mods.git",
                        arguments.mods_ref,
                    ],
                    check=True,
                )
                subprocess.run(
                    ["git", "-C", str(checkout), "checkout", "--detach", "FETCH_HEAD"],
                    check=True,
                )
            checkout = checkout.resolve()
            manifest = json.loads(
                checkout.joinpath(".claude-plugin/marketplace.json").read_text()
            )
            names = [plugin["name"] for plugin in manifest["plugins"]]
            assert len(names) == 11, names
            home, project, actual_home, environment = configure(root, model)
            binary = arguments.binary.resolve()
            execute(
                [str(binary), "plugin", "marketplace", "add", str(checkout)],
                environment,
                project,
            )
            for name in names:
                execute(
                    [str(binary), "plugin", "add", f"{name}@claude-code-mods"],
                    environment,
                    project,
                )
            log = (
                arguments.output.with_suffix(".server.log")
                if arguments.output
                else root / "server.log"
            )
            server = AppServer(binary, environment, project, log)
            try:
                started = server.request(
                    "thread/start",
                    {
                        "cwd": str(project),
                        "model": "gpt-6.1-sol",
                        "approvalPolicy": "never",
                        "sandbox": "workspace-write",
                    },
                )
                thread = started["thread"]["id"]
                checks = check_catalog(server, thread, names)
                checks.extend(verify_controls(server, thread, home))
                checks.append(
                    verify_individual(binary, environment, project, names, log)
                )
                checks.extend(check_native_turn(server, thread, project, model))
                checks.append(verify_work_reports(server, thread))
                checks.extend(check_scope_guard(server, thread, project, model))
                helpers = [
                    request
                    for request in model.requests
                    if request.get("max_output_tokens") is not None
                ]
                assert helpers, "No native model helper request reached the provider"
                assert all(request["model"] == "gpt-6.1-sol" for request in helpers)
                assert all(
                    1 <= request["max_output_tokens"] <= 65536 for request in helpers
                )
                checks.append("active Codex model and helper output limits")
                errors = [
                    message
                    for message in server.messages
                    if message.get("method") == "claudeMod/changed"
                    and message["params"]["event"]["kind"] == "error"
                ]
                assert not errors, errors
                assert not actual_home.joinpath(".claude").exists()
                assert home.joinpath("claude-mods/home/.claude").is_dir()
                checks.append("separate Codex mod data")
                result = {
                    "modsRef": arguments.mods_ref,
                    "loaded": len(names),
                    "passed": checks,
                    "failed": [],
                    "providerRequests": len(model.requests),
                    "helperRequests": len(helpers),
                }

            finally:
                if arguments.output:
                    arguments.output.with_suffix(".requests.json").write_text(
                        json.dumps(model.requests, indent=2)
                    )
                    events = [
                        message
                        for message in server.messages
                        if message.get("method") != "claudeMod/changed"
                        or message["params"]["event"]["kind"] != "raster"
                    ]
                    arguments.output.with_suffix(".events.json").write_text(
                        json.dumps(events, indent=2)
                    )
                server.close()
            checks.append(
                verify_persistence(
                    binary,
                    environment,
                    project,
                    names,
                    log.with_suffix(".persistence.log"),
                )
            )
            if os.name == "posix":
                from claude_mods.terminal import verify_terminal

                checks.append(
                    verify_terminal(
                        binary, environment, project, log.with_suffix(".terminal.log")
                    )
                )
        if arguments.output:
            arguments.output.write_text(json.dumps(result, indent=2) + "\n")
        print(json.dumps(result, indent=2))
    except Exception as error:
        if arguments.output:
            arguments.output.write_text(
                json.dumps(
                    {
                        "status": "failed",
                        "modsRef": arguments.mods_ref,
                        "passed": checks,
                        "failed": [str(error)],
                    },
                    indent=2,
                )
                + "\n"
            )
        raise
    finally:
        model.close()


if __name__ == "__main__":
    main()
