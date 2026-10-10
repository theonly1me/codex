use anyhow::Result;

pub(crate) fn require_supported_backend() -> Result<()> {
    anyhow::bail!(
        "The unelevated Windows sandbox cannot enforce deletion isolation. Set windows.sandbox = \"elevated\" in config.toml. Run /setup-default-sandbox to configure the secure backend."
    );
}

#[cfg(test)]
mod tests;
