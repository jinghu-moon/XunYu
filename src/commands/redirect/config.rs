use crate::config::{GlobalConfig, RedirectProfile, RedirectUnmatched};
use regex::Regex;

pub(crate) fn get_profile<'a>(
    cfg: &'a GlobalConfig,
    name: &str,
) -> Result<&'a RedirectProfile, String> {
    let Some(p) = cfg.redirect.profiles.get(name) else {
        return Err(format!("Redirect profile not found: {name}"));
    };
    Ok(p)
}

pub(crate) fn validate_profile(p: &RedirectProfile) -> Result<(), String> {
    if p.rules.is_empty() {
        return Err("Redirect profile rules is empty.".to_string());
    }
    // Both `unmatched` and `on_conflict` are deserialized as enums. Invalid values are rejected at
    // config parse time.
    let _ = &p.on_conflict;
    if p.recursive && p.max_depth == 0 {
        return Err("recursive=true requires max_depth >= 1".to_string());
    }
    for (idx, r) in p.rules.iter().enumerate() {
        if r.dest.trim().is_empty() {
            return Err(format!("Redirect rule[{idx}] dest is empty."));
        }
        validate_dest_template(&r.dest)
            .map_err(|e| format!("Redirect rule[{idx}] dest invalid: {e}"))?;
        let has_ext = !r.match_cond.ext.is_empty();
        let has_glob = r
            .match_cond
            .glob
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let has_regex = r
            .match_cond
            .regex
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let has_size = r
            .match_cond
            .size
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        let has_age = r
            .match_cond
            .age
            .as_deref()
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);

        if !has_ext && !has_glob && !has_regex && !has_size && !has_age {
            return Err(format!(
                "Redirect rule[{idx}] match is empty (need ext and/or glob and/or regex and/or size and/or age)."
            ));
        }

        if let Some(re) = r.match_cond.regex.as_deref()
            && !re.trim().is_empty()
        {
            Regex::new(re).map_err(|e| format!("Redirect rule[{idx}] regex invalid: {e}"))?;
        }

        if let Some(sz) = r.match_cond.size.as_deref()
            && !sz.trim().is_empty()
        {
            super::matcher::parse_size_expr(sz)
                .map_err(|e| format!("Redirect rule[{idx}] size invalid: {e}"))?;
        }

        if let Some(age) = r.match_cond.age.as_deref()
            && !age.trim().is_empty()
        {
            super::matcher::parse_age_expr(age)
                .map_err(|e| format!("Redirect rule[{idx}] age invalid: {e}"))?;
        }
    }

    if let RedirectUnmatched::Archive { age_expr, dest } = &p.unmatched {
        super::matcher::parse_age_expr(age_expr)
            .map_err(|e| format!("Redirect unmatched age invalid: {e}"))?;
        validate_dest_template(dest)
            .map_err(|e| format!("Redirect unmatched dest invalid: {e}"))?;
    }
    Ok(())
}

fn validate_dest_template(dest: &str) -> Result<(), String> {
    let allowed = ["{name}", "{ext}", "{created.year}", "{created.month}"];

    let mut i = 0usize;
    let bytes = dest.as_bytes();
    while i < bytes.len() {
        let ch = bytes[i] as char;
        if ch == '}' {
            return Err("unexpected '}'".to_string());
        }
        if ch != '{' {
            i += 1;
            continue;
        }
        let Some(end) = dest[i..].find('}') else {
            return Err("unclosed '{'".to_string());
        };
        let end_idx = i + end;
        let token = &dest[i..=end_idx];
        if !allowed.contains(&token) {
            return Err(format!("unknown template token: {token}"));
        }
        i = end_idx + 1;
    }
    Ok(())
}
