from contextlib import ExitStack

import json
import os
import queue
import signal
import subprocess
import threading
import time


class AppServer:
    def __init__(self, binary, environment, project, log):
        self.resources = ExitStack()
        self.log = self.resources.enter_context(open(log, "w", encoding="utf-8"))
        self.process = subprocess.Popen(
            [str(binary), "app-server"],
            cwd=project,
            env=environment,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=self.log,
            text=True,
            bufsize=1,
            start_new_session=True,
        )
        self.incoming = queue.Queue()
        self.messages = []
        self.sequence = 0
        threading.Thread(target=self.read, daemon=True).start()
        self.request(
            "initialize",
            {
                "clientInfo": {"name": "claude_mods_smoke", "version": "1.0"},
                "capabilities": {"experimentalApi": True},
            },
        )
        self.send({"method": "initialized"})

    def read(self):
        for line in self.process.stdout:
            self.incoming.put(json.loads(line))
        self.incoming.put(None)

    def send(self, message):
        self.process.stdin.write(json.dumps(message) + "\n")
        self.process.stdin.flush()

    def receive(self, deadline):
        message = self.incoming.get(timeout=max(0.01, deadline - time.monotonic()))
        if message is None:
            raise RuntimeError(f"app-server exited with {self.process.poll()}")
        self.messages.append(message)
        if "id" in message and "method" in message:
            method = message["method"]
            if method == "item/tool/requestUserInput":
                questions = message["params"]["questions"]
                answers = {}
                for question in questions:
                    options = question.get("options", [])
                    label = next(
                        (
                            option["label"]
                            for option in options
                            if option["label"] == "Stop"
                        ),
                        options[0]["label"] if options else "Stop",
                    )
                    answers[question["id"]] = {"answers": [label]}
                self.send({"id": message["id"], "result": {"answers": answers}})
            else:
                raise RuntimeError(f"unexpected native approval request: {method}")
        return message

    def request(self, method, parameters, timeout=45):
        self.sequence += 1
        identity = self.sequence
        self.send({"id": identity, "method": method, "params": parameters})
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            message = self.receive(deadline)
            if message.get("id") == identity and "method" not in message:
                if "error" in message:
                    raise RuntimeError(f"{method}: {message['error']}")
                return message["result"]
        raise TimeoutError(method)

    def turn(self, thread, text):
        result = self.request(
            "turn/start",
            {"threadId": thread, "input": [{"type": "text", "text": text}]},
        )
        turn = result["turn"]["id"]
        deadline = time.monotonic() + 60
        while time.monotonic() < deadline:
            message = self.receive(deadline)
            if message.get("method") == "turn/completed":
                completed = message["params"]["turn"]
                if completed["id"] == turn:
                    if completed.get("error"):
                        raise RuntimeError(completed["error"])
                    return completed
        raise TimeoutError("native turn completion")

    def close(self):
        self.process.stdin.close()
        try:
            self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.process.terminate()
            self.process.wait(timeout=10)
        try:
            if os.name == "posix":
                os.killpg(self.process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        self.resources.close()
