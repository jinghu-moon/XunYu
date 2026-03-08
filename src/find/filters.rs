#[path = "filters/attr.rs"]
mod attr;
#[path = "filters/compile.rs"]
mod compile;
#[path = "filters/depth.rs"]
mod depth;
#[path = "filters/matchers.rs"]
mod matchers;
#[path = "filters/size.rs"]
mod size;
#[path = "filters/time.rs"]
mod time;
#[path = "filters/types.rs"]
mod types;

pub(crate) use types::{EmptyFilterMode, FindFilters};

pub(crate) use compile::compile_filters;
pub(crate) use matchers::{
    attr_filter_match, depth_filter_match, needs_metadata_for_entry, size_filters_match,
    time_filters_match,
};
pub(crate) use time::system_time_to_secs;

#[cfg(test)]
#[path = "filters/tests.rs"]
mod tests;
