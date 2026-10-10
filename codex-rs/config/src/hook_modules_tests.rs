use crate::HooksFile;
use serde_json::json;

#[test]
fn claude_modules_load_beside_existing_command_hooks() -> Result<(), serde_json::Error> {
    let parsed = serde_json::from_value::<HooksFile>(json!({
        "modules": ["./register.tsx"],
        "hooks": {
            "Stop": [{"hooks": [{"type": "command", "command": "echo checked"}]}]
        }
    }));
    assert!(
        parsed.is_ok(),
        "a Claude mod must preserve its settings hooks: {parsed:?}"
    );
    let parsed = parsed?;
    assert_eq!(parsed.modules, vec!["./register.tsx"]);
    assert_eq!(parsed.hooks.stop.len(), 1);
    Ok(())
}

#[test]
fn claude_modules_require_an_array_of_paths() {
    for modules in [json!("./register.ts"), json!([false]), json!(null)] {
        assert!(serde_json::from_value::<HooksFile>(json!({"modules": modules})).is_err());
    }
}
