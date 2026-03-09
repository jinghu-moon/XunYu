use super::types::{
    AttrFilter, DepthFilter, EmptyFilterMode, FindFilters, RangeBound, SizeCompare, SizeFilter,
    TimeFilter, TimeType,
};

pub(crate) fn needs_metadata_for_entry(filters: &FindFilters, is_dir: bool) -> bool {
    if filters.attr.is_some() || !filters.time_filters.is_empty() {
        return true;
    }
    if !is_dir {
        if !filters.size_filters.is_empty() {
            return true;
        }
        if filters.empty_files != EmptyFilterMode::None {
            return true;
        }
    }
    false
}

pub(crate) fn size_filters_match(filters: &[SizeFilter], size: u64) -> bool {
    if filters.is_empty() {
        return true;
    }
    filters.iter().any(|f| size_filter_match(f, size))
}

pub(crate) fn depth_filter_match(filter: Option<&DepthFilter>, depth: i32) -> bool {
    let Some(filter) = filter else {
        return true;
    };
    if let Some(min) = filter.min
        && depth < min
    {
        return false;
    }
    if let Some(max) = filter.max
        && depth > max
    {
        return false;
    }
    true
}

pub(crate) fn attr_filter_match(filter: Option<&AttrFilter>, attrs: u32) -> bool {
    let Some(filter) = filter else {
        return true;
    };
    if (attrs & filter.required) != filter.required {
        return false;
    }
    if (attrs & filter.forbidden) != 0 {
        return false;
    }
    true
}

pub(crate) fn time_filters_match(
    filters: &[TimeFilter],
    mtime: Option<i64>,
    ctime: Option<i64>,
    atime: Option<i64>,
) -> bool {
    if filters.is_empty() {
        return true;
    }
    for f in filters {
        let value = match f.kind {
            TimeType::Mtime => mtime,
            TimeType::Ctime => ctime,
            TimeType::Atime => atime,
        };
        let Some(v) = value else {
            return false;
        };
        if !time_filter_match(f, v) {
            return false;
        }
    }
    true
}

fn size_filter_match(filter: &SizeFilter, size: u64) -> bool {
    match filter {
        SizeFilter::Compare { op, value } => match op {
            SizeCompare::Lt => size < *value,
            SizeCompare::Le => size <= *value,
            SizeCompare::Gt => size > *value,
            SizeCompare::Ge => size >= *value,
            SizeCompare::Eq => size == *value,
        },
        SizeFilter::Range {
            min,
            max,
            left,
            right,
        } => {
            let left_ok = match left {
                RangeBound::Closed => size >= *min,
                RangeBound::Open => size > *min,
            };
            let right_ok = match right {
                RangeBound::Closed => size <= *max,
                RangeBound::Open => size < *max,
            };
            left_ok && right_ok
        }
    }
}

fn time_filter_match(filter: &TimeFilter, value: i64) -> bool {
    if filter.end == -1 {
        return value >= filter.start;
    }
    value >= filter.start && value <= filter.end
}
