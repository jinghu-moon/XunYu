use std::path::Path;
use std::time::Instant;

use clap::Parser;

use crate::bookmark::commands;
use crate::cli::BookmarkSubCommand;
use crate::output;

#[derive(Parser, Debug, Clone)]
#[command(name = "bm", about = "bm - bookmark CLI")]
struct BookmarkCli {
    /// disable ANSI colors
    #[arg(long)]
    no_color: bool,

    /// show version and exit
    #[arg(long)]
    version: bool,

    /// suppress UI output
    #[arg(short = 'q', long)]
    quiet: bool,

    /// verbose output
    #[arg(short = 'v', long)]
    verbose: bool,

    /// force non-interactive mode
    #[arg(long)]
    non_interactive: bool,

    #[command(subcommand)]
    cmd: BookmarkSubCommand,
}

fn invoked_command_name(raw: &str) -> String {
    Path::new(raw)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or("bm")
        .to_string()
}

fn resolve_command_name(raw_args: &[String], invoked_name: Option<&str>) -> String {
    invoked_name
        .map(str::to_string)
        .or_else(|| raw_args.first().map(|arg| invoked_command_name(arg)))
        .unwrap_or_else(|| "bm".to_string())
}

fn is_global_flag(arg: &str) -> bool {
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
    cli_args.iter().all(|arg| is_global_flag(arg)) && cli_args.iter().any(|arg| arg == "--version")
}

fn timing_enabled() -> bool {
    ["XUN_BM_TIMING", "XUN_BOOKMARK_TIMING", "XUN_CMD_TIMING"]
        .into_iter()
        .any(|name| std::env::var_os(name).is_some())
}

pub(crate) fn run_from_env(invoked_name: Option<&str>) {
    let total = Instant::now();
    let raw_args: Vec<String> = std::env::args().collect();
    let cmd = resolve_command_name(&raw_args, invoked_name);
    if wants_version_only(&raw_args) {
        out_println!("{} {}", cmd, env!("CARGO_PKG_VERSION"));
        return;
    }

    let parse_start = Instant::now();
    let parsed = BookmarkCli::try_parse_from(&raw_args).unwrap_or_else(|e| {
        e.print().expect("failed to print clap error");
        std::process::exit(
            if e.use_stderr() {
                1
            } else {
                0
            },
        )
    });
    let parse_ms = parse_start.elapsed().as_millis();
    let timing = timing_enabled();

    if parsed.version {
        out_println!("{} {}", cmd, env!("CARGO_PKG_VERSION"));
        return;
    }

    let runtime_start = Instant::now();
    crate::runtime::init_direct(
        parsed.no_color,
        parsed.quiet,
        parsed.verbose,
        parsed.non_interactive,
    );
    let runtime_ms = runtime_start.elapsed().as_millis();

    let dispatch_start = Instant::now();
    let result = commands::cmd_bookmark(crate::cli::BookmarkCmd { cmd: parsed.cmd });
    let dispatch_ms = dispatch_start.elapsed().as_millis();

    if timing {
        eprintln!("bm timing:");
        eprintln!("  [parse]   {:>5}ms", parse_ms);
        eprintln!("  [runtime] {:>5}ms", runtime_ms);
        eprintln!("  [dispatch]{:>5}ms", dispatch_ms);
        eprintln!("  [total]   {:>5}ms", total.elapsed().as_millis());
    }

    if let Err(err) = result {
        output::print_cli_error(&err);
        std::process::exit(err.code);
    }
}
