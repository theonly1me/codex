use crate::ModResult;
use serde_json::Value;

pub(crate) fn decode(
    scope: &mut v8::PinScope<'_, '_>,
    value: v8::Local<'_, v8::Value>,
) -> ModResult<Value> {
    if value.is_undefined() {
        return Ok(Value::Null);
    }
    let json = v8::json::stringify(scope, value)
        .ok_or_else(|| "mod value cannot be serialized".to_string())?;
    let json = json.to_rust_string_lossy(scope);
    if json.len() > 4 * 1024 * 1024 {
        return Err("mod value exceeds 4 MiB".into());
    }
    serde_json::from_str(&json).map_err(|error| error.to_string())
}

pub(crate) fn encode<'scope>(
    scope: &mut v8::PinScope<'scope, '_>,
    value: &Value,
) -> ModResult<v8::Local<'scope, v8::Value>> {
    let json = serde_json::to_string(value).map_err(|error| error.to_string())?;
    let text =
        v8::String::new(scope, &json).ok_or_else(|| "cannot allocate mod value".to_string())?;
    v8::json::parse(scope, text).ok_or_else(|| "cannot parse mod value".to_string())
}

pub(crate) fn error_text(
    scope: &mut v8::PinScope<'_, '_>,
    value: v8::Local<'_, v8::Value>,
) -> String {
    value.to_rust_string_lossy(scope)
}
