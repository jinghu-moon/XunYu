use super::*;

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
        config::load(&self.config_path)
    }

    pub(super) fn save(&self, cfg: &Config) -> Result<()> {
        config::save(&self.config_path, cfg)
    }

    pub(super) fn sync_shims(&self, cfg: &Config) -> Result<()> {
        let entries = shim_gen::config_to_sync_entries(cfg);
        let report = shim_gen::sync_all(
            &entries,
            &self.shims_dir,
            &self.template_path,
            &self.template_gui_path,
        )?;
        for (name, err) in report.errors {
            ui_println!("Warning: shim sync failed [{name}]: {err}");
        }
        Ok(())
    }

    pub(super) fn sync_shells(&self, cfg: &Config, setup: Option<&AliasSetupCmd>) -> Result<()> {
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

        for (enabled, backend) in backends {
            if !enabled {
                continue;
            }
            match backend.update(cfg) {
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
