use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn now_secs() -> u64 {
    if let Ok(v) = env::var("XUN_TEST_NOW_SECS")
        && let Ok(parsed) = v.parse::<u64>()
    {
        return parsed;
    }
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn now_secs_is_reasonable() {
        let n = now_secs();
        assert!(n > 1_000_000_000);
        let sys = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let diff = sys.saturating_sub(n).max(n.saturating_sub(sys));
        assert!(diff <= 5, "unexpected now_secs drift: {diff}s");
    }
}
