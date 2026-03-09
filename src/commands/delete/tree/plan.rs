use super::{FileTree, NodeKind};

impl FileTree {
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
                if (node.kind == NodeKind::Dir || node.kind == NodeKind::File)
                    && !self.subtree_has_match(id, &q)
                {
                    return;
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
        if let Some(&id) = visible.get(self.cursor)
            && self.nodes[id].kind == NodeKind::Dir
        {
            self.nodes[id].expanded = true;
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
        if let Some(&id) = visible.get(self.cursor)
            && self.nodes[id].kind == NodeKind::Dir
        {
            self.nodes[id].expanded = !self.nodes[id].expanded;
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
}
