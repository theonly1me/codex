use super::ExecCell;
use serde_json::Value;
use serde_json::json;

impl ExecCell {
    pub(crate) fn mod_props(&self) -> Value {
        let calls = self.iter_calls().map(|call| {
            let output = call.output.as_ref().map(|output| json!({"stdout":output.transcript_lines().map(std::borrow::Cow::into_owned).collect::<Vec<_>>().join("\n"),"stderr":"","exitCode":output.exit_code}));
            json!({"tool":"Bash","input":{"command":call.command.join(" ")},"output":output,"isRunning":call.output.is_none(),"isErrored":call.output.as_ref().is_some_and(|output| output.exit_code != 0),"isInterrupted":false})
        }).collect::<Vec<_>>();
        if calls.len() == 1 {
            calls.into_iter().next().unwrap_or(Value::Null)
        } else {
            json!({"calls":calls,"isExpanded":false,"isActive":false})
        }
    }
}
