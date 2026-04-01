use std::path::Path;
use std::time::Instant;

use argh::FromArgs;

use crate::bookmark::commands;
use crate::cli::BookmarkSubCommand;
use crate::output;

#[derive(FromArgs)]
#[argh(description = "bm - bookmark CLI")]
struct BookmarkCli {
    /// disable ANSI colors
    #[argh(switch)]
    no_color: bool,

    /// show version and exit
    #[argh(switch)]
    version: bool,

    /// suppress UI output
    #[argh(switch, short = 'q')]
    quiet: bool,

    /// verbose output
    #[argh(switch, short = 'v')]
    verbose: bool,

    /// force non-interactive mode
    #[argh(switch)]
    non_interactive: bool,

    #[argh(subcommand)]
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
    cli_args.iter().all(|arg| is_global_flag(arg))
        && cli_args.iter().any(|arg| arg == "--version")
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

    let args: Vec<&str> = raw_args.iter().map(|arg| arg.as_str()).collect();
    let parse_start = Instant::now();
    let parsed: BookmarkCli =
        <BookmarkCli as argh::FromArgs>::from_args(&[cmd.as_str()], &args[1..]).unwrap_or_else(
            |early_exit| {
                std::process::exit(match early_exit.status {
                    Ok(()) => {
                        println!("{}", early_exit.output);
                        0
                    }
                    Err(()) => {
                        eprintln!(
                            "{}\nRun {} --help for more information.",
                            early_exit.output, cmd
                        );
                        1
                    }
                })
            },
        );
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
