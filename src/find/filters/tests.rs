use super::attr::parse_attr_filter;
use super::depth::parse_depth_filter;
use super::size::parse_size_filter;
use super::time::{days_from_civil, parse_time_filter};
use super::types::{RangeBound, SizeCompare, SizeFilter, TimeType};
use crate::store::now_secs;
use windows_sys::Win32::Storage::FileSystem::{FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_READONLY};

#[test]
fn parse_size_filter_compare_and_range() {
    let f = parse_size_filter(">=10k", false).unwrap();
    match f {
        SizeFilter::Compare { op, value } => {
            assert_eq!(op, SizeCompare::Ge);
            assert_eq!(value, 10 * 1024);
        }
        _ => panic!("expected compare"),
    }

    let f = parse_size_filter("[5k-10k)", false).unwrap();
    match f {
        SizeFilter::Range {
            min,
            max,
            left,
            right,
        } => {
            assert_eq!(min, 5 * 1024);
            assert_eq!(max, 10 * 1024);
            assert_eq!(left, RangeBound::Closed);
            assert_eq!(right, RangeBound::Open);
        }
        _ => panic!("expected range"),
    }
}

#[test]
fn parse_fuzzy_size_filter() {
    let f = parse_size_filter("3M", true).unwrap();
    match f {
        SizeFilter::Range { min, max, .. } => {
            assert_eq!(min, 2 * 1024 * 1024 + 1);
            assert_eq!(max, 3 * 1024 * 1024);
        }
        _ => panic!("expected range"),
    }

    let f = parse_size_filter("5k-10k", true).unwrap();
    match f {
        SizeFilter::Range { min, max, .. } => {
            assert_eq!(min, 4 * 1024 + 1);
            assert_eq!(max, 10 * 1024);
        }
        _ => panic!("expected range"),
    }

    assert!(parse_size_filter(">10k", true).is_err());
}

#[test]
fn parse_depth_filter_variants() {
    let f = parse_depth_filter(">=2").unwrap();
    assert_eq!(f.min, Some(2));
    assert_eq!(f.max, None);

    let f = parse_depth_filter("2-4").unwrap();
    assert_eq!(f.min, Some(2));
    assert_eq!(f.max, Some(4));

    let f = parse_depth_filter("(2-4]").unwrap();
    assert_eq!(f.min, Some(3));
    assert_eq!(f.max, Some(4));

    let f = parse_depth_filter("<3").unwrap();
    assert_eq!(f.min, None);
    assert_eq!(f.max, Some(2));
}

#[test]
fn parse_attr_filter_masks() {
    let f = parse_attr_filter("+h,-r").unwrap();
    assert_ne!(f.required & FILE_ATTRIBUTE_HIDDEN, 0);
    assert_ne!(f.forbidden & FILE_ATTRIBUTE_READONLY, 0);

    let err = parse_attr_filter("+h,-h").unwrap_err();
    assert!(err.message.contains("Attribute conflict"));
}

#[test]
fn parse_time_filter_absolute() {
    let base = days_from_civil(2024, 1, 1) * 86_400;
    let f = parse_time_filter("+2024.01.01", TimeType::Mtime).unwrap();
    assert_eq!(f.start, 0);
    assert_eq!(f.end, base);

    let f = parse_time_filter("~2024.01.01", TimeType::Mtime).unwrap();
    assert_eq!(f.start, base);
    assert_eq!(f.end, base + 86_400);

    let f = parse_time_filter("-2024.01.01", TimeType::Mtime).unwrap();
    assert_eq!(f.start, base);
    assert_eq!(f.end, -1);
}

#[test]
fn parse_time_filter_relative_and_range() {
    let before = now_secs() as i64;
    let f = parse_time_filter("-7d", TimeType::Mtime).unwrap();
    let after = now_secs() as i64;
    let expected = before - 7 * 86_400;
    assert!(f.start >= expected - 2);
    assert!(f.start <= after - 7 * 86_400 + 2);
    assert_eq!(f.end, -1);

    let before = now_secs() as i64;
    let f = parse_time_filter("10d-2d", TimeType::Mtime).unwrap();
    let after = now_secs() as i64;
    let start_expected = before - 11 * 86_400;
    let end_expected = before - 2 * 86_400;
    assert!(f.start >= start_expected - 2);
    assert!(f.start <= after - 11 * 86_400 + 2);
    assert!(f.end >= end_expected - 2);
    assert!(f.end <= after - 2 * 86_400 + 2);
}

#[test]
fn parse_time_filter_rejects_prefixed_relative_range() {
    assert!(parse_time_filter("+10d-2d", TimeType::Mtime).is_err());
}
