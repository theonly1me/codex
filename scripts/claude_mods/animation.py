import os
import signal

from .terminal import Terminal


def scene_cells(output):
    text = output.decode(errors="replace")
    return text.count("▀") + text.count("▄")


def verify_animation(binary, environment, project, log):
    for inline in [False, True]:
        mode = "inline" if inline else "fullscreen"
        terminal = Terminal(
            binary, environment, project, only_plugin="samurai-dojo", inline=inline
        )
        try:
            terminal.wait("gpt-6.1-sol")
            terminal.read(1)
            terminal.clear()
            terminal.submit("/dojo on")
            terminal.wait("Samurai Dojo is on")
            terminal.read(0.5)
            assert scene_cells(terminal.output) > 100, (
                f"{mode}: The dojo command returned, but the terminal shows no scene"
            )
            terminal.clear()
            terminal.read(1)
            assert scene_cells(terminal.output) > 0, (
                f"{mode}: The terminal shows no changing dojo animation"
            )
            terminal.resize(42, 30)
            os.kill(terminal.process.pid, signal.SIGWINCH)
            terminal.wait("Widen this pane")
            terminal.clear()
            terminal.resize(100, 40)
            os.kill(terminal.process.pid, signal.SIGWINCH)
            terminal.read(1)
            assert scene_cells(terminal.output) > 100, (
                f"{mode}: The dojo scene did not return after the terminal widened"
            )
        finally:
            log.with_suffix(f".{mode}.log").write_bytes(terminal.transcript)
            terminal.close()
