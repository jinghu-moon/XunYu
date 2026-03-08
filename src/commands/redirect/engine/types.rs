use crate::model::ListFormat;

#[derive(Clone, Copy)]
pub(crate) struct RedirectOptions {
    pub(crate) dry_run: bool,
    pub(crate) copy: bool,
    pub(crate) yes: bool,
    pub(crate) format: ListFormat,
    pub(crate) audit: bool,
}

#[derive(Clone)]
pub(crate) struct RedirectResult {
    pub(crate) action: String,
    pub(crate) src: String,
    pub(crate) dst: String,
    pub(crate) rule: String,
    pub(crate) result: String,
    pub(crate) reason: String,
}
