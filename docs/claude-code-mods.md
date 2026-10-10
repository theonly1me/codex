# Claude Code mods

This fork runs executable Claude Code plugin modules in the Codex CLI and app-server.
The compatibility target is the 11 personal mods at commit `6d69bf8871bb088c3bdf5b5526b03dce633cdb88`.
See [the integration boundaries](claude-code-mods-architecture.md) for loader isolation and upstream maintenance.

## Install

1. Build this fork with the repository's Rust toolchain and V8 setup.
2. Add the marketplace:

```sh
codex plugin marketplace add theonly1me/claude-code-mods --ref 6d69bf8871bb088c3bdf5b5526b03dce633cdb88
```

3. Install the plugins you want:

```sh
codex plugin add scope-guard@claude-code-mods
codex plugin add looks@claude-code-mods
codex plugin add samurai-dojo@claude-code-mods
```

4. Start Codex in your project with `codex --enable plugins`.
5. Enter a registered mod command, such as `/scope`, `/looks`, or `/dojo`.

Use `codex plugin list --json` to inspect installed plugins.
Use the existing plugin settings to enable or disable a plugin.
Codex refreshes mod settings before each submitted turn.
A source change restarts executable state at that boundary.
Persistent state remains in the Codex mod store.
Reactive state survives a module reload and resets when the session ends.
These lifetimes follow the [Claude Code state reference](https://code.claude.com/docs/en/plugins/mods/interface#keep-state).

Built-in slash commands retain their existing behavior.
When a mod command has the same name, use its qualified name, such as `/looks:theme`.
`/looks` opens the mod theme picker.
Its base theme button opens the native Codex theme picker.

## Options and models

Set options under the plugin's existing configuration entry:

```toml
[plugins."scope-guard@claude-code-mods"]
enabled = true

[plugins."scope-guard@claude-code-mods".options]
confirmModel = "gpt-6.1-sol"
```

Use the option names declared by the installed plugin's `userConfig`.
Invalid option names or types stop mod initialization with an error.
Claude model defaults, including `haiku`, use the active Codex model.
A startup message lists these remapped options.
Explicit model overrides remain explicit and use the active Codex provider and credentials.
The selected provider must serve that model.

Helper calls use low reasoning effort by default.
An explicit helper effort overrides this default.
Helpers use an output limit of 4096 tokens and a timeout of 30 seconds by default.
The output limit includes reasoning tokens when the provider counts them.
Helpers have no tools.
A fork includes the current conversation history.
Helper calls can consume paid model usage.

## Data and permissions

Mod state lives under `$CODEX_HOME/claude-mods`.
`$.env.get('HOME')` returns `$CODEX_HOME/claude-mods/home` for compatibility.
The games and Change Journal write their `.claude` data below this private home.
The user's actual `~/.claude` data remains separate.

Project file access and subprocesses use Codex's existing sandbox and approval interfaces.
Mods can read assets inside their own installed plugin root.
The compatibility home and persistent store permit mod data writes.
Install only plugins whose code you trust.
The JavaScript execution limits do not make a plugin a security sandbox.

A failed guard blocks the native operation before execution.
A failure after native execution reports the completed native result and a separate mod diagnostic.
This prevents a successful write from appearing as a failed operation that the model could repeat.
A removed or unreadable module declaration requires explicit plugin disablement when that mod was already active.

## Terminal controls

Press F6 to select a mod pane or an interactive prompt band.
Press Tab to select a button or input in that pane.
Press Enter to activate the selected control or submit input.
Button hotkeys also work inside the selected pane.
Press Escape to close a pane when it allows closing, or return to the composer.
Native questions retain keyboard priority.

The terminal renderer supports Box, Text, Button, Input, Markdown, Code, and Raster.
Panes, prompt bands, transcript decorations, spinner words, prompt hints, and turn durations use mod render events.
Raster frames update without replacing the whole pane.
Terminal resize requests a new render tree.

## App-server clients

Clients use these methods after `thread/start`:

| Method              | Purpose                                                                                 |
| ------------------- | --------------------------------------------------------------------------------------- |
| `claudeMod/list`    | List registered commands and panes. Set `interactive` when the client renders controls. |
| `claudeMod/command` | Run a registered command with its plugin ID and arguments.                              |
| `claudeMod/render`  | Request a typed render tree for a component and viewport.                               |
| `claudeMod/control` | Invoke a current callback, close a pane, or report focus.                               |
| `claudeMod/changed` | Receive command, pane, raster, status, clipboard, and diagnostic notifications.         |

Clients must use callback IDs from their latest render tree.
Old callback IDs fail after a replacement render or pane close.
Questions and native approvals use the existing app-server methods.
A client must render those requests to support interactive guards.

## Compatibility boundaries

Supported event names are `session.start`, `session.end`, `session.measure`, `classic.SessionStart`, `turn.start`, `turn.complete`, and `turn.step`.
Prompt events are `prompt.submit` and `prompt.compose`.
Tool middleware uses `tool.call` around native execution.
Command and configuration events are `command.run` and `config.set`.
UI events are `ui.render`, `ui.close`, and `ui.focus`.

Local JavaScript, TypeScript, and TSX imports resolve inside the plugin root.
The runtime supports the `claude-code` and `claude-code/jsx-runtime` compatibility modules.
Bare package imports, Node APIs, and asynchronous module initialization are unsupported.
Unsupported events and host API calls report errors.
The subprocess adapter uses POSIX shell syntax.
Windows subprocess support requires a platform adapter.

Native patches expose complete file changes to Edit and Write guards.
A rename also guards its destination.
Patch content rewrites are unsupported and fail before execution.
Shell tools expose Bash command and result fields.
Long subprocesses retain their original command during subsequent `write_stdin` calls.

This implementation does not provide full Claude Code API parity.
Streaming middleware supports passthrough and text deltas.
Native structural stream events must remain intact.
Codex selects the native request model and reasoning effort.
Delta rewrites affect live output. Stored final messages retain their native content.
The source tree does not include the desktop renderer.
Other clients can implement the typed app-server UI transport.

## Verify

Run the isolated native suite from the repository root:

```sh
python3 scripts/claude_mods_smoke.py --mods-ref 6d69bf8871bb088c3bdf5b5526b03dce633cdb88
```

Use `--mods-checkout PATH` for an existing copy and `--binary PATH` for another built CLI.
Use `--output PATH` to retain the result and app-server log.
The suite uses an isolated Codex home, an isolated project under `.tmp`, and a local model provider.
It performs real plugin installation, native file patches, shell calls, and app-server requests.
It does not install plugins into the user's configuration or call a paid provider.
