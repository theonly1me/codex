use crate::ModResult;
use serde_json::Value;

pub(super) struct DimensionRequest<'value> {
    pub value: &'value Value,
    pub default: u16,
    pub maximum: u16,
}

pub(super) fn dimension(request: DimensionRequest<'_>) -> ModResult<u16> {
    if request.value.is_null() {
        return Ok(request.default);
    }
    let value = request
        .value
        .as_u64()
        .filter(|value| *value > 0 && *value <= u64::from(request.maximum))
        .ok_or_else(|| format!("mod dimension must be between 1 and {}", request.maximum))?;
    u16::try_from(value).map_err(|error| error.to_string())
}
