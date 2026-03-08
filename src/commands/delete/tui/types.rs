use std::sync::mpsc::Receiver;

use super::super::DeleteRecord;

pub(super) enum AppState {
    Loading,
    Browsing,
    Filtering,
    ConfirmDelete,
    Deleting { rx: Receiver<Vec<DeleteRecord>> },
    Done(Vec<DeleteRecord>),
}
