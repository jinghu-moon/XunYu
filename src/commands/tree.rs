#[path = "tree/build.rs"]
mod build;
#[path = "tree/clipboard.rs"]
mod clipboard;
#[path = "tree/cmd.rs"]
mod cmd;
#[path = "tree/collect.rs"]
mod collect;
#[path = "tree/constants.rs"]
mod constants;
#[path = "tree/filters.rs"]
mod filters;
#[path = "tree/format.rs"]
mod format;
#[path = "tree/stats.rs"]
mod stats;
#[path = "tree/types.rs"]
mod types;

pub(crate) use cmd::cmd_tree;

#[cfg(test)]
#[path = "tree/tests.rs"]
mod tests;
