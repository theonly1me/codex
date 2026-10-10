use crate::HostCall;
use crate::ModPlugin;
use crate::ModResult;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Default)]
pub(crate) struct ReactiveState {
    state: Mutex<State>,
}

#[derive(Default)]
struct State {
    values: HashMap<(String, String), (Value, usize)>,
    bytes: usize,
}

impl ReactiveState {
    pub(crate) fn clear(&self) {
        *self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = State::default();
    }

    pub(crate) fn call(
        &self,
        call: &HostCall,
        plugins: &HashMap<String, ModPlugin>,
    ) -> ModResult<(Value, bool)> {
        let point = call.arguments.first().ok_or("state requires a point")?;
        let namespace = point["plugin"]
            .as_str()
            .ok_or("state point requires a plugin")?;
        let owner = plugins.get(&call.plugin).ok_or("unknown state caller")?;
        let target = if owner.name == namespace {
            owner
        } else {
            let mut candidates = plugins.values().filter(|plugin| plugin.name == namespace);
            let target = candidates
                .next()
                .ok_or_else(|| format!("unknown state namespace {namespace}"))?;
            if candidates.next().is_some() {
                return Err(format!("ambiguous state namespace {namespace}"));
            }
            target
        };
        let key = point["key"].as_str().ok_or("state point requires a key")?;
        if key.len() > 256 {
            return Err("state key exceeds 256 bytes".into());
        }
        let identity = (target.id.clone(), key.to_string());
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match call.method.as_str() {
            "state.get" => Ok((
                state
                    .values
                    .get(&identity)
                    .map(|(value, _)| value.clone())
                    .unwrap_or(Value::Null),
                false,
            )),
            "state.set" => {
                let value = call.arguments.get(1).cloned().unwrap_or(Value::Null);
                if state
                    .values
                    .get(&identity)
                    .is_some_and(|(existing, _)| *existing == value)
                {
                    return Ok((Value::Null, false));
                }
                if !state.values.contains_key(&identity) && state.values.len() >= 4096 {
                    return Err("session state exceeds 4096 keys".into());
                }
                let size = serde_json::to_vec(&value)
                    .map_err(|error| error.to_string())?
                    .len();
                let previous = state
                    .values
                    .get(&identity)
                    .map(|(_, size)| *size)
                    .unwrap_or(0);
                let total = state.bytes.saturating_sub(previous).saturating_add(size);
                if size > 1024 * 1024 || total > 16 * 1024 * 1024 {
                    return Err(
                        "session state exceeds its 1 MiB value or 16 MiB total limit".into(),
                    );
                }
                state.values.insert(identity, (value, size));
                state.bytes = total;
                Ok((Value::Null, true))
            }
            _ => Err(format!("unsupported state method {}", call.method)),
        }
    }
}
