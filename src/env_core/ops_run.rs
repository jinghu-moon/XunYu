use super::*;

impl EnvManager {
    pub fn template_expand(&self, scope: EnvScope, input: &str) -> EnvResult<TemplateExpandResult> {
        template::template_expand(scope, input)
    }

    pub fn template_validate(
        &self,
        scope: EnvScope,
        input: &str,
    ) -> EnvResult<TemplateValidationReport> {
        template::template_validate(scope, input)
    }

    pub fn runtime_env(
        &self,
        scope: EnvScope,
        env_files: &[PathBuf],
        set_pairs: &[(String, String)],
    ) -> EnvResult<std::collections::BTreeMap<String, String>> {
        template::build_runtime_env(scope, env_files, set_pairs)
    }

    pub fn render_shell_exports(
        &self,
        scope: EnvScope,
        env_files: &[PathBuf],
        set_pairs: &[(String, String)],
        shell: ShellExportFormat,
    ) -> EnvResult<String> {
        let env_map = self.runtime_env(scope, env_files, set_pairs)?;
        Ok(template::render_shell_exports(&env_map, shell))
    }

    pub fn export_live(
        &self,
        scope: EnvScope,
        format: LiveExportFormat,
        env_files: &[PathBuf],
        set_pairs: &[(String, String)],
    ) -> EnvResult<String> {
        let env_map = self.runtime_env(scope, env_files, set_pairs)?;
        template::render_live_export(scope, &env_map, format)
    }

    pub fn merged_env_pairs(
        &self,
        scope: EnvScope,
        env_files: &[PathBuf],
        set_pairs: &[(String, String)],
    ) -> EnvResult<Vec<(String, String)>> {
        let env_map = self.runtime_env(scope, env_files, set_pairs)?;
        Ok(env_map.into_iter().collect())
    }

    pub fn notify_run_result(
        &self,
        command_line: &str,
        exit_code: Option<i32>,
        success: bool,
    ) -> EnvResult<bool> {
        notifier::notify_run_result(&self.cfg, command_line, exit_code, success)
    }

    pub fn dependency_tree(
        &self,
        scope: EnvScope,
        root: &str,
        max_depth: usize,
    ) -> EnvResult<EnvDepTree> {
        let vars = self.list_vars(scope)?;
        dep_graph::build_tree(scope, &vars, root, max_depth)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn run_command(
        &self,
        scope: EnvScope,
        env_files: &[PathBuf],
        set_pairs: &[(String, String)],
        command: &[String],
        cwd: Option<&Path>,
        schema_check: bool,
        notify: bool,
        capture_output: bool,
        max_output: usize,
    ) -> EnvResult<RunCommandResult> {
        if command.is_empty() {
            return Err(EnvError::InvalidInput(
                "run requires command tokens".to_string(),
            ));
        }

        if schema_check {
            let report = self.validate_schema(scope, false)?;
            if report.errors > 0 {
                return Err(EnvError::Other(format!(
                    "schema-check failed: errors={}, warnings={}",
                    report.errors, report.warnings
                )));
            }
            if report.warnings > 0 {
                return Err(EnvError::InvalidInput(format!(
                    "schema-check failed: errors={}, warnings={}",
                    report.errors, report.warnings
                )));
            }
        }

        let env_map = self.runtime_env(scope, env_files, set_pairs)?;
        let mut cmd = Command::new(&command[0]);
        if command.len() > 1 {
            cmd.args(&command[1..]);
        }
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        cmd.env_clear();
        cmd.envs(&env_map);

        let command_line = command.join(" ");
        let result = if capture_output {
            let output = cmd.output()?;
            let cap = max_output.clamp(1024, 1024 * 1024 * 8);
            let (stdout, stdout_truncated) = truncate_output(&output.stdout, cap);
            let (stderr, stderr_truncated) = truncate_output(&output.stderr, cap);
            RunCommandResult {
                command_line,
                exit_code: output.status.code(),
                success: output.status.success(),
                stdout,
                stderr,
                truncated: stdout_truncated || stderr_truncated,
            }
        } else {
            let status = cmd.status()?;
            RunCommandResult {
                command_line,
                exit_code: status.code(),
                success: status.success(),
                stdout: String::new(),
                stderr: String::new(),
                truncated: false,
            }
        };

        if notify {
            let _ = self.notify_run_result(&result.command_line, result.exit_code, result.success);
        }

        Ok(result)
    }
}

fn truncate_output(raw: &[u8], max_output: usize) -> (String, bool) {
    if raw.len() <= max_output {
        return (String::from_utf8_lossy(raw).into_owned(), false);
    }
    (
        String::from_utf8_lossy(&raw[..max_output]).into_owned(),
        true,
    )
}
