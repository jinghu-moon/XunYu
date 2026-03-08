use crate::ctx_store::{load_session, session_path_from_env};

pub(super) fn active_profile_name() -> Option<String> {
    let path = session_path_from_env()?;
    let session = load_session(&path)?;
    if session.active.trim().is_empty() {
        None
    } else {
        Some(session.active)
    }
}
