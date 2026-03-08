use crate::cli::FindCmd;
use crate::output::{CliError, CliResult};

use super::attr::parse_attr_filter;
use super::depth::parse_depth_filter;
use super::size::parse_size_filter;
use super::time::parse_time_filter;
use super::types::{EmptyFilterMode, FindFilters, TimeType};

pub(crate) fn compile_filters(args: &FindCmd) -> CliResult<FindFilters> {
    let empty_files = parse_empty_filter(args.empty_files, args.not_empty_files, "files")?;
    let empty_dirs = parse_empty_filter(args.empty_dirs, args.not_empty_dirs, "dirs")?;

    let mut size_filters = Vec::new();
    for raw in &args.size {
        let filter = parse_size_filter(raw, false)
            .map_err(|e| CliError::new(2, format!("Invalid --size: {e}")))?;
        size_filters.push(filter);
    }
    if let Some(raw) = args.fuzzy_size.as_deref() {
        let filter = parse_size_filter(raw, true)
            .map_err(|e| CliError::new(2, format!("Invalid --fuzzy-size: {e}")))?;
        size_filters.push(filter);
    }

    let mut time_filters = Vec::new();
    for raw in &args.mtime {
        time_filters.push(parse_time_filter(raw, TimeType::Mtime)?);
    }
    for raw in &args.ctime {
        time_filters.push(parse_time_filter(raw, TimeType::Ctime)?);
    }
    for raw in &args.atime {
        time_filters.push(parse_time_filter(raw, TimeType::Atime)?);
    }

    let depth = match args.depth.as_deref() {
        Some(raw) => Some(
            parse_depth_filter(raw)
                .map_err(|e| CliError::new(2, format!("Invalid --depth: {e}")))?,
        ),
        None => None,
    };

    let attr = match args.attribute.as_deref() {
        Some(raw) => Some(parse_attr_filter(raw)?),
        None => None,
    };

    Ok(FindFilters {
        size_filters,
        time_filters,
        depth,
        attr,
        empty_files,
        empty_dirs,
    })
}

fn parse_empty_filter(only: bool, exclude: bool, label: &str) -> CliResult<EmptyFilterMode> {
    match (only, exclude) {
        (true, true) => Err(CliError::new(
            2,
            format!("Conflicting empty filter for {label}."),
        )),
        (true, false) => Ok(EmptyFilterMode::Only),
        (false, true) => Ok(EmptyFilterMode::Exclude),
        (false, false) => Ok(EmptyFilterMode::None),
    }
}
