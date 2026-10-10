mod support;

use codex_mod_runtime::Dispatch;
use codex_mod_runtime::ModRuntime;
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

#[tokio::test]
async fn middleware_preserves_state_rewrites_inputs_and_observes_native_results()
-> Result<(), Box<dyn std::error::Error>> {
    let (_directory, plugin) = support::plugin(
        r#"
let calls: number = 0;
export function register(on) {
  on('tool.call', {tool:'Bash'}, async ($, event, next) => {
    calls += 1;
    const result = await next({...event, command:event.command + ' checked'});
    return {...result, calls, observed:result.result.stdout};
  });
  on('ui.render', {component:'Pane'}, async ($, event) => {
    const {Text} = $.ui.resolve(event);
    return <Text bold>{calls}</Text>;
  });
}
"#,
    )?;
    let runtime = ModRuntime::start(vec![plugin], Arc::new(support::RejectHost)).await?;
    let native_calls = Arc::new(AtomicUsize::new(0));
    for count in 1..=2 {
        let native_calls = Arc::clone(&native_calls);
        let result = runtime
            .dispatch(
                Dispatch {
                    name: "tool.call".into(),
                    input: json!({"tool":"Bash","command":"echo"}),
                },
                move |input| {
                    native_calls.fetch_add(1, Ordering::Relaxed);
                    async move {
                        assert_eq!(input["command"], "echo checked");
                        Ok(json!({"result":{"stdout":"real output"},"isError":false}))
                    }
                },
            )
            .await?;
        assert_eq!(result["calls"], count);
        assert_eq!(result["observed"], "real output");
    }
    assert_eq!(native_calls.load(Ordering::Relaxed), 2);
    let rendered = runtime
        .observe(Dispatch {
            name: "ui.render".into(),
            input: json!({"component":"Pane","requestId":"fixture"}),
        })
        .await?;
    assert_eq!(rendered["element"], "Text");
    assert_eq!(rendered["props"]["bold"], true);
    assert_eq!(rendered["children"], json!([2]));
    Ok(())
}

#[tokio::test]
async fn denying_a_tool_prevents_its_native_side_effect() -> Result<(), Box<dyn std::error::Error>>
{
    let (_directory, plugin) = support::plugin(
        "export function register(on) { on('tool.call', async () => ({deny:'outside scope'})); }",
    )?;
    let runtime = ModRuntime::start(vec![plugin], Arc::new(support::RejectHost)).await?;
    let native_calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&native_calls);
    let result = runtime
        .dispatch(
            Dispatch {
                name: "tool.call".into(),
                input: json!({"tool":"Write"}),
            },
            move |_| {
                observed.fetch_add(1, Ordering::Relaxed);
                async { Ok(json!({"isError":false})) }
            },
        )
        .await?;
    assert_eq!(result["deny"], "outside scope");
    assert_eq!(native_calls.load(Ordering::Relaxed), 0);
    Ok(())
}

#[tokio::test]
async fn runaway_guard_fails_before_native_execution() -> Result<(), Box<dyn std::error::Error>> {
    let (_directory, plugin) = support::plugin(
        "export function register(on) { on('tool.call', () => { while (true) {} }); }",
    )?;
    let runtime = ModRuntime::start(vec![plugin], Arc::new(support::RejectHost)).await?;
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        runtime.dispatch(
            Dispatch {
                name: "tool.call".into(),
                input: json!({"tool":"Bash"}),
            },
            |_| async { panic!("runaway guard must prevent native execution") },
        ),
    )
    .await?;
    let error = result.err().ok_or("runaway guard succeeded")?;
    assert!(error.contains("execution deadline"), "{error}");
    Ok(())
}
