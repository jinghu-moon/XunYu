// batch_rename/natural_sort.rs
//
// Natural (human) sort comparison for file names.
// "track_2" < "track_10" instead of the lexicographic "track_10" < "track_2".

use std::cmp::Ordering;

/// Compare two strings using natural sort order.
pub fn natural_cmp(a: &str, b: &str) -> Ordering {
    let mut ai = a.char_indices().peekable();
    let mut bi = b.char_indices().peekable();

    loop {
        match (ai.peek(), bi.peek()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(&(_, ac)), Some(&(_, bc))) => {
                if ac.is_ascii_digit() && bc.is_ascii_digit() {
                    // Extract full numeric segments
                    let an = collect_digits(a, &mut ai);
                    let bn = collect_digits(b, &mut bi);
                    let ord = an.cmp(&bn);
                    if ord != Ordering::Equal {
                        return ord;
                    }
                } else {
                    let ord = ac.to_lowercase().cmp(bc.to_lowercase());
                    ai.next();
                    bi.next();
                    if ord != Ordering::Equal {
                        return ord;
                    }
                }
            }
        }
    }
}

/// Collect a contiguous run of ASCII digits as a u64.
fn collect_digits(s: &str, iter: &mut std::iter::Peekable<std::str::CharIndices>) -> u64 {
    let mut num: u64 = 0;
    while let Some(&(_, c)) = iter.peek() {
        if c.is_ascii_digit() {
            num = num.saturating_mul(10).saturating_add(c as u64 - '0' as u64);
            iter.next();
        } else {
            break;
        }
    }
    let _ = s; // s is only used for context; iteration is done via iter
    num
}
