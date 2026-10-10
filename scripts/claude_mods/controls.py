import time

from .scenarios import render


def button(node, label):
    if isinstance(node, dict):
        properties = node.get("props", {})
        if node.get("element") == "Button" and properties.get("label") == label:
            return properties["onPress"]["callbackId"]
        children = node.get("children", [])
    elif isinstance(node, list):
        children = node
    else:
        return None
    return next((found for child in children if (found := button(child, label))), None)


def verify_controls(server, thread, home):
    catalog = server.request(
        "claudeMod/list", {"threadId": thread, "interactive": True}
    )
    pane = next(pane for pane in catalog["panes"] if pane["id"] == "change-journal")
    tree = render(
        server, thread, "Pane", pane["id"], {"targetPluginId": pane["pluginId"]}
    )
    callback = button(tree, "Close")
    assert callback, "Change Journal has no close callback"
    server.request(
        "claudeMod/control",
        {
            "threadId": thread,
            "interaction": {"kind": "callback", "callbackId": callback, "value": None},
        },
    )
    catalog = server.request("claudeMod/list", {"threadId": thread})
    assert not any(pane["id"] == "change-journal" for pane in catalog["panes"])
    try:
        server.request(
            "claudeMod/control",
            {
                "threadId": thread,
                "interaction": {
                    "kind": "callback",
                    "callbackId": callback,
                    "value": None,
                },
            },
        )
    except RuntimeError as error:
        assert callback in str(error) and "no longer active" in str(error), error
    else:
        raise AssertionError("Closed pane accepted its stale callback")
    command = next(
        command for command in catalog["commands"] if command["name"] == "changes"
    )
    server.request(
        "claudeMod/command",
        {"threadId": thread, "pluginId": command["pluginId"], "name": command["name"]},
    )
    game = next(
        command
        for command in catalog["commands"]
        if command["pluginId"].startswith("samurai-dojo@")
    )
    server.request(
        "claudeMod/command",
        {
            "threadId": thread,
            "pluginId": game["pluginId"],
            "name": game["name"],
            "arguments": "on",
        },
    )
    tree = render(
        server, thread, "Pane", "samurai-dojo", {"targetPluginId": game["pluginId"]}
    )
    assert "Raster" in str(tree), tree
    deadline = time.monotonic() + 3
    frames = set()
    while time.monotonic() < deadline:
        server.request("claudeMod/list", {"threadId": thread, "interactive": True})
        frames.update(
            message["params"]["event"]["cells"]
            for message in server.messages
            if message.get("method") == "claudeMod/changed"
            and message["params"]["event"]["kind"] == "raster"
        )
        if len(frames) >= 2:
            break
        time.sleep(0.05)
    assert len(frames) >= 2, "No changing raster frames reached the client"
    server.request(
        "claudeMod/control",
        {
            "threadId": thread,
            "interaction": {
                "kind": "close",
                "pluginId": game["pluginId"],
                "id": "samurai-dojo",
            },
        },
    )
    assert home.joinpath("claude-mods/home/.claude/mods/active-game").read_text() == ""
    return [
        "native pane controls and stale callback rejection",
        "changing raster frames through timer notifications",
        "person close clears the saved active game",
    ]
