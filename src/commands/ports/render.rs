use super::common::trunc;
use super::*;

pub(super) fn render_ports_table(items: &[PortInfo]) {
    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Port")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("PID")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Process")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
        Cell::new("Path")
            .add_attribute(Attribute::Bold)
            .fg(Color::Magenta),
    ]);

    for p in items {
        let path_cell = if p.exe_path.is_empty() {
            Cell::new("<denied>")
                .fg(Color::DarkGrey)
                .add_attribute(Attribute::Dim)
        } else {
            Cell::new(super::common::trunc(&p.exe_path, 55))
                .fg(Color::DarkGrey)
                .add_attribute(Attribute::Dim)
        };
        table.add_row(vec![
            Cell::new(p.port).fg(Color::Green),
            Cell::new(p.pid).fg(Color::Cyan),
            Cell::new(&p.name)
                .add_attribute(Attribute::Bold)
                .fg(Color::Yellow),
            path_cell,
        ]);
    }

    print_table(&table);
}
pub(super) fn render_targets_table(targets: &[PortInfo]) {
    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    table.set_header(vec![
        Cell::new("Proto")
            .add_attribute(Attribute::Bold)
            .fg(Color::DarkGrey),
        Cell::new("Port")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("PID")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Process")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
        Cell::new("Path")
            .add_attribute(Attribute::Bold)
            .fg(Color::Magenta),
    ]);

    for p in targets {
        let proto = match p.protocol {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
        };
        let path_cell = if p.exe_path.is_empty() {
            Cell::new("<denied>")
                .fg(Color::DarkGrey)
                .add_attribute(Attribute::Dim)
        } else {
            Cell::new(trunc(&p.exe_path, 55))
                .fg(Color::DarkGrey)
                .add_attribute(Attribute::Dim)
        };
        table.add_row(vec![
            Cell::new(proto).fg(Color::DarkGrey),
            Cell::new(p.port).fg(Color::Green),
            Cell::new(p.pid).fg(Color::Cyan),
            Cell::new(&p.name)
                .add_attribute(Attribute::Bold)
                .fg(Color::Yellow),
            path_cell,
        ]);
    }
    print_table(&table);
}

pub(super) fn render_processes_table(procs: &[ProcInfo], show_path: bool) {
    let has_titles = procs.iter().any(|p| !p.window_title.is_empty());
    let has_paths = show_path && procs.iter().any(|p| !p.exe_path.is_empty());

    let mut table = Table::new();
    apply_pretty_table_style(&mut table);
    let mut headers = vec![
        Cell::new("PID")
            .add_attribute(Attribute::Bold)
            .fg(Color::Green),
        Cell::new("PPID")
            .add_attribute(Attribute::Bold)
            .fg(Color::Cyan),
        Cell::new("Name")
            .add_attribute(Attribute::Bold)
            .fg(Color::Yellow),
        Cell::new("Threads")
            .add_attribute(Attribute::Bold)
            .fg(Color::DarkGrey),
    ];
    if has_titles {
        headers.push(
            Cell::new("Window")
                .add_attribute(Attribute::Bold)
                .fg(Color::Magenta),
        );
    }
    if has_paths {
        headers.push(
            Cell::new("Path")
                .add_attribute(Attribute::Bold)
                .fg(Color::DarkGrey),
        );
    }
    table.set_header(headers);

    for p in procs {
        let mut row = vec![
            Cell::new(p.pid).fg(Color::Green),
            Cell::new(p.ppid).fg(Color::Cyan),
            Cell::new(&p.name).fg(Color::Yellow),
            Cell::new(p.thread_cnt).fg(Color::DarkGrey),
        ];
        if has_titles {
            if p.window_title.is_empty() {
                row.push(
                    Cell::new("-")
                        .fg(Color::DarkGrey)
                        .add_attribute(Attribute::Dim),
                );
            } else {
                row.push(Cell::new(trunc(&p.window_title, 50)).fg(Color::Magenta));
            }
        }
        if has_paths {
            if p.exe_path.is_empty() {
                row.push(
                    Cell::new("<denied>")
                        .fg(Color::DarkGrey)
                        .add_attribute(Attribute::Dim),
                );
            } else {
                row.push(
                    Cell::new(trunc(&p.exe_path, 55))
                        .fg(Color::DarkGrey)
                        .add_attribute(Attribute::Dim),
                );
            }
        }
        table.add_row(row);
    }
    print_table(&table);
}
