use std::collections::HashSet;
use std::path::PathBuf;

use regex::Regex;

use super::scanner;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NodeKind {
    Dir,
    File,
    TargetFile,
    ExcludedDir,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CheckState {
    Checked,
    Unchecked,
    Indeterminate,
}

#[derive(Debug, Clone)]
pub(crate) struct TreeNode {
    pub(crate) id: usize,
    pub(crate) path: PathBuf,
    pub(crate) name: String,
    pub(crate) kind: NodeKind,
    pub(crate) depth: usize,
    pub(crate) expanded: bool,
    pub(crate) check: CheckState,
    pub(crate) children: Vec<usize>,
    pub(crate) parent: Option<usize>,
    pub(crate) size: Option<u64>,
    pub(crate) target_count: usize,
}

impl TreeNode {}

pub(crate) struct FileTree {
    pub(crate) nodes: Vec<TreeNode>,
    pub(crate) cursor: usize,
    pub(crate) filter: String,
    pub(crate) filter_active: bool,
}

impl FileTree {
    pub(crate) fn build(
        root: &PathBuf,
        target_names: &HashSet<String>,
        match_all: bool,
        exclude_dirs: &HashSet<String>,
        patterns: &[Regex],
        cancel: &std::sync::Arc<std::sync::atomic::AtomicBool>,
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
        cancel: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) {
        use std::sync::atomic::Ordering;
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

    pub(crate) fn visible_nodes(&self) -> Vec<usize> {
        let mut result = Vec::new();
        self.collect_visible(0, &mut result);
        result
    }

    fn collect_visible(&self, id: usize, out: &mut Vec<usize>) {
        let node = &self.nodes[id];
        let is_root = id == 0;
        if !is_root {
            if self.filter_active && !self.filter.is_empty() {
                let q = self.filter.to_lowercase();
                if node.kind == NodeKind::TargetFile && !node.name.to_lowercase().contains(&q) {
                    return;
                }
                if node.kind == NodeKind::Dir || node.kind == NodeKind::File {
                    if !self.subtree_has_match(id, &q) {
                        return;
                    }
                }
            }
            out.push(id);
        }

        if (is_root || node.expanded) && node.kind == NodeKind::Dir {
            for &cid in &node.children {
                self.collect_visible(cid, out);
            }
        }
    }

    fn subtree_has_match(&self, id: usize, q: &str) -> bool {
        let node = &self.nodes[id];
        if node.kind == NodeKind::TargetFile && node.name.to_lowercase().contains(q) {
            return true;
        }
        node.children
            .iter()
            .any(|&cid| self.subtree_has_match(cid, q))
    }

    pub(crate) fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub(crate) fn move_down(&mut self) {
        let len = self.visible_nodes().len();
        if self.cursor + 1 < len {
            self.cursor += 1;
        }
    }

    pub(crate) fn move_page_up(&mut self, page: usize) {
        self.cursor = self.cursor.saturating_sub(page);
    }

    pub(crate) fn move_page_down(&mut self, page: usize) {
        let len = self.visible_nodes().len();
        self.cursor = (self.cursor + page).min(len.saturating_sub(1));
    }

    pub(crate) fn jump_top(&mut self) {
        self.cursor = 0;
    }

    pub(crate) fn jump_bottom(&mut self) {
        self.cursor = self.visible_nodes().len().saturating_sub(1);
    }

    pub(crate) fn expand_cursor(&mut self) {
        let visible = self.visible_nodes();
        if let Some(&id) = visible.get(self.cursor) {
            if self.nodes[id].kind == NodeKind::Dir {
                self.nodes[id].expanded = true;
            }
        }
    }

    pub(crate) fn collapse_cursor(&mut self) {
        let visible = self.visible_nodes();
        if let Some(&id) = visible.get(self.cursor) {
            let node = &self.nodes[id];
            if node.kind == NodeKind::Dir && node.expanded {
                self.nodes[id].expanded = false;
            } else if let Some(parent) = node.parent {
                let parent_pos = visible.iter().position(|&x| x == parent);
                if let Some(pos) = parent_pos {
                    self.cursor = pos;
                    self.nodes[parent].expanded = false;
                }
            }
        }
    }

    pub(crate) fn toggle_expand_cursor(&mut self) {
        let visible = self.visible_nodes();
        if let Some(&id) = visible.get(self.cursor) {
            if self.nodes[id].kind == NodeKind::Dir {
                self.nodes[id].expanded = !self.nodes[id].expanded;
            }
        }
    }

    pub(crate) fn expand_all(&mut self) {
        for node in &mut self.nodes {
            if node.kind == NodeKind::Dir {
                node.expanded = true;
            }
        }
    }

    pub(crate) fn collapse_all(&mut self) {
        for node in &mut self.nodes {
            if node.kind == NodeKind::Dir && node.id != 0 {
                node.expanded = false;
            }
        }
    }

    pub(crate) fn toggle_check_cursor(&mut self) {
        let visible = self.visible_nodes();
        if let Some(&id) = visible.get(self.cursor) {
            match self.nodes[id].kind {
                NodeKind::TargetFile => {
                    let new = if self.nodes[id].check == CheckState::Checked {
                        CheckState::Unchecked
                    } else {
                        CheckState::Checked
                    };
                    self.nodes[id].check = new;
                    if let Some(pid) = self.nodes[id].parent {
                        self.update_dir_check(pid);
                    }
                }
                NodeKind::Dir => {
                    let new_checked = self.nodes[id].check != CheckState::Checked;
                    self.set_subtree_check(id, new_checked);
                    if let Some(pid) = self.nodes[id].parent {
                        self.update_dir_check(pid);
                    }
                }
                _ => {}
            }
        }
    }

    fn set_subtree_check(&mut self, id: usize, checked: bool) {
        let children: Vec<usize> = self.nodes[id].children.clone();
        for cid in children {
            self.set_subtree_check(cid, checked);
        }
        if self.nodes[id].kind == NodeKind::TargetFile {
            self.nodes[id].check = if checked {
                CheckState::Checked
            } else {
                CheckState::Unchecked
            };
        }
        self.update_dir_check_local(id);
    }

    fn update_dir_check(&mut self, id: usize) {
        self.update_dir_check_local(id);
        if let Some(pid) = self.nodes[id].parent {
            self.update_dir_check(pid);
        }
    }

    fn update_dir_check_local(&mut self, id: usize) {
        if self.nodes[id].kind != NodeKind::Dir {
            return;
        }
        let children: Vec<usize> = self.nodes[id].children.clone();
        let total = children
            .iter()
            .filter(|&&c| self.nodes[c].kind == NodeKind::TargetFile)
            .count();
        let checked = children
            .iter()
            .filter(|&&c| {
                self.nodes[c].kind == NodeKind::TargetFile
                    && self.nodes[c].check == CheckState::Checked
            })
            .count();

        self.nodes[id].check = if total == 0 || checked == 0 {
            CheckState::Unchecked
        } else if checked == total {
            CheckState::Checked
        } else {
            CheckState::Indeterminate
        };
    }

    pub(crate) fn check_all_targets(&mut self) {
        for node in &mut self.nodes {
            if node.kind == NodeKind::TargetFile {
                node.check = CheckState::Checked;
            }
        }
        update_counts(&mut self.nodes, 0);
        self.propagate_all_dir_checks();
    }

    pub(crate) fn uncheck_all(&mut self) {
        for node in &mut self.nodes {
            if node.kind == NodeKind::TargetFile {
                node.check = CheckState::Unchecked;
            }
            if node.kind == NodeKind::Dir {
                node.check = CheckState::Unchecked;
            }
        }
    }

    fn propagate_all_dir_checks(&mut self) {
        let ids: Vec<usize> = (0..self.nodes.len()).rev().collect();
        for id in ids {
            self.update_dir_check_local(id);
        }
    }

    pub(crate) fn checked_paths(&self) -> Vec<PathBuf> {
        self.nodes
            .iter()
            .filter(|n| n.kind == NodeKind::TargetFile && n.check == CheckState::Checked)
            .map(|n| n.path.clone())
            .collect()
    }

    pub(crate) fn stats(&self) -> (usize, usize) {
        let total = self
            .nodes
            .iter()
            .filter(|n| n.kind == NodeKind::TargetFile)
            .count();
        let checked = self
            .nodes
            .iter()
            .filter(|n| n.kind == NodeKind::TargetFile && n.check == CheckState::Checked)
            .count();
        (total, checked)
    }

    pub(crate) fn cursor_node_id(&self) -> Option<usize> {
        self.visible_nodes().get(self.cursor).copied()
    }
}

fn update_counts(nodes: &mut Vec<TreeNode>, id: usize) -> usize {
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
