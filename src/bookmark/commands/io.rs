use std::fs;

use crate::bookmark::storage::db_path;
use crate::bookmark_state::Store;
use crate::cli::ExportCmd;
use crate::model::{IoFormat, ListItem, parse_io_format};
use crate::output::{CliError, CliResult};

pub(crate) fn cmd_export(args: ExportCmd) -> CliResult {
    let format = parse_io_format(&args.format).ok_or_else(|| {
        CliError::with_details(
            2,
            format!("Invalid format: {}.", args.format),
            &["Fix: Use one of: json | tsv"],
        )
    })?;

    let file = db_path();
    let store =
        Store::load_or_default(&file).map_err(|e| CliError::new(1, format!("Failed to load store: {e}")))?;
    let mut items: Vec<ListItem> = store
        .bookmarks
        .iter()
        .filter_map(|bookmark| {
            Some(ListItem {
                name: bookmark.name.clone()?,
                path: bookmark.path.clone(),
                tags: bookmark.tags.clone(),
                visits: bookmark.visit_count.unwrap_or(0),
                last_visited: bookmark.last_visited.unwrap_or(0),
                workspace: bookmark.workspace.clone(),
            })
        })
        .collect();

    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let output = match format {
        IoFormat::Json => serde_json::to_string_pretty(&items).unwrap_or_default(),
        IoFormat::Tsv => {
            let mut s = String::new();
            for i in items {
                s.push_str(&format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\n",
                    i.name,
                    i.path,
                    i.tags.join(","),
                    i.visits,
                    i.last_visited,
                    i.workspace.unwrap_or_default()
                ));
            }
            s
        }
    };

    if let Some(path) = args.out {
        fs::write(&path, output).map_err(|e| CliError::new(1, format!("export failed: {e}")))?;
    } else {
        out_println!("{}", output.trim_end());
    }
    Ok(())
}
