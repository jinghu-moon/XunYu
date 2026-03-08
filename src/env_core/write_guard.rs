use super::*;

impl EnvManager {
    pub(super) fn with_write_guard<F>(
        &self,
        scope: EnvScope,
        action: &str,
        snapshot_before: bool,
        op: F,
    ) -> EnvResult<()>
    where
        F: FnOnce() -> EnvResult<Option<EnvEvent>>,
    {
        if !matches!(scope, EnvScope::User | EnvScope::System | EnvScope::All) {
            return Err(EnvError::ScopeNotWritable(scope));
        }
        if uac::requires_elevation(scope) {
            return Err(EnvError::PermissionDenied(uac::elevation_hint(scope)));
        }

        lock::try_with_lock(&self.cfg, action, || {
            if snapshot_before {
                let _ = snapshot::create_snapshot(&self.cfg, &format!("pre-{action}"));
            }
            match op() {
                Ok(evt_opt) => {
                    let (name, message) = if let Some(evt) = evt_opt.as_ref() {
                        (evt.name.clone(), evt.message.clone())
                    } else {
                        (None, Some("write operation completed".to_string()))
                    };
                    let _ = audit::append_audit(
                        &self.cfg,
                        &EnvAuditEntry {
                            at: events::now_iso(),
                            action: action.to_string(),
                            scope,
                            result: "ok".to_string(),
                            name,
                            message,
                        },
                    );
                    if let Some(evt) = evt_opt {
                        self.emit_event(evt);
                    }
                    Ok(())
                }
                Err(err) => {
                    let _ = audit::append_audit(
                        &self.cfg,
                        &EnvAuditEntry {
                            at: events::now_iso(),
                            action: action.to_string(),
                            scope,
                            result: "error".to_string(),
                            name: None,
                            message: Some(err.to_string()),
                        },
                    );
                    Err(err)
                }
            }
        })
    }

    pub(super) fn emit_event(&self, event: EnvEvent) {
        if let Some(cb) = &self.event_cb {
            cb(event);
        }
    }
}
