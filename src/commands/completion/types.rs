pub(super) struct CompletionItem {
    pub(super) value: String,
    pub(super) desc: String,
}

impl CompletionItem {
    pub(super) fn new(value: String) -> Self {
        Self {
            value,
            desc: String::new(),
        }
    }
}
