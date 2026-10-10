use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use codex_claude_mods_extension::ModResult;
use serde_json::Value;
use serde_json::json;

pub(crate) fn update(
    invocation: &mut ToolInvocation,
    original: &Value,
    changed: &Value,
) -> ModResult<()> {
    if original["tool"] == "Bash" && original["command"] != changed["command"] {
        let command = changed["command"]
            .as_str()
            .ok_or_else(|| "mod command rewrite must be text".to_string())?;
        let environments = invocation.step_context.environments();
        let cwd = environments
            .first()
            .ok_or_else(|| "mod shell has no environment".to_string())?
            .0
            .cwd();
        super::shell_patch::validate_rewrite(
            original["command"].as_str().unwrap_or_default(),
            command,
            cwd,
        )?;
        let ToolPayload::Function { arguments } = &mut invocation.payload else {
            return Err("this shell tool cannot rewrite its input".into());
        };
        let mut input: Value =
            serde_json::from_str(arguments).map_err(|error| error.to_string())?;
        match invocation.tool_name.name.as_str() {
            "exec_command" => input["cmd"] = json!(command),
            "shell_command" => input["command"] = json!(command),
            _ => return Err("this shell tool cannot rewrite its input".into()),
        }
        *arguments = input.to_string();
    }
    for field in ["file_path", "old_string", "new_string", "content"] {
        if original[field] != changed[field] {
            return Err(format!(
                "mod rewrites of patch field {field} are unsupported"
            ));
        }
    }
    Ok(())
}
