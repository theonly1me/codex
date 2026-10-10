use crate::ModResult;
use serde_json::Map;
use serde_json::Value;

pub fn resolve_options(
    definitions: &Map<String, Value>,
    overrides: &Map<String, Value>,
) -> ModResult<Value> {
    let mut options = Map::new();
    for (name, definition) in definitions {
        let kind = definition
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("mod option `{name}` has no type"))?;
        let value = overrides.get(name).or_else(|| definition.get("default"));
        let Some(value) = value else { continue };
        let valid = match kind {
            "boolean" => value.is_boolean(),
            "string" => value.is_string(),
            "number" => value.is_number(),
            _ => return Err(format!("mod option `{name}` has unsupported type `{kind}`")),
        };
        if !valid {
            return Err(format!("mod option `{name}` must be a {kind}"));
        }
        if let Some(allowed) = definition.get("options").and_then(Value::as_array)
            && !allowed.contains(value)
        {
            return Err(format!(
                "mod option `{name}` is not one of its declared options"
            ));
        }
        options.insert(name.clone(), value.clone());
    }
    for name in overrides.keys() {
        if !definitions.contains_key(name) {
            return Err(format!("unknown mod option `{name}`"));
        }
    }
    Ok(Value::Object(options))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn options_preserve_defaults_and_explicit_overrides() -> ModResult<()> {
        let definitions = Map::from_iter([
            (
                "mode".into(),
                json!({"type":"string", "default":"ask", "options":["ask","off"]}),
            ),
            ("count".into(), json!({"type":"number", "default":8})),
        ]);
        let overrides = Map::from_iter([("mode".into(), json!("off"))]);
        assert_eq!(
            resolve_options(&definitions, &overrides)?,
            json!({"mode":"off","count":8})
        );
        assert!(
            resolve_options(
                &definitions,
                &Map::from_iter([("count".into(), json!("8"))])
            )
            .is_err()
        );
        Ok(())
    }
}
