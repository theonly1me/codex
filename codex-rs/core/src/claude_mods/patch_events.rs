use crate::tools::context::ToolInvocation;
use codex_apply_patch::Hunk;
use codex_claude_mods_extension::ModResult;
use codex_file_system::EnvironmentAccessExt;
use codex_file_system::ReadFileOptions;
use serde_json::Value;
use serde_json::json;

pub(crate) async fn events(invocation: &ToolInvocation, hunk: Hunk) -> ModResult<Vec<Value>> {
    let environments = invocation.step_context.environments();
    let (environment, access) = environments
        .first()
        .ok_or_else(|| "mod tool has no environment".to_string())?;
    let uri = hunk
        .resolve_path(environment.cwd())
        .map_err(|error| error.to_string())?;
    let path = uri
        .to_abs_path()
        .map_err(|error| error.to_string())?
        .as_path()
        .display()
        .to_string();
    let mut moved = None;
    let mut event = match hunk {
        Hunk::AddFile { contents, .. } => {
            json!({"tool":"Write","file_path":path,"content":contents,"structuredPatch":[],"type":"create"})
        }
        Hunk::DeleteFile { .. } => json!({"tool":"Bash","command":format!("rm -- '{path}'")}),
        Hunk::UpdateFile {
            chunks, move_path, ..
        } => {
            let original = access
                .read_file_text(&uri, ReadFileOptions::default())
                .await
                .map_err(|error| error.to_string())?;
            let preview =
                codex_apply_patch::preview_file_update(codex_apply_patch::ProposedFileUpdate {
                    path: &uri,
                    original_contents: &original,
                    chunks: &chunks,
                    update_file_mode: if invocation
                        .turn
                        .config
                        .features
                        .enabled(codex_features::Feature::ApplyPatchPreserveLineEndings)
                    {
                        codex_apply_patch::ApplyPatchFileUpdateMode::PreserveLineEndings
                    } else {
                        codex_apply_patch::ApplyPatchFileUpdateMode::NormalizeToLf
                    },
                })
                .map_err(|error| error.to_string())?;
            if let Some(destination) = &move_path {
                let destination = environment
                    .cwd()
                    .join(
                        destination
                            .to_str()
                            .ok_or_else(|| "mod move path must be UTF-8".to_string())?,
                    )
                    .map_err(|error| error.to_string())?
                    .to_abs_path()
                    .map_err(|error| error.to_string())?
                    .as_path()
                    .display()
                    .to_string();
                moved = Some(
                    json!({"tool":"Write","file_path":destination,"content":preview,"type":"create","structuredPatch":[],"requestId":invocation.call_id,"tool_use_id":invocation.call_id,"turnId":invocation.turn.sub_id}),
                );
            }
            let before = chunks
                .iter()
                .flat_map(|chunk| &chunk.old_lines)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n");
            let after = chunks
                .iter()
                .flat_map(|chunk| &chunk.new_lines)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n");
            let patch = codex_apply_patch::structured_file_diff(&original, &preview).into_iter().map(|hunk| json!({"oldStart":hunk.old_start,"oldLines":hunk.old_lines,"newStart":hunk.new_start,"newLines":hunk.new_lines,"lines":hunk.lines})).collect::<Vec<_>>();
            json!({"tool":"Edit","file_path":path,"old_string":before,"new_string":after,"structuredPatch":patch,"move_path":move_path,"originalFile":original})
        }
    };
    event["requestId"] = json!(invocation.call_id);
    event["tool_use_id"] = json!(invocation.call_id);
    event["turnId"] = json!(invocation.turn.sub_id);
    let mut events = vec![super::lifecycle::scoped(&invocation.turn, event)];
    if let Some(moved) = moved {
        events.push(super::lifecycle::scoped(&invocation.turn, moved));
    }
    Ok(events)
}
