mod item;

use crate::config::RedirectProfile;

use super::super::plan::PlanItem;
use super::types::{RedirectOptions, RedirectResult};

pub(crate) fn apply_plan(
    tx: &str,
    profile: &RedirectProfile,
    opts: &RedirectOptions,
    items: &[PlanItem],
) -> Vec<RedirectResult> {
    let mut out = Vec::new();
    for it in items {
        item::apply_plan_item(tx, profile, opts, it, &mut out);
    }
    out
}
