pub(super) const EXCLUDE_NAMES: &[&str] = &[
    ".vscode",
    "dist",
    "node_modules",
    "public",
    "icons",
    "debug",
    "release",
    "bin",
    "obj",
    "target",
    ".git",
    ".vs",
    ".spec-workflow",
    ".trae",
    ".agent",
    "sv.ps1",
    "tree.ps1",
    "fileTree.txt",
    "test.svg",
    "gen",
    "workers",
    "README.md",
    ".gitignore",
    "script",
];

pub(super) const EXCLUDE_PATHS: &[&str] = &["src\\assets", "src/assets"];
pub(super) const EXCLUDE_EXTS: &[&str] = &[".dll", ".exe", ".obj", ".pdb", ".ilk"];

pub(super) const BRANCH_MID: &str = "\u{251c}\u{2500}\u{2500}";
pub(super) const BRANCH_END: &str = "\u{2514}\u{2500}\u{2500}";
