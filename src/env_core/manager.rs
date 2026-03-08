use super::*;

#[derive(Clone)]
pub struct EnvManager {
    pub(super) cfg: EnvCoreConfig,
    pub(super) event_cb: Option<EventCallback>,
}

impl Default for EnvManager {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvManager {
    pub fn new() -> Self {
        Self {
            cfg: load_env_config(),
            event_cb: None,
        }
    }

    #[cfg(feature = "dashboard")]
    pub fn with_event_callback(mut self, event_cb: EventCallback) -> Self {
        self.event_cb = Some(event_cb);
        self
    }

    #[cfg(feature = "dashboard")]
    pub fn config(&self) -> &EnvCoreConfig {
        &self.cfg
    }
}
