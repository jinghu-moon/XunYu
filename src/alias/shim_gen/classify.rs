use super::*;

#[allow(dead_code)]
pub(crate) fn classify_command(command: &str) -> ShimKind {
    classify_mode(command, AliasMode::Auto)
}

pub(crate) fn classify_mode(command: &str, mode: AliasMode) -> ShimKind {
    let trimmed = command.trim();
    match mode {
        AliasMode::Cmd => {
            return ShimKind::Cmd {
                command: trimmed.to_string(),
            };
        }
        AliasMode::Exe => {
            if let Some((path, args)) = parse_exe_candidate(trimmed) {
                return ShimKind::Exe {
                    path,
                    fixed_args: args,
                };
            }
            return ShimKind::Cmd {
                command: trimmed.to_string(),
            };
        }
        AliasMode::Auto => {}
    }

    if contains_shell_operators(trimmed) {
        return ShimKind::Cmd {
            command: trimmed.to_string(),
        };
    }
    if let Some((path, args)) = parse_exe_candidate(trimmed) {
        return ShimKind::Exe {
            path,
            fixed_args: args,
        };
    }
    ShimKind::Cmd {
        command: trimmed.to_string(),
    }
}

fn parse_exe_candidate(command: &str) -> Option<(String, Option<String>)> {
    let mut parts = command.splitn(2, char::is_whitespace);
    let exe = parts.next()?.trim();
    let rest = parts.next().map(str::trim).filter(|v| !v.is_empty());

    let path = Path::new(exe);
    if path.is_absolute() && path.is_file() {
        return Some((exe.to_string(), rest.map(str::to_string)));
    }

    find_in_path(exe).map(|p| (p, rest.map(str::to_string)))
}

fn contains_shell_operators(command: &str) -> bool {
    command
        .chars()
        .any(|ch| matches!(ch, '|' | '&' | '<' | '>' | ';' | '`'))
}

fn find_in_path(exe: &str) -> Option<String> {
    let path_var = std::env::var("PATH").ok()?;
    let candidates = executable_candidates(exe);
    for dir in path_var.split(';') {
        if dir.is_empty() {
            continue;
        }
        let base = Path::new(dir);
        for name in &candidates {
            let p = base.join(name);
            if p.is_file() {
                return Some(p.to_string_lossy().into_owned());
            }
        }
    }
    None
}

fn executable_candidates(exe: &str) -> Vec<String> {
    if Path::new(exe).extension().is_some() {
        return vec![exe.to_string()];
    }
    vec![
        format!("{exe}.exe"),
        format!("{exe}.cmd"),
        format!("{exe}.bat"),
        format!("{exe}.com"),
    ]
}
