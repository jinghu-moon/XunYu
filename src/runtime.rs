use std::env;
use std::sync::OnceLock;

use crate::cli::Xun;

#[derive(Clone, Copy, Default)]
pub(crate) struct RuntimeOptions {
    quiet: bool,
    verbose: bool,
    non_interactive: bool,
    no_color: bool,
}

static RUNTIME: OnceLock<RuntimeOptions> = OnceLock::new();

pub(crate) fn init_direct(no_color: bool, quiet: bool, verbose: bool, non_interactive: bool) {
    init_from_flags(no_color, quiet, verbose, non_interactive);
}

/// 从布尔标志初始化运行时（xun_core dispatch 入口使用）。
pub(crate) fn init_from_flags(no_color: bool, quiet: bool, verbose: bool, non_interactive: bool) {
    let opts = compute_options_from_flags(
        no_color,
        quiet,
        verbose,
        non_interactive,
        env_flag("XUN_QUIET"),
        env_flag("XUN_VERBOSE"),
        env_flag("XUN_NON_INTERACTIVE"),
        env::var_os("NO_COLOR").is_some(),
    );
    let _ = RUNTIME.set(opts);
    if opts.no_color {
        console::set_colors_enabled(false);
    }
}

pub(crate) fn is_quiet() -> bool {
    RUNTIME.get().copied().unwrap_or_default().quiet
}

pub(crate) fn is_verbose() -> bool {
    RUNTIME.get().copied().unwrap_or_default().verbose
}

pub(crate) fn is_non_interactive() -> bool {
    RUNTIME.get().copied().unwrap_or_default().non_interactive
}

pub(crate) fn is_no_color() -> bool {
    RUNTIME.get().copied().unwrap_or_default().no_color
}

fn env_flag(key: &str) -> bool {
    env_flag_value(env::var(key).ok().as_deref())
}

fn env_flag_value(raw: Option<&str>) -> bool {
    matches!(raw, Some("1") | Some("true") | Some("yes"))
}

#[cfg_attr(not(test), allow(dead_code))]
fn compute_options(
    args: &Xun,
    env_quiet: bool,
    env_verbose: bool,
    env_non_interactive: bool,
    env_no_color: bool,
) -> RuntimeOptions {
    compute_options_from_flags(
        args.no_color,
        args.quiet,
        args.verbose,
        args.non_interactive,
        env_quiet,
        env_verbose,
        env_non_interactive,
        env_no_color,
    )
}

fn compute_options_from_flags(
    no_color: bool,
    quiet: bool,
    verbose: bool,
    non_interactive: bool,
    env_quiet: bool,
    env_verbose: bool,
    env_non_interactive: bool,
    env_no_color: bool,
) -> RuntimeOptions {
    let mut opts = RuntimeOptions {
        quiet: quiet || env_quiet,
        verbose: verbose || env_verbose,
        non_interactive: non_interactive || env_non_interactive,
        no_color: no_color || env_no_color,
    };
    if opts.verbose {
        opts.quiet = false;
    }
    opts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{InitCmd, SubCommand, Xun};

    fn base_args() -> Xun {
        Xun {
            no_color: false,
            quiet: false,
            verbose: false,
            non_interactive: false,
            cmd: SubCommand::Init(InitCmd {
                shell: "powershell".to_string(),
            }),
        }
    }

    #[test]
    fn compute_options_respects_args_and_env_flags() {
        let mut args = base_args();
        args.quiet = true;

        let opts = compute_options(&args, false, false, false, false);
        assert!(opts.quiet);
        assert!(!opts.verbose);
        assert!(!opts.non_interactive);
        assert!(!opts.no_color);

        let opts = compute_options(&base_args(), true, false, true, true);
        assert!(opts.quiet);
        assert!(!opts.verbose);
        assert!(opts.non_interactive);
        assert!(opts.no_color);
    }

    #[test]
    fn compute_options_verbose_disables_quiet() {
        let mut args = base_args();
        args.quiet = true;
        args.verbose = true;

        let opts = compute_options(&args, false, false, false, false);
        assert!(opts.verbose);
        assert!(!opts.quiet, "verbose should override quiet");

        let opts = compute_options(&base_args(), true, true, false, false);
        assert!(opts.verbose);
        assert!(!opts.quiet, "env verbose should override env quiet");
    }

    #[test]
    fn env_flag_value_accepts_common_truthy_values() {
        assert!(env_flag_value(Some("1")));
        assert!(env_flag_value(Some("true")));
        assert!(env_flag_value(Some("yes")));
        assert!(!env_flag_value(Some("0")));
        assert!(!env_flag_value(Some("no")));
        assert!(!env_flag_value(None));
    }
}
