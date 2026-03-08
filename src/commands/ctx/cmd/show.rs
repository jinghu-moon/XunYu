use super::super::common::resolve_ctx_list_format;
use super::*;

pub(super) fn cmd_show(args: CtxShowCmd) -> CliResult {
    let name = match args.name {
        Some(n) => n,
        None => active_profile_name().ok_or_else(|| {
            CliError::with_details(
                2,
                "No active profile.".to_string(),
                &[
                    "Hint: Run `xun ctx list` to see available profiles.",
                    "Fix: Activate one with `xun ctx use <name>`.",
                ],
            )
        })?,
    };

    let path = ctx_store_path();
    let store = load_store(&path);
    let profile = store.profiles.get(&name).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Profile not found: {}", name),
            &["Hint: Run `xun ctx list` to see existing profiles."],
        )
    })?;

    let format = resolve_ctx_list_format(&args.format)?;

    if format == ListFormat::Json {
        let obj = serde_json::json!({
            "name": name,
            "path": profile.path,
            "tags": profile.tags,
            "proxy": profile.proxy,
            "env": profile.env,
        });
        out_println!(
            "{}",
            serde_json::to_string(&obj).unwrap_or_else(|_| "{}".to_string())
        );
        return Ok(());
    }

    if format == ListFormat::Tsv {
        out_println!("name\t{name}");
        out_println!("path\t{}", profile.path);
        out_println!("tags\t{}", profile.tags.join(","));
        out_println!("proxy\t{}", proxy_summary(&profile.proxy));
        if let Some(url) = &profile.proxy.url {
            out_println!("proxy_url\t{url}");
        }
        if let Some(np) = &profile.proxy.noproxy {
            out_println!("no_proxy\t{np}");
        }
        if !profile.env.is_empty() {
            for (k, v) in &profile.env {
                out_println!("env\t{}={}", k, v);
            }
        }
        return Ok(());
    }

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Key")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Value")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
    ]);
    table.add_row(vec![Cell::new("name"), Cell::new(name.as_str())]);
    table.add_row(vec![Cell::new("path"), Cell::new(profile.path.as_str())]);
    let tags_value = if profile.tags.is_empty() {
        "-".to_string()
    } else {
        profile.tags.join(",")
    };
    table.add_row(vec![Cell::new("tags"), Cell::new(tags_value)]);
    table.add_row(vec![
        Cell::new("proxy"),
        Cell::new(proxy_summary(&profile.proxy)),
    ]);
    if let Some(url) = &profile.proxy.url {
        table.add_row(vec![Cell::new("proxy_url"), Cell::new(url.as_str())]);
    }
    if let Some(np) = &profile.proxy.noproxy {
        table.add_row(vec![Cell::new("no_proxy"), Cell::new(np.as_str())]);
    }
    if profile.env.is_empty() {
        table.add_row(vec![Cell::new("env"), Cell::new("-")]);
    } else {
        for (k, v) in &profile.env {
            table.add_row(vec![Cell::new("env"), Cell::new(format!("{k}={v}"))]);
        }
    }
    print_table(&table);
    Ok(())
}
