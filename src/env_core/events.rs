use chrono::Utc;

use super::types::{EnvEvent, EnvEventType, EnvScope};

pub fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

pub fn build_event(
    event_type: EnvEventType,
    scope: EnvScope,
    name: Option<String>,
    message: Option<String>,
) -> EnvEvent {
    EnvEvent {
        event_type,
        scope,
        at: now_iso(),
        name,
        message,
    }
}
