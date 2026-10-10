import json
import time

from .model import patch as patch_call
from .model import tool


def render(server, thread, component, identity, properties=None, columns=80):
    return server.request(
        "claudeMod/render",
        {
            "threadId": thread,
            "render": {
                "component": component,
                "requestId": identity,
                "columns": columns,
                "rows": 20,
                "isFocused": True,
                "isWorking": False,
                "props": properties or {},
            },
        },
    )["tree"]


def wait_render(server, thread, component, identity, expected, properties=None):
    deadline = time.monotonic() + 10
    while time.monotonic() < deadline:
        tree = render(server, thread, component, identity, properties)
        if expected.lower() in json.dumps(tree).lower():
            return tree
        time.sleep(0.05)
    raise AssertionError(f"{identity}: render never contained {expected}")


def check_catalog(server, thread, names):
    catalog = server.request(
        "claudeMod/list", {"threadId": thread, "interactive": True}
    )
    commands = catalog["commands"]
    assert len(commands) == 12, commands
    assert {command["pluginId"].split("@")[0] for command in commands} == set(names)
    for command in commands:
        server.request(
            "claudeMod/command",
            {
                "threadId": thread,
                "pluginId": command["pluginId"],
                "name": command["name"],
                "arguments": "",
            },
        )
    catalog = server.request(
        "claudeMod/list", {"threadId": thread, "interactive": True}
    )
    assert len(catalog["panes"]) >= 9, catalog
    for pane in catalog["panes"]:
        properties = {
            "id": pane["id"],
            "placement": "dock",
            "targetPluginId": pane["pluginId"],
        }
        tree = render(server, thread, "Pane", pane["id"], properties)
        assert tree, pane
        narrow = render(server, thread, "Pane", pane["id"], properties, columns=40)
        assert narrow, pane
    return [
        "11 mods discovered through native plugin installation",
        "12 app-server slash commands",
        "9 panes and narrow viewport rendering",
    ]


def check_native_turn(server, thread, project, model):
    patch = "*** Begin Patch\n*** Update File: value.txt\n@@\n-one\n+two\n*** End Patch"
    model.next_items.extend(
        [
            [patch_call(patch, "update-value")],
            [
                tool(
                    "exec_command",
                    {"cmd": "printf '1 passed\\n'", "login": False},
                    "check-value",
                )
            ],
        ]
    )
    server.turn(
        thread,
        "Change value.txt from one to two. Run its check. Do not change other files.",
    )
    assert project.joinpath("value.txt").read_text() == "two\n"
    primary = [
        request
        for request in model.requests
        if request.get("max_output_tokens") is None
        and (
            request.get("tools")
            or any(
                item.get("type") == "additional_tools" and item.get("tools")
                for item in request.get("input", [])
            )
        )
    ]
    assert primary and all(
        "Write your chat replies to the user in ASD-STE100 Simplified Technical English"
        in request.get("instructions", "") + json.dumps(request.get("input", []))
        for request in primary
    ), "Unslop style is absent from the native sampling requests"
    orphaned = [
        message
        for message in server.messages
        if message.get("method") in ["item/started", "item/completed"]
        and message["params"]["item"]["id"].startswith("mod-")
    ]
    assert not orphaned, orphaned
    journal = render(
        server,
        thread,
        "Pane",
        "change-journal",
        {"targetPluginId": "change-journal@claude-code-mods"},
    )
    assert "value.txt" in json.dumps(journal), journal
    wait_render(
        server,
        thread,
        "Pane",
        "decision-lens",
        "native patch",
        {"targetPluginId": "decision-lens@claude-code-mods"},
    )
    wait_render(
        server,
        thread,
        "AbovePrompt",
        "AbovePrompt",
        "Which result",
        {"hasSurvey": False},
    )
    return [
        "native patch and shell outcomes",
        "Unslop adds its style rules to native model instructions",
        "Change Journal edits and summary",
        "Decision Lens helper analysis",
        "Grill background questions",
    ]
