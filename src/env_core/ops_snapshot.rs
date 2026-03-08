use super::*;

impl EnvManager {
    pub fn snapshot_create(&self, desc: Option<&str>) -> EnvResult<SnapshotMeta> {
        let description = desc.unwrap_or("manual snapshot");
        let meta = snapshot::create_snapshot(&self.cfg, description)?;
        self.emit_event(events::build_event(
            EnvEventType::Snapshot,
            EnvScope::All,
            None,
            Some(format!("snapshot created: {}", meta.id)),
        ));
        Ok(meta)
    }

    pub fn snapshot_list(&self) -> EnvResult<Vec<SnapshotMeta>> {
        snapshot::list_snapshots(&self.cfg)
    }

    pub fn snapshot_prune(&self, keep: usize) -> EnvResult<usize> {
        let keep = keep.min(10_000);
        let removed = lock::try_with_lock(&self.cfg, "snapshot.prune", || {
            snapshot::prune_snapshots(&self.cfg, keep)
        })?;
        let message = format!("snapshot prune keep={}, removed={}", keep, removed);
        let _ = audit::append_audit(
            &self.cfg,
            &EnvAuditEntry {
                at: events::now_iso(),
                action: "snapshot.prune".to_string(),
                scope: EnvScope::All,
                result: "ok".to_string(),
                name: None,
                message: Some(message.clone()),
            },
        );
        self.emit_event(events::build_event(
            EnvEventType::Snapshot,
            EnvScope::All,
            None,
            Some(message),
        ));
        Ok(removed)
    }

    pub fn snapshot_restore(
        &self,
        scope: EnvScope,
        snapshot_id: Option<&str>,
        latest: bool,
    ) -> EnvResult<SnapshotMeta> {
        let target = if let Some(id) = snapshot_id {
            let snap = snapshot::load_snapshot_by_id(&self.cfg, id)?;
            SnapshotMeta {
                id: snap.id,
                description: snap.description,
                created_at: snap.created_at,
                path: self.cfg.snapshot_dir().join(format!("{id}.json")),
            }
        } else if latest {
            let snap = snapshot::load_latest_snapshot(&self.cfg)?;
            SnapshotMeta {
                id: snap.id,
                description: snap.description,
                created_at: snap.created_at,
                path: self.cfg.snapshot_dir().join("latest.json"),
            }
        } else {
            return Err(EnvError::InvalidInput(
                "restore requires --id <id> or --latest".to_string(),
            ));
        };

        self.with_write_guard(scope, "snapshot.restore", true, || {
            let snapshot = snapshot::load_snapshot_by_id(&self.cfg, &target.id)
                .or_else(|_| snapshot::load_latest_snapshot(&self.cfg))?;
            snapshot::restore_snapshot(&snapshot, scope)?;
            Ok(Some(events::build_event(
                EnvEventType::Snapshot,
                scope,
                None,
                Some(format!("snapshot restored: {}", target.id)),
            )))
        })?;

        Ok(target)
    }

    pub fn diff_live(&self, scope: EnvScope, snapshot_id: Option<&str>) -> EnvResult<EnvDiff> {
        let snapshot = match snapshot_id {
            Some(id) => snapshot::load_snapshot_by_id(&self.cfg, id)?,
            None => snapshot::load_latest_snapshot(&self.cfg)?,
        };
        let reference = snapshot_to_env_vars(&snapshot, scope);
        let live = registry::list_vars(scope)?;
        Ok(diff::diff_var_lists(&reference, &live))
    }

    pub fn diff_since(&self, scope: EnvScope, since_raw: &str) -> EnvResult<EnvDiff> {
        let since = parse_since_datetime(since_raw)?;
        let baseline = select_snapshot_id_by_since(&self.cfg, since)?
            .ok_or_else(|| EnvError::NotFound("no snapshots found".to_string()))?;
        self.diff_live(scope, Some(&baseline))
    }
}

fn snapshot_to_env_vars(snapshot: &types::Snapshot, scope: EnvScope) -> Vec<EnvVar> {
    let mut out = Vec::new();
    match scope {
        EnvScope::User => {
            out.extend(snapshot.user_vars.iter().cloned().map(|v| EnvVar {
                scope: EnvScope::User,
                inferred_kind: var_type::infer_var_kind(&v.name, &v.raw_value),
                name: v.name,
                raw_value: v.raw_value,
                reg_type: v.reg_type,
            }));
        }
        EnvScope::System => {
            out.extend(snapshot.system_vars.iter().cloned().map(|v| EnvVar {
                scope: EnvScope::System,
                inferred_kind: var_type::infer_var_kind(&v.name, &v.raw_value),
                name: v.name,
                raw_value: v.raw_value,
                reg_type: v.reg_type,
            }));
        }
        EnvScope::All => {
            out.extend(snapshot.user_vars.iter().cloned().map(|v| EnvVar {
                scope: EnvScope::User,
                inferred_kind: var_type::infer_var_kind(&v.name, &v.raw_value),
                name: v.name,
                raw_value: v.raw_value,
                reg_type: v.reg_type,
            }));
            out.extend(snapshot.system_vars.iter().cloned().map(|v| EnvVar {
                scope: EnvScope::System,
                inferred_kind: var_type::infer_var_kind(&v.name, &v.raw_value),
                name: v.name,
                raw_value: v.raw_value,
                reg_type: v.reg_type,
            }));
        }
    }
    out
}

fn parse_since_datetime(raw: &str) -> EnvResult<DateTime<Utc>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(EnvError::InvalidInput("since cannot be empty".to_string()));
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(dt.with_timezone(&Utc));
    }
    if let Ok(dt) = NaiveDateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S") {
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
    }
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        let dt = date
            .and_hms_opt(0, 0, 0)
            .ok_or_else(|| EnvError::InvalidInput(format!("invalid since date '{}'", raw)))?;
        return Ok(DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
    }
    Err(EnvError::InvalidInput(format!(
        "invalid since '{}', expected RFC3339 or YYYY-MM-DD or YYYY-MM-DD HH:MM:SS",
        raw
    )))
}

fn parse_snapshot_time(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn select_snapshot_id_by_since(
    cfg: &EnvCoreConfig,
    since: DateTime<Utc>,
) -> EnvResult<Option<String>> {
    let metas = snapshot::list_snapshots(cfg)?;
    if metas.is_empty() {
        return Ok(None);
    }
    let first_id = metas.first().map(|m| m.id.clone());

    let mut best: Option<(DateTime<Utc>, String)> = None;
    let mut oldest: Option<(DateTime<Utc>, String)> = None;

    for meta in metas {
        let ts = match parse_snapshot_time(&meta.created_at) {
            Some(v) => v,
            None => continue,
        };
        if oldest.as_ref().map(|(d, _)| ts < *d).unwrap_or(true) {
            oldest = Some((ts, meta.id.clone()));
        }
        if ts <= since && best.as_ref().map(|(d, _)| ts > *d).unwrap_or(true) {
            best = Some((ts, meta.id.clone()));
        }
    }

    if let Some((_, id)) = best {
        return Ok(Some(id));
    }
    if let Some((_, id)) = oldest {
        return Ok(Some(id));
    }

    Ok(first_id)
}
