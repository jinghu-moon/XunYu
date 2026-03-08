use std::path::Path;

use comfy_table::{Attribute, Cell, Color, Table};
use console::Term;
use dialoguer::{Confirm, theme::ColorfulTheme};

use crate::cli::GcCmd;
use crate::model::ListFormat;
use crate::output::{CliError, CliResult};
use crate::output::{apply_pretty_table_style, can_interact, print_table};
use crate::store::{Lock, db_path, load, save_db};

use super::report::resolve_output_format;

pub(crate) fn cmd_gc(args: GcCmd) -> CliResult {
    let file = db_path();
    let _lock = Lock::acquire(&file.with_extension("lock"))
        .map_err(|e| CliError::new(1, format!("Failed to acquire db lock: {e}")))?;
    let mut db = load(&file);

    let dead: Vec<String> = db
        .iter()
        .filter(|(_, e)| !Path::new(&e.path).exists())
        .map(|(k, _)| k.clone())
        .collect();

    if dead.is_empty() {
        ui_println!("No dead links found.");
        return Ok(());
    }

    let format = resolve_output_format(&args.format)?;

    if args.purge {
        for k in &dead {
            db.remove(k);
        }
        save_db(&file, &db).map_err(|e| CliError::new(1, format!("Failed to save db: {e}")))?;
        ui_println!("Purged {} dead bookmarks.", dead.len());
        return Ok(());
    } else {
        if format == ListFormat::Tsv {
            for k in &dead {
                out_println!("{}\t{}", k, db[k].path);
            }
            return Ok(());
        }
        if format == ListFormat::Json {
            let items: Vec<serde_json::Value> = dead
                .iter()
                .map(|k| serde_json::json!({ "name": k, "path": db[k].path }))
                .collect();
            out_println!("{}", serde_json::Value::Array(items));
            return Ok(());
        }

        let mut table = Table::new();
        apply_pretty_table_style(&mut table);
        table.set_header(vec![
            Cell::new("Dead Bookmark")
                .add_attribute(Attribute::Bold)
                .fg(Color::Red),
            Cell::new("Missing Path")
                .add_attribute(Attribute::Bold)
                .fg(Color::Magenta),
        ]);
        for k in &dead {
            table.add_row(vec![
                Cell::new(k).fg(Color::Red).add_attribute(Attribute::Bold),
                Cell::new(&db[k].path)
                    .fg(Color::DarkGrey)
                    .add_attribute(Attribute::Dim),
            ]);
        }
        print_table(&table);

        if can_interact() {
            let ans = Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Delete all dead bookmarks?")
                .default(false)
                .interact_on(&Term::stderr());

            if matches!(ans, Ok(true)) {
                for k in &dead {
                    db.remove(k);
                }
                save_db(&file, &db)
                    .map_err(|e| CliError::new(1, format!("Failed to save db: {e}")))?;
                ui_println!("Purged.");
            } else {
                return Err(CliError::new(3, "Cancelled."));
            }
        }
    }

    Ok(())
}
