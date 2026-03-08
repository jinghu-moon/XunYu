use std::path::PathBuf;

use super::{CheckState, FileTree, NodeKind};

impl FileTree {
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
        super::walk::update_counts(&mut self.nodes, 0);
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
}
