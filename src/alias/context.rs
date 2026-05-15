use super::*;

/// 当 `XUN_ALIAS_TIMING=1` 时输出计时信息到 stderr
#[inline]
fn timing_enabled() -> bool {
    std::env::var("XUN_ALIAS_TIMING").as_deref() == Ok("1")
}

macro_rules! t_print {
    ($($arg:tt)*) => {
        if timing_enabled() {
            eprintln!("[timing] {}", format_args!($($arg)*));
        }
    }
}

/// 供同级模块调用的总耗时打印
#[inline]
pub(super) fn t_print_total(label: &str, t0: std::time::Instant) {
    if timing_enabled() {
        eprintln!(
            "[timing] {label}: total={:.1}ms",
            t0.elapsed().as_secs_f64() * 1000.0
        );
    }
}

pub(super) struct AliasCtx {
    pub(super) config_path: PathBuf,
    pub(super) shims_dir: PathBuf,
    pub(super) template_path: PathBuf,
    pub(super) template_gui_path: PathBuf,
    pub(super) config_dir: PathBuf,
}

impl AliasCtx {
    pub(super) fn from_cli(cli: &AliasCmd) -> Self {
        let override_path = cli.config.as_deref().map(Path::new);
        let config_path = config::config_path(override_path);
        let shims_dir = config::shims_dir(&config_path);
        let template_path = config::shim_template_path(&config_path);
        let template_gui_path = config::shim_gui_template_path(&config_path);
        let config_dir = config_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        Self {
            config_path,
            shims_dir,
            template_path,
            template_gui_path,
            config_dir,
        }
    }

    pub(super) fn load(&self) -> Result<Config> {
        let t0 = std::time::Instant::now();
        let r = config::load(&self.config_path);
        t_print!("load: total={:.1}ms", t0.elapsed().as_secs_f64() * 1000.0);
        r
    }

    pub(super) fn save(&self, cfg: &Config) -> Result<()> {
        let t0 = std::time::Instant::now();
        let r = config::save(&self.config_path, cfg);
        t_print!("save: total={:.1}ms", t0.elapsed().as_secs_f64() * 1000.0);
        r
    }

    pub(super) fn sync_shims(&self, cfg: &Config) -> Result<()> {
        let t0 = std::time::Instant::now();
        let entries = shim_gen::config_to_sync_entries(cfg);
        let t_entries = t0.elapsed();

        let t1 = std::time::Instant::now();
        let report = shim_gen::sync_all(
            &entries,
            &self.shims_dir,
            &self.template_path,
            &self.template_gui_path,
        )?;
        let t_sync = t1.elapsed();

        t_print!(
            "sync_shims: entries={:.1}ms sync={:.1}ms total={:.1}ms (n={} created={} removed={} errors={})",
            t_entries.as_secs_f64() * 1000.0,
            t_sync.as_secs_f64() * 1000.0,
            t0.elapsed().as_secs_f64() * 1000.0,
            entries.len(),
            report.created.len(),
            report.removed.len(),
            report.errors.len()
        );
        for (name, err) in report.errors {
            ui_println!("Warning: shim sync failed [{name}]: {err}");
        }
        Ok(())
    }

    pub(super) fn sync_selected_shims(&self, entries: &[shim_gen::SyncEntry]) -> Result<()> {
        let t0 = std::time::Instant::now();
        let report = shim_gen::sync_entries(
            entries,
            &self.shims_dir,
            &self.template_path,
            &self.template_gui_path,
        )?;
        t_print!(
            "sync_selected_shims: total={:.1}ms (n={} created={} removed={} errors={})",
            t0.elapsed().as_secs_f64() * 1000.0,
            entries.len(),
            report.created.len(),
            report.removed.len(),
            report.errors.len()
        );
        for (name, err) in report.errors {
            ui_println!("Warning: shim sync failed [{name}]: {err}");
        }
        Ok(())
    }

    pub(super) fn sync_shell_alias_shim(&self, name: &str, alias: &ShellAlias) -> Result<()> {
        shim_gen::sync_shell_alias(
            &self.shims_dir,
            &self.template_path,
            &self.template_gui_path,
            name,
            alias,
        )
    }

    pub(super) fn sync_app_alias_shim(&self, name: &str, alias: &AppAlias) -> Result<()> {
        shim_gen::sync_app_alias(
            &self.shims_dir,
            &self.template_path,
            &self.template_gui_path,
            name,
            alias,
        )
    }

    pub(super) fn sync_shells(&self, cfg: &Config, setup: Option<&AliasSetupArgs>) -> Result<()> {
        let mut skip_cmd = false;
        let mut skip_ps = false;
        let mut skip_bash = false;
        let mut skip_nu = false;
        if let Some(setup) = setup {
            skip_cmd = setup.no_cmd;
            skip_ps = setup.no_ps;
            skip_bash = setup.no_bash || setup.core_only;
            skip_nu = setup.no_nu || setup.core_only;
        }

        #[allow(unused_mut)]
        let mut backends: Vec<(bool, Box<dyn ShellBackend>)> = vec![
            (!skip_cmd, Box::new(CmdBackend::new(&self.config_dir))),
            (!skip_ps, Box::new(PsBackend::new(None))),
        ];
        #[cfg(feature = "alias-shell-extra")]
        {
            backends.push((!skip_bash, Box::new(BashBackend::new(None))));
            backends.push((!skip_nu, Box::new(NuBackend::new(None))));
        }
        #[cfg(not(feature = "alias-shell-extra"))]
        {
            let _ = (skip_bash, skip_nu);
        }

        let t_shells = std::time::Instant::now();
        for (enabled, backend) in backends {
            if !enabled {
                continue;
            }
            let t_b = std::time::Instant::now();
            let result = backend.update(cfg);
            t_print!(
                "sync_shells: {}={:.1}ms",
                backend.name(),
                t_b.elapsed().as_secs_f64() * 1000.0
            );
            match result {
                Ok(shell::UpdateResult::Written { path }) => {
                    ui_println!("Updated {} profile: {}", backend.name(), path.display());
                }
                Ok(shell::UpdateResult::Skipped { reason }) => {
                    ui_println!("Skipped {}: {reason}", backend.name());
                }
                Err(err) => {
                    ui_println!("Warning: {} backend update failed: {err}", backend.name());
                }
            }
        }
        t_print!(
            "sync_shells: total={:.1}ms",
            t_shells.elapsed().as_secs_f64() * 1000.0
        );
        Ok(())
    }
}

pub(super) fn split_csv_multi(values: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for value in values {
        for part in value.split(',') {
            let part = part.trim();
            if !part.is_empty() {
                out.push(part.to_string());
            }
        }
    }
    out.sort();
    out.dedup();
    out
}
