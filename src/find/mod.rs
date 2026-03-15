mod filters;
mod glob;
mod ignore;
mod matcher;
#[cfg(windows)]
mod mft;
mod rules;
mod walker;

use std::io::{self, Write};

use comfy_table::{Attribute, Cell, Color, Table};

use crate::cli::FindCmd;
use crate::model::{ListFormat, parse_list_format};
use crate::output::{CliError, CliResult};
use crate::output::{apply_pretty_table_style, format_age, prefer_table_output, print_table};
use crate::path_guard::{PathPolicy, validate_paths};
use crate::runtime;
use filters::compile_filters;
use matcher::{determine_path_state, rule_matches};
use rules::compile_rules;
use walker::{ScanItem, scan, scan_count};

#[allow(unused_imports)]
pub(crate) use rules::{CompiledRules, PatternType, Rule, RuleKind};

pub(crate) fn cmd_find(args: FindCmd) -> CliResult {
    let mut base_dirs = if args.paths.is_empty() {
        vec![".".to_string()]
    } else {
        args.paths.clone()
    };
    let rules = compile_rules(&args)?;
    let filters = compile_filters(&args)?;
    let is_dry_run = args.dry_run || args.test_path.is_some();

    if is_dry_run {
        let test_path = args.test_path.as_deref().ok_or_else(|| {
            CliError::with_details(
                2,
                "Missing --test-path for --dry-run.",
                &["Fix: Provide --test-path <path> for rule testing."],
            )
        })?;
        let allowed = run_dry_run(&rules, test_path);
        if args.count {
            out_println!("{}", if allowed { 1 } else { 0 });
        }
        return Ok(());
    }

    let mut policy = PathPolicy::for_read();
    policy.allow_relative = true;
    let validation = validate_paths(base_dirs.clone(), &policy);
    if !validation.issues.is_empty() {
        let details: Vec<String> = validation
            .issues
            .iter()
            .map(|issue| format!("Invalid base path: {} ({})", issue.raw, issue.detail))
            .collect();
        return Err(CliError::with_details(
            2,
            "Invalid base path.".to_string(),
            &details,
        ));
    }
    base_dirs = validation
        .ok
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    let mut format = parse_list_format(&args.format).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid format: {}.", args.format),
            &["Fix: Use one of: auto | table | tsv | json"],
        )
    })?;
    if format == ListFormat::Auto {
        format = if prefer_table_output() {
            ListFormat::Table
        } else {
            ListFormat::Tsv
        };
    }

    if args.count {
        let count = scan_count(&base_dirs, &rules, &filters, false)?;
        out_println!("{count}");
        return Ok(());
    }

    let force_meta = matches!(
        format,
        ListFormat::Table | ListFormat::Tsv | ListFormat::Json
    );
    let items = scan(&base_dirs, &rules, &filters, force_meta)?;

    match format {
        ListFormat::Tsv => render_tsv(&items)?,
        ListFormat::Json => render_json(&items, &base_dirs, &args)?,
        ListFormat::Table => render_table(&items, &rules)?,
        ListFormat::Auto => unreachable!(),
    }
    Ok(())
}

fn run_dry_run(rules: &CompiledRules, raw_path: &str) -> bool {
    let mut path = raw_path.trim().replace('\\', "/");
    let mut is_dir = false;
    if path.ends_with('/') {
        is_dir = true;
        while path.ends_with('/') {
            path.pop();
        }
    }
    if path.is_empty() {
        ui_println!("path: \"{}\"  (is_dir={})", raw_path, is_dir);
        ui_println!("  → Decision: EXCLUDE (source: inherited)");
        return false;
    }

    ui_println!("path: \"{}\"  (is_dir={})", raw_path, is_dir);
    for rule in &rules.rules {
        let matched = rule_matches(rule, &path, is_dir, rules.case_sensitive);
        let action = if matched {
            if rule.kind == RuleKind::Include {
                "INCLUDE"
            } else {
                "EXCLUDE"
            }
        } else {
            "SKIP"
        };
        let kind = if rule.kind == RuleKind::Include {
            "include"
        } else {
            "exclude"
        };
        let ptype = if rule.pattern_type == PatternType::Glob {
            "glob"
        } else {
            "regex"
        };
        let pattern = format_rule_pattern(rule);
        if runtime::is_verbose() {
            let source = rule.source.as_deref().unwrap_or("-");
            ui_println!(
                "  → Rule #{} ({}, {}, \"{}\", source={}) → {}",
                rule.idx + 1,
                kind,
                ptype,
                pattern,
                source,
                action
            );
        } else {
            ui_println!(
                "  → Rule #{} ({}, {}, \"{}\") → {}",
                rule.idx + 1,
                kind,
                ptype,
                pattern,
                action
            );
        }
    }

    let inherited = if rules.default_include {
        RuleKind::Include
    } else {
        RuleKind::Exclude
    };
    let decision = determine_path_state(rules, &path, is_dir, inherited);
    let final_state = if decision.final_state == RuleKind::Include {
        "INCLUDE"
    } else {
        "EXCLUDE"
    };
    let source = if decision.explicit {
        match decision.rule_idx {
            Some(idx) => format!("explicit rule #{}", idx + 1),
            None => "explicit".to_string(),
        }
    } else {
        "inherited".to_string()
    };
    ui_println!("  → Decision: {} (source: {})", final_state, source);
    decision.final_state == RuleKind::Include
}

fn render_tsv(items: &[ScanItem]) -> CliResult {
    let mut out = io::BufWriter::new(io::stdout());
    for item in items {
        let path = format_output_path(&item.base_dir, &item.rel_path);
        let is_dir = if item.is_dir { 1 } else { 0 };
        let size = item.size.unwrap_or(0);
        let mtime = item.mtime.unwrap_or(0);
        let depth = item.depth;
        let rule_type = rule_kind_str(item.final_state);
        let rule_idx = item.rule_idx.map(|v| v + 1).unwrap_or(0);
        let decision_source = if item.explicit {
            "explicit"
        } else {
            "inherited"
        };
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            path, is_dir, size, mtime, depth, rule_type, rule_idx, decision_source
        )
        .map_err(|e| CliError::new(1, format!("Failed to write output: {e}")))?;
    }
    out.flush()
        .map_err(|e| CliError::new(1, format!("Failed to write output: {e}")))?;
    Ok(())
}

fn render_json(items: &[ScanItem], base_dirs: &[String], args: &FindCmd) -> CliResult {
    let list: Vec<serde_json::Value> = items
        .iter()
        .map(|item| {
            let path = format_output_path(&item.base_dir, &item.rel_path);
            let rule_idx = item.rule_idx.map(|v| v + 1);
            serde_json::json!({
                "path": path,
                "is_dir": item.is_dir,
                "size": item.size.unwrap_or(0),
                "mtime": item.mtime.unwrap_or(0),
                "depth": item.depth,
                "rule_type": rule_kind_str(item.final_state),
                "rule_idx": rule_idx,
                "decision_source": if item.explicit { "explicit" } else { "inherited" },
            })
        })
        .collect();

    let query = serde_json::json!({
        "paths": base_dirs,
        "case_sensitive": args.case,
        "include": args.include,
        "exclude": args.exclude,
        "regex_include": args.regex_include,
        "regex_exclude": args.regex_exclude,
        "extension": args.extension,
        "not_extension": args.not_extension,
        "name": args.name,
        "filter_file": args.filter_file,
        "size": args.size,
        "fuzzy_size": args.fuzzy_size,
        "mtime": args.mtime,
        "ctime": args.ctime,
        "atime": args.atime,
        "depth": args.depth,
        "attribute": args.attribute,
        "empty_files": args.empty_files,
        "not_empty_files": args.not_empty_files,
        "empty_dirs": args.empty_dirs,
        "not_empty_dirs": args.not_empty_dirs,
    });

    let payload = serde_json::json!({
        "query": query,
        "results": list,
    });
    let s = serde_json::to_string(&payload)
        .map_err(|e| CliError::new(1, format!("json error: {e}")))?;
    out_println!("{}", s);
    Ok(())
}

fn render_table(items: &[ScanItem], rules: &CompiledRules) -> CliResult {
    if items.is_empty() {
        ui_println!("No matches.");
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    let mut header = vec![
        Cell::new("Path")
            .add_attribute(Attribute::Bold)
            .fg(Color::Magenta),
        Cell::new("Type")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Size")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("Mtime")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
    ];
    if runtime::is_verbose() {
        header.push(
            Cell::new("Rule")
                .add_attribute(Attribute::Bold)
                .fg(Color::Blue),
        );
        header.push(
            Cell::new("Source")
                .add_attribute(Attribute::Bold)
                .fg(Color::DarkGrey),
        );
    }
    table.set_header(header);

    for item in items {
        let path = format_table_path(&item.base_dir, &item.rel_path);
        let size = format_size(item.size.unwrap_or(0));
        let mtime = if let Some(ts) = item.mtime {
            format_age(ts as u64)
        } else {
            "-".to_string()
        };
        let mut row = vec![
            Cell::new(path),
            Cell::new(if item.is_dir { "dir" } else { "file" }),
            Cell::new(size),
            Cell::new(mtime),
        ];
        if runtime::is_verbose() {
            let rule_idx = item.rule_idx.map(|v| v + 1);
            let rule_str = rule_idx
                .map(|v| format!("Rule #{v}"))
                .unwrap_or_else(|| "-".to_string());
            let source = item
                .rule_idx
                .and_then(|idx| rules.rules.get(idx))
                .and_then(|r| r.source.as_deref())
                .unwrap_or("-");
            row.push(Cell::new(rule_str));
            row.push(Cell::new(source));
        }
        table.add_row(row);
    }

    print_table(&table);
    Ok(())
}

fn format_output_path(base: &str, rel: &str) -> String {
    let mut b = base.trim().replace('\\', "/");
    while b.ends_with('/') && b.len() > 1 {
        b.pop();
    }
    if rel.is_empty() {
        return b;
    }
    if b.is_empty() {
        return rel.to_string();
    }
    if b == "/" {
        return format!("/{rel}");
    }
    format!("{}/{}", b, rel)
}

fn format_table_path(base: &str, rel: &str) -> String {
    format_output_path(base, rel).replace('/', "\\")
}

fn format_rule_pattern(rule: &Rule) -> String {
    if rule.dir_only {
        format!("{}/", rule.pattern)
    } else {
        rule.pattern.clone()
    }
}

fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    let b = bytes as f64;
    if bytes < 1024 {
        format!("{bytes}B")
    } else if b < MB {
        format!("{:.1}KB", b / KB)
    } else if b < GB {
        format!("{:.1}MB", b / MB)
    } else {
        format!("{:.2}GB", b / GB)
    }
}

fn rule_kind_str(kind: RuleKind) -> &'static str {
    match kind {
        RuleKind::Include => "include",
        RuleKind::Exclude => "exclude",
    }
}
