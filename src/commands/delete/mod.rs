mod deleter;
mod file_info;
mod progress;
mod reboot_delete;
mod scanner;
mod usn_scan;
mod winapi;

#[cfg(feature = "delete_tui")]
mod tree;
#[cfg(feature = "delete_tui")]
mod tui;

mod cmd;
mod filters;
mod paths;
mod pipeline;
mod render;
mod types;

#[cfg(feature = "delete_tui")]
pub(crate) use types::{DeleteOptions, DeleteRecord};

pub(crate) use cmd::cmd_delete;

const DEFAULT_EXCLUDES: &[&str] = &[
    "node_modules",
    ".yarn",
    "jspm_packages",
    "bower_components",
    ".venv",
    "venv",
    "env",
    "__pycache__",
    ".tox",
    ".pytest_cache",
    ".mypy_cache",
    "site-packages",
    "vendor",
    ".bundle",
    "target",
    ".gradle",
    "build",
    "packages",
    ".nuget",
    ".m2",
    "pods",
    "carthage",
    ".build",
    ".pub-cache",
    ".dart_tool",
    "deps",
    "_build",
    ".stack-work",
    ".cabal",
    "elm-stuff",
    "renv",
    "packrat",
    ".terraform",
    ".next",
    ".nuxt",
    ".svelte-kit",
];
