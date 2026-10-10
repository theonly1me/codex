mod support;

use codex_claude_mod_runtime::Dispatch;
use codex_claude_mod_runtime::ModRuntime;
use serde_json::Value;
use serde_json::json;
use std::sync::Arc;
use std::sync::Mutex;

#[tokio::test]
async fn stream_middleware_receives_chunks_and_the_native_completion()
-> Result<(), Box<dyn std::error::Error>> {
    let (_directory, plugin) = support::plugin(
        r#"
export function register(on) {
  on('turn.step', async function* ($, event, next) {
    const stream = next(event);
    for await (const chunk of stream) yield {...chunk, text: chunk.text.toUpperCase()};
    return {...await stream.result, observed:true};
  });
}
"#,
    )?;
    let runtime = ModRuntime::start(vec![plugin], Arc::new(support::RejectHost)).await?;
    let emitted = Arc::new(Mutex::new(Vec::new()));
    let output = Arc::clone(&emitted);
    let mut pulled = false;
    let result = runtime
        .dispatch(
            Dispatch {
                name: "turn.step".into(),
                input: json!({"turnId":"turn"}),
            },
            move |request| {
                let result = match request["streamOperation"].as_str() {
                    Some("start") => Value::Null,
                    Some("pull") if !pulled => {
                        pulled = true;
                        json!({"done":false,"value":{"kind":"text","text":"answer"}})
                    }
                    Some("pull") => json!({"done":true,"result":{"text":"answer","tokens":7}}),
                    Some("emit") => {
                        output.lock().unwrap().push(request["chunk"].clone());
                        Value::Null
                    }
                    _ => return std::future::ready(Err("unexpected stream operation".into())),
                };
                std::future::ready(Ok(result))
            },
        )
        .await?;
    assert_eq!(
        emitted.lock().unwrap().as_slice(),
        &[json!({"kind":"text","text":"ANSWER"})]
    );
    assert_eq!(result, json!({"text":"answer","tokens":7,"observed":true}));
    Ok(())
}

#[tokio::test]
async fn closed_controls_cannot_call_a_replaced_callback() -> Result<(), Box<dyn std::error::Error>>
{
    let (_directory, plugin) = support::plugin(
        r#"
let count = 0;
export function register(on) {
  on('ui.render', ($, event) => {
    const {Button} = $.ui.resolve(event);
    return <Button label={String(count)} onPress={() => { count += 1; }} />;
  });
}
"#,
    )?;
    let runtime = ModRuntime::start(vec![plugin], Arc::new(support::RejectHost)).await?;
    let render = || Dispatch {
        name: "ui.render".into(),
        input: json!({"component":"Pane","requestId":"controls"}),
    };
    let first = runtime.observe(render()).await?;
    let callback = first["props"]["onPress"]["callbackId"]
        .as_str()
        .ok_or("missing callback")?;
    runtime
        .observe(Dispatch {
            name: "runtime.control".into(),
            input: json!({"callbackId":callback}),
        })
        .await?;
    let second = runtime.observe(render()).await?;
    assert_eq!(second["props"]["label"], "1");
    let stale = runtime
        .observe(Dispatch {
            name: "runtime.control".into(),
            input: json!({"callbackId":callback}),
        })
        .await;
    assert!(stale.is_err());
    assert_ne!(first["props"]["onPress"], second["props"]["onPress"]);
    Ok(())
}
