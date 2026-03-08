use super::FileTree;

impl FileTree {
    pub(crate) fn cursor_node_id(&self) -> Option<usize> {
        self.visible_nodes().get(self.cursor).copied()
    }
}
