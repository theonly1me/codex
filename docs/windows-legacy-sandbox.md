# Windows sandbox migration

The unelevated Windows sandbox is disabled in this fork.
Its token keeps the signed-in user's parent-directory deletion permissions.
Those permissions can permit deletion outside the writable roots and deletion of protected `.git` directories.
The same failure is reported in [upstream issue #32915](https://github.com/openai/codex/issues/32915).

The fork rejects an unelevated launch before it creates a process or applies sandbox ACLs.
It does not select another backend silently.
The current account-isolated and MXC backends remain available.

1. Set the Windows sandbox mode in `config.toml`:

   ```toml
   [windows]
   sandbox = "elevated"
   ```

2. Start Codex and run `/sandbox` to configure the Windows sandbox.
3. Complete the setup prompt if Windows requests administrator approval.

The account-isolated backend executes commands under a managed account.
Its access to user credentials and profile directories differs from the former unelevated backend.
Use the configured writable roots for command output.
