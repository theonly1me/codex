from .model import patch, tool


def check_scope_guard(server, thread, project, model):
    original = project.joinpath("value.txt").read_text()
    changes = [
        "*** Begin Patch\n*** Add File: unrelated/extra.txt\n+outside the request\n*** End Patch",
        "*** Begin Patch\n*** Update File: value.txt\n@@\n-two\n+three\n*** Add File: unrelated/extra.txt\n+outside the request\n*** End Patch",
        "*** Begin Patch\n*** Update File: value.txt\n*** Move to: unrelated/extra.txt\n@@\n-two\n+three\n*** End Patch",
    ]
    for index, text in enumerate(changes):
        model.next_items.append([patch(text, f"denied-native-{index}")])
        server.turn(
            thread, "Explain value.txt. Do not create files or change other folders."
        )
        assert project.joinpath("value.txt").read_text() == original
        assert not project.joinpath("unrelated/extra.txt").exists()
    command = f"apply_patch <<'PATCH'\n{changes[0]}\nPATCH"
    model.next_items.append(
        [tool("exec_command", {"cmd": command, "login": False}, "denied-shell-patch")]
    )
    server.turn(
        thread, "Explain value.txt. Do not create files or change other folders."
    )
    assert not project.joinpath("unrelated/extra.txt").exists()
    requests = [
        message
        for message in server.messages
        if message.get("method") == "item/tool/requestUserInput"
    ]
    assert len(requests) >= 4, requests
    return [
        "Scope Guard denies native, multi-file, rename, and shell patches before changes"
    ]
