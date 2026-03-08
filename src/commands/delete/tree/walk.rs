use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use regex::Regex;

use super::super::scanner;
use super::{CheckState, FileTree, NodeKind, TreeNode};

impl FileTree {
    pub(crate) fn build(
        root: &PathBuf,
        target_names: &HashSet<String>,
        match_all: bool,
        exclude_dirs: &HashSet<String>,
        patterns: &[Regex],
        cancel: &Arc<AtomicBool>,
    ) -> Self {
        let mut nodes: Vec<TreeNode> = Vec::new();

        let root_name = root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(root.to_str().unwrap_or("/"))
            .to_string();
        nodes.push(TreeNode {
            id: 0,
            path: root.clone(),
            name: root_name,
            kind: NodeKind::Dir,
            depth: 0,
            expanded: true,
            check: CheckState::Unchecked,
            children: vec![],
            parent: None,
            size: None,
            target_count: 0,
        });

        Self::populate(
            &mut nodes,
            0,
            root,
            target_names,
            match_all,
            exclude_dirs,
            patterns,
            1,
            cancel,
        );

        update_counts(&mut nodes, 0);

        let root_children: Vec<usize> = nodes[0].children.clone();
        for cid in root_children {
            if nodes[cid].target_count > 0 {
                nodes[cid].expanded = true;
            }
        }

        FileTree {
            nodes,
            cursor: 0,
            filter: String::new(),
            filter_active: false,
        }
    }

    fn populate(
        nodes: &mut Vec<TreeNode>,
        parent_id: usize,
        dir: &PathBuf,
        target_names: &HashSet<String>,
        match_all: bool,
        exclude_dirs: &HashSet<String>,
        patterns: &[Regex],
        depth: usize,
        cancel: &Arc<AtomicBool>,
    ) {
        if cancel.load(Ordering::Relaxed) {
            return;
        }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        let mut dirs: Vec<(String, PathBuf)> = Vec::new();
        let mut files: Vec<(String, PathBuf, bool, Option<u64>)> = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if scanner::matches_any(path.to_string_lossy().as_ref(), patterns) {
                continue;
            }
            if path.is_dir() {
                dirs.push((name, path));
            } else {
                let is_target = match_all || target_names.contains(&name.to_lowercase());
                let size = entry.metadata().ok().map(|m| m.len());
                files.push((name, path, is_target, size));
            }
        }

        files.sort_by(|a, b| {
            b.2.cmp(&a.2)
                .then(a.0.to_lowercase().cmp(&b.0.to_lowercase()))
        });
        dirs.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

        for (name, path, is_target, size) in files {
            let id = nodes.len();
            nodes.push(TreeNode {
                id,
                path,
                name,
                kind: if is_target {
                    NodeKind::TargetFile
                } else {
                    NodeKind::File
                },
                depth,
                expanded: false,
                check: CheckState::Unchecked,
                children: vec![],
                parent: Some(parent_id),
                size,
                target_count: if is_target { 1 } else { 0 },
            });
            nodes[parent_id].children.push(id);
        }

        for (name, path) in dirs {
            let id = nodes.len();
            let lower = name.to_lowercase();

            if exclude_dirs.contains(&lower) {
                nodes.push(TreeNode {
                    id,
                    path,
                    name,
                    kind: NodeKind::ExcludedDir,
                    depth,
                    expanded: false,
                    check: CheckState::Unchecked,
                    children: vec![],
                    parent: Some(parent_id),
                    size: None,
                    target_count: 0,
                });
                nodes[parent_id].children.push(id);
            } else {
                nodes.push(TreeNode {
                    id,
                    path: path.clone(),
                    name,
                    kind: NodeKind::Dir,
                    depth,
                    expanded: false,
                    check: CheckState::Unchecked,
                    children: vec![],
                    parent: Some(parent_id),
                    size: None,
                    target_count: 0,
                });
                nodes[parent_id].children.push(id);
                Self::populate(
                    nodes,
                    id,
                    &path,
                    target_names,
                    match_all,
                    exclude_dirs,
                    patterns,
                    depth + 1,
                    cancel,
                );
            }
        }
    }
}

pub(super) fn update_counts(nodes: &mut Vec<TreeNode>, id: usize) -> usize {
    let children: Vec<usize> = nodes[id].children.clone();
    let mut count = 0;
    for cid in children {
        count += update_counts(nodes, cid);
    }
    if nodes[id].kind == NodeKind::TargetFile {
        count += 1;
    }
    nodes[id].target_count = count;
    count
}
