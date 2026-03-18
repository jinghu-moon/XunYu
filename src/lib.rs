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
mod batch_rename;

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

fn wants_version_only(args: &[String]) -> bool {
    let cli_args = match args.get(1..) {
        Some(value) if !value.is_empty() => value,
        _ => return false,
    };
    cli_args.iter().all(|arg| is_global_cli_flag(arg))
        && cli_args.iter().any(|arg| arg == "--version")
}

pub fn run_from_env(invoked_name: Option<&str>) {
    let mut raw_args: Vec<String> = std::env::args().collect();
    if raw_args.len() > 1 && raw_args[1] == "del" {
        raw_args[1] = "delete".to_string();
    }
    for arg in raw_args.iter_mut().skip(1) {
        if arg == "-bm" {
            *arg = "--bookmark".to_string();
        }
    }

    let cmd = resolve_command_name(&raw_args, invoked_name);
    if wants_version_only(&raw_args) {
        out_println!("{} {}", cmd, env!("CARGO_PKG_VERSION"));
        return;
    }

    let args: Vec<&str> = raw_args.iter().map(|arg| arg.as_str()).collect();
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

    if args.version {
        out_println!("{} {}", cmd, env!("CARGO_PKG_VERSION"));
        return;
    }

    runtime::init(&args);
    if let Err(err) = commands::dispatch(args) {
        output::print_cli_error(&err);
        std::process::exit(err.code);
    }
}
