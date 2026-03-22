#[macro_use]
mod macros;

pub mod acl;
#[cfg(feature = "alias")]
pub mod alias;
mod cli;
mod commands;
mod config;
mod ctx_store;
#[cfg(feature = "desktop")]
mod desktop;
mod env_core;
mod find;
mod fuzzy;
mod model;
mod output;
pub mod path_guard;
mod ports;
mod proc;
mod runtime;
mod security;
mod store;
mod suggest;
mod util;
mod windows;

use std::path::Path;
use std::time::Instant;

#[cfg(feature = "protect")]
pub(crate) mod protect;

#[cfg(feature = "crypt")]
pub(crate) mod age_wrapper;

#[cfg(feature = "crypt")]
pub(crate) mod filevault;

#[cfg(feature = "diff")]
mod diff;

#[cfg(feature = "cstat")]
mod cstat;

#[cfg(feature = "batch_rename")]
pub mod batch_rename;

#[cfg(feature = "img")]
mod img;

fn invoked_command_name(raw: &str) -> String {
    Path::new(raw)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("xun")
        .to_string()
}

fn resolve_command_name(raw_args: &[String], invoked_name: Option<&str>) -> String {
    invoked_name
        .map(str::to_string)
        .or_else(|| raw_args.first().map(|arg| invoked_command_name(arg)))
        .unwrap_or_else(|| "xun".to_string())
}

fn is_global_cli_flag(arg: &str) -> bool {
    matches!(
        arg,
        "--version" | "--no-color" | "-q" | "--quiet" | "-v" | "--verbose" | "--non-interactive"
    )
}

fn normalize_top_level_aliases(raw_args: &mut [String]) {
    for arg in raw_args.iter_mut().skip(1) {
        if is_global_cli_flag(arg) {
            continue;
        }
        if arg.starts_with('-') {
            return;
        }
        match arg.as_str() {
            "bak" => *arg = "backup".to_string(),
            "rst" => *arg = "restore".to_string(),
            _ => {}
        }
        return;
    }
}

fn env_flag_present(names: &[&str]) -> bool {
    names.iter().any(|name| std::env::var_os(name).is_some())
}

fn command_timing_enabled(cmd: &cli::SubCommand) -> bool {
    if env_flag_present(&["XUN_CMD_TIMING"]) {
        return true;
    }
    match cmd {
        cli::SubCommand::Backup(_) => env_flag_present(&["XUN_BACKUP_TIMING", "XUN_BAK_TIMING"]),
        cli::SubCommand::Restore(_) => env_flag_present(&["XUN_RESTORE_TIMING"]),
        _ => false,
    }
}

fn wants_version_only(args: &[String]) -> bool {
    let cli_args = match args.get(1..) {
        Some(value) if !value.is_empty() => value,
        _ => return false,
    };
    cli_args.iter().all(|arg| is_global_cli_flag(arg))
        && cli_args.iter().any(|arg| arg == "--version")
}

pub fn run_from_env(invoked_name: Option<&str>) {
    let t_total = Instant::now();
    let mut raw_args: Vec<String> = std::env::args().collect();
    let t_prepare = Instant::now();
    if raw_args.len() > 1 && raw_args[1] == "del" {
        raw_args[1] = "delete".to_string();
    }
    for arg in raw_args.iter_mut().skip(1) {
        if arg == "-bm" {
            *arg = "--bookmark".to_string();
        }
    }
    normalize_top_level_aliases(&mut raw_args);
    let elapsed_prepare = t_prepare.elapsed();

    let cmd = resolve_command_name(&raw_args, invoked_name);
    if wants_version_only(&raw_args) {
        out_println!("{} {}", cmd, env!("CARGO_PKG_VERSION"));
        return;
    }

    let args: Vec<&str> = raw_args.iter().map(|arg| arg.as_str()).collect();
    let t_parse = Instant::now();
    let args: cli::Xun = <cli::Xun as argh::FromArgs>::from_args(&[cmd.as_str()], &args[1..])
        .unwrap_or_else(|early_exit| {
            std::process::exit(match early_exit.status {
                Ok(()) => {
                    println!("{}", early_exit.output);
                    0
                }
                Err(()) => {
                    eprintln!(
                        "{}
Run {} --help for more information.",
                        early_exit.output, cmd
                    );
                    1
                }
            })
        });
    let elapsed_parse = t_parse.elapsed();
    let timing = command_timing_enabled(&args.cmd);

    if args.version {
        out_println!("{} {}", cmd, env!("CARGO_PKG_VERSION"));
        return;
    }

    let t_runtime = Instant::now();
    runtime::init(&args);
    let elapsed_runtime = t_runtime.elapsed();
    let t_dispatch = Instant::now();
    let result = commands::dispatch(args);
    let elapsed_dispatch = t_dispatch.elapsed();
    if timing {
        eprintln!("xun timing:");
        eprintln!("  [prepare] {:>5}ms", elapsed_prepare.as_millis());
        eprintln!("  [parse]   {:>5}ms", elapsed_parse.as_millis());
        eprintln!("  [runtime] {:>5}ms", elapsed_runtime.as_millis());
        eprintln!("  [dispatch]{:>5}ms", elapsed_dispatch.as_millis());
        eprintln!("  [total]   {:>5}ms", t_total.elapsed().as_millis());
    }
    if let Err(err) = result {
        output::print_cli_error(&err);
        std::process::exit(err.code);
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_top_level_aliases;

    #[test]
    fn normalize_top_level_aliases_maps_bak_and_rst() {
        let mut args = vec!["xun".to_string(), "bak".to_string()];
        normalize_top_level_aliases(&mut args);
        assert_eq!(args[1], "backup");

        let mut args = vec!["xun".to_string(), "--quiet".to_string(), "rst".to_string()];
        normalize_top_level_aliases(&mut args);
        assert_eq!(args[2], "restore");
    }
}
