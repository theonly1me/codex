# Claude Code mod integration

The loader belongs to `ext/claude-mods`.
The runtime belongs to `mod-runtime` and uses the crate name `codex-mod-runtime`.
Native plugin discovery decides which installed plugins can run.
It passes a source snapshot to the mod loader.
It does not import the mod runtime or parse executable modules.

```text
Native plugin manager -> ModSource[] -> ModLoader -> ModSession -> JavaScript runtime
Core lifecycle -> SessionHooks -> ModHooks -> ModSession
ModSession -> typed events -> app-server transport -> client renderer
```

## Loader boundary

`codex-claude-mods-extension` exports `ModLoader`, `InstalledModLoader`, and `ModSource`.
`ModLoader::load` returns enabled plugin identities and discovered executable modules.
A different catalog can implement this trait without importing the native plugin manager.

`ModSession::load` owns discovery errors, reload checks, activation, and runtime replacement.
It requests host capabilities only when executable state must run or stop.
An empty catalog does not start V8 or request filesystem access.
An enabled guard cannot disappear during reload without explicit plugin disablement.

The native source adapter lives in `core/src/claude_mods/loader.rs`.
It maps the native plugin manager's approved result into `ModSource` values.
The extension loader reads module declarations from the original plugin files.
This preserves inline declarations without adding mod parsing to `core-plugins`.

## Session hook boundary

`core/src/session_hooks` owns the hook interface and ordered middleware chain.
Its default registration contains `ModHooks`.
Core lifecycle paths call this interface.
They do not access `ModSession` or import Claude compatibility code.

`SessionHook` covers input, instructions, turn events, streams, tools, compaction, and client event filtering.
Defaults preserve native input, instructions, streams, and tool execution.
A host can seed a replacement `SessionHooks` value in upstream `ExtensionData`.
An empty hook list disables the adapter without changing the lifecycle call sites.

The hook registry registers shutdown through upstream `ThreadLifecycleContributor`.
It registers compaction through upstream `TurnLifecycleContributor`.
Sessions with no active mods retain the native registry.
Isolated sessions do not install mod hooks.
Inherited registries reuse the same contributor without duplicate registration.
The three native compaction implementations contain no mod calls.
The native session service structure has no mod field.
The adapter stores its runtime state in upstream `ExtensionData`.

Native filesystem, model, approval, and subprocess bridges stay in `core/src/claude_mods`.
Helper output limits belong to the helper model client session.
The shared `Prompt` type and its constructors match upstream.
They retain native permissions and admission rules.
Future native API changes can require edits inside this adapter.
The loader and JavaScript runtime remain independent of those private APIs.

## Remaining upstream integration

Some integration changes remain necessary:

1. Core calls the session hook interface at native lifecycle boundaries.
2. Plugin configuration accepts per-plugin options and module declarations.
3. Native patch preview and model requests expose the capabilities that mod helpers require.
4. Protocol enums carry typed mod events and app-server requests.
5. The terminal composes mod views above the existing composer and routes controls.
6. Generated schemas and SDK types reflect those protocol additions.

These changes can still conflict with upstream edits.
Keep each adapter call small and keep compatibility decisions inside the owned modules.
Regenerate schemas after an upstream merge.
Do not resolve generated schema conflicts by selecting an entire side.

## Verify the boundary

1. Run `cargo test -p codex-claude-mods-extension --test loader`.
2. Run `cargo test -p codex-core --lib session_hooks::tests` with `RUST_MIN_STACK=8388608`.
3. Run the native isolation and internal-session tests to check registry preservation.
4. Run `python3 scripts/claude_mods_smoke.py` with the current CLI binary.
5. Check both terminal modes for changing Dojo scene cells and resize behavior.

The loader tests exercise a replacement catalog, inactive native plugins, and failed guard reloads.
The hook tests exercise replacement registration and native behavior with an empty registry.
The native suite uses all 11 unchanged personal mods in temporary homes.

## Fork CI runners

The repository variable `CODEX_CI_RUNNERS` supplies runner labels by platform or target.
The checked-in policy lives in `.github/fork/ci-runners.json`.
Unset entries retain the upstream runner selection.
The build timeout entry permits cold builds on standard hosted machines.
The Linux space entry removes unused preinstalled Android, .NET, and Haskell SDKs on disposable hosted runners.
The cleanup rejects local machines and self-hosted runners.
Unset timeouts retain the upstream limits.
Each workflow keeps its existing checks, targets, and failure rules.

Apply the public fork's policy with:

```sh
gh variable set CODEX_CI_RUNNERS --repo theonly1me/codex < .github/fork/ci-runners.json
```

The policy uses standard Linux, Windows, and macOS runners.
The x86 macOS target uses an Intel host.
The ARM macOS target uses an ARM host.
Windows without remote execution uses a GNU LLVM target and native MSVC host tools.
The native C toolchain declares an MSVC ABI constraint so GNU targets retain their LLVM toolchain.

Windows voice builds require the operator's MSVC license acknowledgement.
After accepting Microsoft's terms, set the repository variable `CODEX_CI_MSVC_RUNTIME_LICENSE_ACCEPTED` to `1`.
The wrapper passes the MSVC repository flag only on Windows with that explicit value.
An unset variable retains the native license check.
