use super::common::{is_dev_port, parse_range};
use super::render::render_ports_table;
use super::*;

pub(crate) fn cmd_ports(args: PortsCmd) -> CliResult {
    let mut items = if args.udp {
        list_udp_endpoints()
    } else {
        list_tcp_listeners()
    };

    if !args.udp && !args.all {
        items.retain(|p| is_dev_port(p.port));
    }

    if let Some(ref raw) = args.range {
        let Some((start, end)) = parse_range(raw) else {
            return Err(CliError::with_details(
                2,
                format!("Invalid range: {}.", raw),
                &["Fix: Use START-END (e.g. 3000-4000)."],
            ));
        };
        items.retain(|p| p.port >= start && p.port <= end);
    }
    if let Some(pid) = args.pid {
        items.retain(|p| p.pid == pid);
    }
    if let Some(ref name) = args.name {
        let needle = name.to_lowercase();
        items.retain(|p| p.name.to_lowercase().contains(&needle));
    }

    if items.is_empty() {
        ui_println!("No ports found.");
        return Ok(());
    }

    items.sort_by(|a, b| a.port.cmp(&b.port).then(a.pid.cmp(&b.pid)));

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

    if format == ListFormat::Tsv {
        for p in items {
            out_println!("{}\t{}\t{}\t{}", p.port, p.pid, p.name, p.exe_path);
        }
        return Ok(());
    }
    if format == ListFormat::Json {
        let list: Vec<serde_json::Value> = items
            .into_iter()
            .map(|p| {
                serde_json::json!({
                    "port": p.port,
                    "pid": p.pid,
                    "name": p.name,
                    "path": p.exe_path,
                    "protocol": match p.protocol { Protocol::Tcp => "tcp", Protocol::Udp => "udp" },
                })
            })
            .collect();
        out_println!("{}", serde_json::Value::Array(list));
        return Ok(());
    }

    render_ports_table(&items);
    Ok(())
}
