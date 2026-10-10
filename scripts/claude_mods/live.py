import http.server
import json
import os
import shutil
import tempfile
import threading
from pathlib import Path

from .transport import AppServer


def verify_live(binary, output):
    auth_path = Path.home() / ".codex/auth.json"
    if not auth_path.is_file():
        return {
            "status": "notRun",
            "reason": "File credentials are unavailable for an isolated live call.",
        }

    class RejectRefresh(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_arguments):
            return

        def do_POST(self):
            self.send_response(403)
            self.end_headers()

    refresh = http.server.ThreadingHTTPServer(("127.0.0.1", 0), RejectRefresh)
    threading.Thread(target=refresh.serve_forever, daemon=True).start()
    try:
        with tempfile.TemporaryDirectory(prefix="codex-mods-live-") as temporary:
            root = Path(temporary)
            home = root / "home"
            plugin = root / "plugin"
            home.mkdir()
            plugin.joinpath(".claude-plugin").mkdir(parents=True)
            plugin.joinpath("hooks").mkdir()
            shutil.copyfile(auth_path, home / "auth.json")
            home.joinpath("auth.json").chmod(0o600)
            source = Path.home() / ".codex/config.toml"
            import tomllib

            model = (
                tomllib.loads(source.read_text()).get("model", "gpt-6.1-sol")
                if source.exists()
                else "gpt-6.1-sol"
            )
            plugin.joinpath(".claude-plugin/plugin.json").write_text(
                json.dumps({"name": "live-helper", "version": "1.0.0"})
            )
            plugin.joinpath("hooks/hooks.json").write_text(
                json.dumps({"modules": ["./register.ts"]})
            )
            plugin.joinpath("hooks/register.ts").write_text("""
export function register(on) {
  on('session.start', async ($, event, next) => {
    await $.command.register({name:'live-helper'});
    return next(event);
  });
  on('command.run', async ($) => {
    const result = await $.model.complete({prompt:'Reply with OK.', maxTokens:512, timeoutMs:30000});
    return {text:JSON.stringify(result)};
  });
}
""")
            root.joinpath(".claude-plugin").mkdir()
            root.joinpath(".claude-plugin/marketplace.json").write_text(
                json.dumps(
                    {
                        "name": "live-test",
                        "plugins": [{"name": "live-helper", "source": "./plugin"}],
                    }
                )
            )
            home.joinpath("config.toml").write_text(
                f'model = {json.dumps(model)}\ncli_auth_credentials_store = "file"\n[features]\nplugins = true\nremote_plugin = false\n[marketplaces.live-test]\nsource_type = "local"\nsource = {json.dumps(str(root))}\n[plugins."live-helper@live-test"]\nenabled = true\n'
            )
            environment = dict(
                os.environ,
                CODEX_HOME=str(home),
                CODEX_REFRESH_TOKEN_URL_OVERRIDE=f"http://127.0.0.1:{refresh.server_port}/reject",
            )
            import subprocess

            install = subprocess.run(
                [str(binary), "plugin", "add", "live-helper@live-test"],
                check=False,
                env=environment,
                cwd=root,
                text=True,
                capture_output=True,
                timeout=45,
            )
            if install.returncode:
                return {
                    "status": "failed",
                    "reason": "The isolated live plugin did not install.",
                }
            server = AppServer(
                binary, environment, root, output.with_suffix(".server.log")
            )
            try:
                models = server.request("model/list", {})["data"]
                available = next(
                    (entry for entry in models if entry.get("isDefault")), models[0]
                )
                model = available["id"]
                thread = server.request(
                    "thread/start", {"cwd": str(root), "model": model}
                )["thread"]["id"]
                reply = server.request(
                    "claudeMod/command",
                    {
                        "threadId": thread,
                        "pluginId": "live-helper@live-test",
                        "name": "live-helper",
                    },
                )
                result = json.loads(reply["text"])
                assert result["isAnswered"], result
                return {
                    "status": "passed",
                    "model": result["model"],
                    "text": result["text"],
                    "usage": result["usage"],
                    "outputLimit": 512,
                }
            finally:
                server.close()
    except (RuntimeError, TimeoutError, OSError) as error:
        return {"status": "failed", "reason": str(error)[:600]}
    finally:
        refresh.shutdown()
        refresh.server_close()
