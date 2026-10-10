use codex_claude_mods_extension::*;
use futures::future::BoxFuture;
use serde_json::Map;
use serde_json::Value;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

struct Host;

impl ModHost for Host {
    fn call(&self, call: HostCall) -> BoxFuture<'_, ModResult<Value>> {
        Box::pin(async move {
            match call.method.as_str() {
                "fs.read" => {
                    std::fs::read_to_string(call.arguments[0].as_str().ok_or("invalid path")?)
                        .map(Value::String)
                        .map_err(|error| error.to_string())
                }
                "process.run" => {
                    let arguments = call.arguments[0]
                        .as_array()
                        .ok_or("invalid arguments")?
                        .iter()
                        .map(|value| value.as_str().ok_or("invalid command"))
                        .collect::<Result<Vec<_>, _>>()?;
                    if arguments.first() != Some(&"git") {
                        return Err("inspection permits only Git subprocesses".into());
                    }
                    let mut command = tokio::process::Command::new(arguments[0]);
                    command.args(&arguments[1..]);
                    if let Some(cwd) = call
                        .arguments
                        .get(1)
                        .and_then(|value| value["cwd"].as_str())
                    {
                        command.current_dir(cwd);
                    }
                    let output = command.output().await.map_err(|error| error.to_string())?;
                    Ok(
                        json!({"stdout":String::from_utf8_lossy(&output.stdout),"stderr":String::from_utf8_lossy(&output.stderr),"exitCode":output.status.code()}),
                    )
                }
                "session.id" => Ok(json!("inspection")),
                "session.usage" => Ok(
                    json!({"context":{"tokens":0,"window":128000,"percent":0},"rateLimits":[],"cost":null}),
                ),
                "model.complete" | "model.fork" => {
                    Err("inspection does not call paid models".into())
                }
                _ => Err(format!("inspection cannot call {}", call.method)),
            }
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), String> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("pass the personal mods checkout")?,
    );
    let temporary = tempfile::tempdir().map_err(|error| error.to_string())?;
    let mut plugins = Vec::new();
    for entry in std::fs::read_dir(root.join("plugins")).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            continue;
        }
        let id = format!("{}@inspection", entry.file_name().to_string_lossy());
        if let Some(plugin) = ModPlugin::discover(codex_mod_runtime::ModPluginRequest {
            id: &id,
            root: &entry.path(),
            options: &Map::new(),
        })? {
            plugins.push(plugin);
        }
    }
    plugins.sort_by(|left, right| left.name.cmp(&right.name));
    let count = plugins.len();
    let session = ModSession::default();
    session
        .start(ModSessionRequest {
            plugins,
            codex_home: temporary.path().into(),
            cwd: std::env::current_dir()
                .map_err(|error| error.to_string())?
                .display()
                .to_string(),
            session_id: "inspection".into(),
            interactive: true,
            delegate: Arc::new(Host),
            sink: Arc::new(|event| {
                Box::pin(async move {
                    println!("{}", serde_json::to_string(&event).unwrap_or_default());
                })
            }),
        })
        .await?;
    println!(
        "{}",
        json!({"loaded":count,"commands":session.commands().await})
    );
    for command in session.commands().await {
        let result = session
            .run_command(&command.plugin_id, &command.name, "")
            .await;
        println!(
            "{}",
            json!({"command":command.qualified_name,"result":result})
        );
    }
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    for pane in session.panes().await {
        let tree = session
            .render(codex_protocol::claude_mods::ModRenderRequest {
                component: "Pane".into(),
                request_id: pane.id.clone(),
                columns: 80,
                rows: 30,
                is_focused: true,
                is_working: false,
                props: json!({"id":pane.id,"placement":"dock"})
                    .as_object()
                    .cloned()
                    .ok_or("invalid props")?,
            })
            .await;
        println!("{}", json!({"pane":pane.id,"render":tree}));
    }
    session.observe("session.end", json!({})).await?;
    Ok(())
}
