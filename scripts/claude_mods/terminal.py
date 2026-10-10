import fcntl
import json
import os
import pty
import re
import select
import signal
import struct
import subprocess
import termios
import time

from .fixtures import execute


class Terminal:
    def __init__(self, binary, environment, project, only_plugin=None, inline=False):
        self.master, slave = pty.openpty()
        self.resize(100, 40)
        settings = [
            "check_for_update_on_startup=false",
            f'projects.{json.dumps(str(project))}.trust_level="trusted"',
        ]
        if only_plugin:
            installed = json.loads(
                execute([str(binary), "plugin", "list", "--json"], environment, project)
            )["installed"]
            for entry in installed:
                plugin = entry["name"] + "@" + entry["marketplaceName"]
                enabled = plugin.split("@")[0] == only_plugin
                settings.append(f"plugins.{plugin}.enabled={str(enabled).lower()}")
        arguments = [str(binary)]
        if inline:
            arguments.append("--no-alt-screen")
        for setting in settings:
            arguments.extend(["-c", setting])
        self.process = subprocess.Popen(
            arguments,
            cwd=project,
            env=dict(environment, TERM="xterm-256color", COLORTERM="truecolor"),
            stdin=slave,
            stdout=slave,
            stderr=slave,
            start_new_session=True,
        )
        os.close(slave)
        self.output = bytearray()
        self.transcript = bytearray()

    def resize(self, columns, rows):
        fcntl.ioctl(
            self.master, termios.TIOCSWINSZ, struct.pack("HHHH", rows, columns, 0, 0)
        )

    def write(self, text):
        os.write(self.master, text.encode())

    def submit(self, text):
        self.write(text)
        self.read(0.2)
        self.write("\r")

    def read(self, duration=0.2):
        deadline = time.monotonic() + duration
        while time.monotonic() < deadline:
            readable, _, _ = select.select(
                [self.master], [], [], max(0, deadline - time.monotonic())
            )
            if not readable:
                break
            try:
                data = os.read(self.master, 65536)
            except OSError:
                break
            self.output.extend(data)
            self.transcript.extend(data)
            if b"\x1b[6n" in data:
                self.write("\x1b[1;1R")
            if b"\x1b[?2026$p" in data:
                self.write("\x1b[?2026;2$y")
            if b"\x1b[c" in data:
                self.write("\x1b[?1;2c")
            if b"\x1b[>c" in data:
                self.write("\x1b[>0;0;0c")
        return re.sub(
            r"\x1b\[[0-?]*[ -/]*[@-~]", "", self.output.decode(errors="replace")
        )

    def wait(self, expected, timeout=15):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            text = self.read()
            compact_text = re.sub(r"\s+", "", text.casefold())
            compact_expected = re.sub(r"\s+", "", expected.casefold())
            if compact_expected in compact_text:
                return text
            if self.process.poll() is not None:
                break
        raise AssertionError(
            f"The native terminal did not show {expected!r}: {text[-5000:]}"
        )

    def clear(self):
        self.read()
        self.output.clear()

    def close(self):
        self.write("\x03\x03")
        try:
            self.process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            os.killpg(self.process.pid, signal.SIGTERM)
            self.process.wait(timeout=5)
        try:
            os.killpg(self.process.pid, signal.SIGTERM)
        except ProcessLookupError:
            pass
        os.close(self.master)


def verify_terminal(binary, environment, project, log):
    from .animation import verify_animation

    verify_animation(binary, environment, project, log)
    terminal = Terminal(binary, environment, project)
    try:
        terminal.wait("gpt-6.1-sol")
        terminal.read(1)
        terminal.clear()
        terminal.submit("/dojo on")
        terminal.wait("Samurai Dojo is on")
        terminal.read(0.5)
        terminal.clear()
        terminal.resize(42, 30)
        os.kill(terminal.process.pid, signal.SIGWINCH)
        terminal.wait("Widen this pane")
        terminal.resize(100, 40)
        os.kill(terminal.process.pid, signal.SIGWINCH)
        terminal.read(0.5)
        terminal.clear()
        terminal.submit("/changes")
        terminal.wait("Change Journal")
        terminal.wait("[Close]")
        terminal.clear()
        terminal.write("x")
        terminal.read(0.5)
        terminal.submit("/dojo stats")
        terminal.wait("Samurai")
        terminal.close()
        terminal = Terminal(binary, environment, project, only_plugin="grill")
        terminal.wait("gpt-6.1-sol")
        terminal.read(1)
        terminal.submit(
            "Explain the current contents of value.txt and describe its purpose."
        )
        terminal.wait("Which result")
        terminal.clear()
        terminal.write("\x1b[17~")
        terminal.read(0.5)
        terminal.write("1")
        terminal.wait("Claude finished before reading")
        terminal.write("\x1b")
        terminal.read(0.5)
        terminal.submit("/grill")
        terminal.read(0.5)
        return "native CLI default and inline animation, resize, pane controls, and Grill band focus"
    finally:
        log.write_bytes(terminal.transcript)
        terminal.close()
