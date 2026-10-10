use crate::ModPlugin;
use crate::ModResult;
use crate::ModSession;

impl ModSession {
    pub fn discovery_failed(&self, message: String) {
        self.state().failure = Some(message);
    }
    pub fn validate_reload(&self, enabled: &[String], discovered: &[ModPlugin]) -> ModResult<()> {
        if let Some(host) = self.host() {
            for id in host.plugins.keys() {
                if enabled.contains(id) && !discovered.iter().any(|plugin| &plugin.id == id) {
                    let message = format!(
                        "{id}: executable modules disappeared; disable the plugin explicitly to remove its guards"
                    );
                    self.discovery_failed(message.clone());
                    return Err(message);
                }
            }
        }
        Ok(())
    }
}
