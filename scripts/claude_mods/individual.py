import json

from .transport import AppServer


def verify_individual(binary, environment, project, names, output):
    for name in names:
        server = AppServer(
            binary, environment, project, output.with_name(f"individual-{name}.log")
        )
        try:
            settings = {
                candidate + "@claude-code-mods": {"enabled": candidate == name}
                for candidate in names
            }
            started = server.request(
                "thread/start",
                {
                    "cwd": str(project),
                    "model": "gpt-6.1-sol",
                    "config": {"plugins": settings},
                },
            )
            thread = started["thread"]["id"]
            catalog = server.request(
                "claudeMod/list", {"threadId": thread, "interactive": True}
            )
            assert catalog["commands"], name
            assert {command["pluginId"] for command in catalog["commands"]} == {
                name + "@claude-code-mods"
            }
            for command in catalog["commands"]:
                server.request(
                    "claudeMod/command",
                    {
                        "threadId": thread,
                        "pluginId": command["pluginId"],
                        "name": command["name"],
                        "arguments": "",
                    },
                )
        finally:
            server.close()
    return "11 mods initialized and commanded individually"
