import json
import re
import time

from .scenarios import render
from .transport import AppServer


def command(server, thread, catalog, name, arguments):
    entry = next(entry for entry in catalog["commands"] if entry["name"] == name)
    return server.request(
        "claudeMod/command",
        {
            "threadId": thread,
            "pluginId": entry["pluginId"],
            "name": name,
            "arguments": arguments,
        },
    )["text"]


def verify_work_reports(server, thread):
    catalog = server.request("claudeMod/list", {"threadId": thread})
    reports = {
        name: command(server, thread, catalog, name, "stats")
        for name in ["dojo", "slayer", "feast", "farm", "pet"]
    }
    expected = {
        "dojo": r"Slain this session: ([0-9]+)",
        "slayer": r"Lifetime: ([0-9]+) attacks",
        "feast": r"Lifetime feeds: ([0-9]+)",
        "pet": r"XP ([0-9]+)",
    }
    for name, pattern in expected.items():
        deadline = time.monotonic() + 15
        while True:
            match = re.search(pattern, reports[name])
            if match and int(match.group(1)) > 0:
                break
            if time.monotonic() >= deadline:
                break
            time.sleep(0.05)
            reports[name] = command(server, thread, catalog, name, "stats")
        assert match and int(match.group(1)) > 0, (name, reports[name])
    assert "value.txt" in reports["farm"], reports["farm"]
    tree = render(
        server,
        thread,
        "Pane",
        "looks-theme",
        {"targetPluginId": "looks@claude-code-mods"},
    )
    assert "Retro" in json.dumps(tree), tree
    return "five game reports observe native work and Looks renders its picker"


def verify_persistence(binary, environment, project, names, log):
    server = AppServer(binary, environment, project, log)
    try:
        settings = {
            name + "@claude-code-mods": {"enabled": name == "samurai-dojo"}
            for name in names
        }
        result = server.request(
            "thread/start",
            {
                "cwd": str(project),
                "model": "gpt-6.1-sol",
                "config": {"plugins": settings},
            },
        )
        thread = result["thread"]["id"]
        catalog = server.request(
            "claudeMod/list", {"threadId": thread, "interactive": True}
        )
        report = command(server, thread, catalog, "dojo", "stats")
        match = re.search(r"Lifetime: ([0-9]+)", report)
        assert match and int(match.group(1)) >= 2, report
        assert "Slain this session: 0." in report, report
        return "game lifetime state survives a native process restart"
    finally:
        server.close()
