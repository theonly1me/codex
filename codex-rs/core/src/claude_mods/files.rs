use super::host::step;
use super::host::string;
use crate::session::session::Session;
use codex_claude_mods_extension::HostCall;
use codex_claude_mods_extension::ModResult;
use codex_file_system::EnvironmentAccess;
use codex_file_system::EnvironmentAccessExt;
use codex_file_system::GetMetadataOptions;
use codex_file_system::ReadFileOptions;
use codex_file_system::WriteFileOptions;
use serde_json::Value;
use serde_json::json;
use std::sync::Arc;

pub(crate) async fn call(session: &Arc<Session>, call: &HostCall) -> ModResult<Value> {
    let captured = step(session).await?;
    let environments = captured.environments();
    let (environment, access) = environments
        .into_iter()
        .next()
        .ok_or_else(|| "mod has no filesystem environment".to_string())?;
    let path = string(call.arguments.first().unwrap_or(&Value::Null))?;
    let path = environment
        .cwd()
        .join(path)
        .map_err(|error| error.to_string())?;
    match call.method.as_str() {
        "fs.read" => access
            .read_file_text(&path, ReadFileOptions::default())
            .await
            .map(Value::String)
            .map_err(|error| error.to_string()),
        "fs.write" => {
            let contents = string(call.arguments.get(1).unwrap_or(&Value::Null))?;
            access
                .write_file(
                    &path,
                    contents.as_bytes().to_vec(),
                    WriteFileOptions {
                        follow_symlinks: false,
                    },
                )
                .await
                .map_err(|error| error.to_string())?;
            Ok(Value::Null)
        }
        "fs.exists" => match access
            .get_metadata(&path, GetMetadataOptions::default())
            .await
        {
            Ok(_) => Ok(json!(true)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(json!(false)),
            Err(error) => Err(error.to_string()),
        },
        "fs.stat" => {
            let metadata = access
                .get_metadata(&path, GetMetadataOptions::default())
                .await
                .map_err(|error| error.to_string())?;
            Ok(
                json!({"isDirectory":metadata.is_directory,"isFile":metadata.is_file,"isSymlink":metadata.is_symlink,"size":metadata.size,"createdAt":metadata.created_at_ms,"modifiedAt":metadata.modified_at_ms}),
            )
        }
        "fs.list" => {
            let entries = access
                .read_directory(&path)
                .await
                .map_err(|error| error.to_string())?;
            Ok(json!(entries.into_iter().map(|entry| json!({"name":entry.file_name,"isDirectory":entry.is_directory,"isFile":entry.is_file})).collect::<Vec<_>>()))
        }
        "fs.ancestors" => Ok(json!(
            path.ancestors()
                .map(|path| path.to_string())
                .collect::<Vec<_>>()
        )),
        _ => Err(format!("unsupported filesystem method {}", call.method)),
    }
}
