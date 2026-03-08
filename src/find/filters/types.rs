#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EmptyFilterMode {
    None,
    Only,
    Exclude,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SizeCompare {
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RangeBound {
    Open,
    Closed,
}

#[derive(Clone, Debug)]
pub(crate) enum SizeFilter {
    Compare {
        op: SizeCompare,
        value: u64,
    },
    Range {
        min: u64,
        max: u64,
        left: RangeBound,
        right: RangeBound,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TimeType {
    Mtime,
    Ctime,
    Atime,
}

#[derive(Clone, Debug)]
pub(crate) struct TimeFilter {
    pub(crate) kind: TimeType,
    pub(crate) start: i64,
    pub(crate) end: i64,
}

#[derive(Clone, Debug)]
pub(crate) struct DepthFilter {
    pub(crate) min: Option<i32>,
    pub(crate) max: Option<i32>,
}

#[derive(Clone, Debug)]
pub(crate) struct AttrFilter {
    pub(crate) required: u32,
    pub(crate) forbidden: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct FindFilters {
    pub(crate) size_filters: Vec<SizeFilter>,
    pub(crate) time_filters: Vec<TimeFilter>,
    pub(crate) depth: Option<DepthFilter>,
    pub(crate) attr: Option<AttrFilter>,
    pub(crate) empty_files: EmptyFilterMode,
    pub(crate) empty_dirs: EmptyFilterMode,
}
