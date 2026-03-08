use crate::cli::DeleteCmd;
use crate::commands::bookmarks;
use crate::output::{CliError, CliResult};

pub(super) fn cmd_delete_bookmark(args: DeleteCmd) -> CliResult {
    if args.paths.is_empty() {
        return Err(CliError::with_details(
            2,
            "No bookmark names provided.".to_string(),
            &["Fix: Use `xun delete -bm <name>`."],
        ));
    }
    for name in args.paths {
        bookmarks::delete_bookmark(&name, args.yes)?;
    }
    Ok(())
}
