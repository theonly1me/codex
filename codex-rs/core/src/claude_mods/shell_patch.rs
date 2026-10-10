use crate::tools::context::ToolInvocation;
use codex_claude_mods_extension::ModResult;
use codex_utils_path_uri::PathUri;
use serde_json::Value;

pub(crate) async fn events(invocation: &ToolInvocation, event: &Value) -> ModResult<Vec<Value>> {
    let Some(command) = event["command"].as_str() else {
        return Ok(Vec::new());
    };
    let cwd = invocation
        .step_context
        .environments()
        .first()
        .ok_or_else(|| "mod shell has no environment".to_string())?
        .0
        .cwd()
        .clone();
    let shell = event["shell"].as_str().unwrap_or("sh");
    let arguments = vec![shell.to_string(), "-c".into(), command.into()];
    let codex_apply_patch::MaybeApplyPatch::Body(patch) =
        codex_apply_patch::maybe_parse_apply_patch(&arguments, &cwd)
    else {
        return Ok(Vec::new());
    };
    if patch.workdir.is_some() || event.get("workdir").is_some() {
        return Err("guarded shell patches require the session working directory; use the native apply_patch tool".into());
    }
    let mut events = Vec::new();
    for hunk in patch.hunks {
        events.extend(super::patch_events::events(invocation, hunk).await?);
    }
    Ok(events)
}

pub(super) fn validate_rewrite(original: &str, changed: &str, cwd: &PathUri) -> ModResult<()> {
    for command in [original, changed] {
        let arguments = vec!["sh".into(), "-c".into(), command.into()];
        if !matches!(
            codex_apply_patch::maybe_parse_apply_patch(&arguments, cwd),
            codex_apply_patch::MaybeApplyPatch::NotApplyPatch
        ) {
            return Err(
                "mod rewrites of shell patches are unsupported; use the native apply_patch tool"
                    .into(),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_command_rewrite_cannot_change_a_guarded_patch_or_introduce_one()
    -> Result<(), Box<dyn std::error::Error>> {
        let cwd = PathUri::parse("file:///workspace")?;
        let patch = "apply_patch <<'PATCH'\n*** Begin Patch\n*** Add File: outside.txt\n+outside\n*** End Patch\nPATCH";
        assert!(validate_rewrite("printf before", patch, &cwd).is_err());
        assert!(validate_rewrite(patch, "printf after", &cwd).is_err());
        assert!(validate_rewrite("printf before", "printf after", &cwd).is_ok());
        Ok(())
    }
}
