use super::ThreadParamsMode;
use super::config_request_overrides_from_config;
use crate::legacy_core::config::ConfigBuilder;
use serde_json::json;

#[tokio::test]
async fn mod_enablement_and_model_options_reach_the_app_server()
-> Result<(), Box<dyn std::error::Error>> {
    let home = tempfile::tempdir()?;
    let plugins: toml::Value = toml::from_str(
        r#"
["grill@mods"]
enabled = false
["grill@mods".options]
model = "gpt-6.1-sol"
"#,
    )?;
    let config = ConfigBuilder::default()
        .codex_home(home.path().to_path_buf())
        .cli_overrides(vec![("plugins".into(), plugins)])
        .loader_overrides(codex_config::LoaderOverrides::without_managed_config_for_tests())
        .build()
        .await?;
    for mode in [ThreadParamsMode::Embedded, ThreadParamsMode::Remote] {
        let overrides = config_request_overrides_from_config(&config, mode)
            .ok_or("missing mod launch overrides")?;
        assert_eq!(
            overrides.get("plugins"),
            Some(&json!({"grill@mods":{"enabled":false,"options":{"model":"gpt-6.1-sol"}}}))
        );
    }
    Ok(())
}
