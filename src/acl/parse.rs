/// CLI argument parsing helpers.
///
/// Extracted into their own module so integration tests can verify them
/// without having to invoke the full binary.
use anyhow::{Result, bail};

use crate::acl::types::{AceType, InheritanceFlags, RIGHTS_TABLE};

/// Resolve a rights string to a 32-bit access mask.
///
/// Accepts:
/// * Table short names: `FullControl`, `Modify`, `ReadAndExecute`, `Read`, `Write`
/// * Hex literals: `0x001F01FF`
/// * Decimal literals: `2032127`
pub fn parse_rights(s: &str) -> Result<u32> {
    for &(mask, short, _) in RIGHTS_TABLE {
        if s.eq_ignore_ascii_case(short) {
            return Ok(mask);
        }
    }
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        return u32::from_str_radix(hex, 16)
            .map_err(|_| anyhow::anyhow!("invalid hex literal: '{s}'"));
    }
    s.parse::<u32>().map_err(|_| {
        anyhow::anyhow!(
            "unknown rights value '{s}'. \
             Valid names: FullControl | Modify | ReadAndExecute | Read | Write"
        )
    })
}

/// Parse `Allow` / `Deny` (case-insensitive) to [`AceType`].
pub fn parse_ace_type(s: &str) -> Result<AceType> {
    match s.to_lowercase().as_str() {
        "allow" => Ok(AceType::Allow),
        "deny" => Ok(AceType::Deny),
        other => bail!("invalid access type '{other}'. Use: Allow | Deny"),
    }
}

/// Parse an inheritance string to [`InheritanceFlags`].
///
/// Accepted values (case-insensitive, hyphens stripped):
/// * `BothInherit` / `Both`
/// * `ContainerOnly` / `Container`
/// * `ObjectOnly` / `Object`
/// * `None` / `No` / `NoInherit`
pub fn parse_inheritance(s: &str) -> Result<InheritanceFlags> {
    match s.to_lowercase().replace('-', "").as_str() {
        "bothinherit" | "both" => Ok(InheritanceFlags::BOTH),
        "containeronly" | "container" => Ok(InheritanceFlags::CONTAINER_INHERIT),
        "objectonly" | "object" => Ok(InheritanceFlags::OBJECT_INHERIT),
        "none" | "no" | "noinherit" => Ok(InheritanceFlags::NONE),
        other => bail!(
            "invalid inheritance '{other}'. \
             Use: BothInherit | ContainerOnly | ObjectOnly | None"
        ),
    }
}

/// Truncate a string to `max` bytes, appending `…` if longer.
pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

/// Truncate a string from the left, keeping the tail and prepending `…`.
pub fn truncate_left(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("…{}", &s[s.len().saturating_sub(max - 1)..])
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acl::types::RIGHTS_TABLE;

    // ── parse_rights ──────────────────────────────────────────────────────────

    #[test]
    fn parse_rights_all_table_entries() {
        for &(mask, short, _) in RIGHTS_TABLE {
            assert_eq!(
                parse_rights(short).unwrap(),
                mask,
                "parse_rights('{short}') should return {mask:#010x}"
            );
        }
    }

    #[test]
    fn parse_rights_case_insensitive() {
        assert_eq!(parse_rights("fullcontrol").unwrap(), 0x1F01FF);
        assert_eq!(parse_rights("FULLCONTROL").unwrap(), 0x1F01FF);
        assert_eq!(parse_rights("FullControl").unwrap(), 0x1F01FF);
    }

    #[test]
    fn parse_rights_hex_lowercase() {
        assert_eq!(parse_rights("0x1f01ff").unwrap(), 0x1F01FF);
    }

    #[test]
    fn parse_rights_hex_uppercase() {
        assert_eq!(parse_rights("0x1F01FF").unwrap(), 0x1F01FF);
    }

    #[test]
    fn parse_rights_hex_uppercase_prefix() {
        assert_eq!(parse_rights("0X001f01ff").unwrap(), 0x1F01FF);
    }

    #[test]
    fn parse_rights_decimal() {
        assert_eq!(parse_rights("2032127").unwrap(), 2_032_127);
    }

    #[test]
    fn parse_rights_zero() {
        assert_eq!(parse_rights("0x0").unwrap(), 0);
        assert_eq!(parse_rights("0").unwrap(), 0);
    }

    #[test]
    fn parse_rights_unknown_string_errors() {
        assert!(parse_rights("SuperControl").is_err());
    }

    #[test]
    fn parse_rights_invalid_hex_errors() {
        assert!(parse_rights("0xGHIJ").is_err());
    }

    #[test]
    fn parse_rights_empty_string_errors() {
        assert!(parse_rights("").is_err());
    }

    // ── parse_ace_type ────────────────────────────────────────────────────────

    #[test]
    fn parse_ace_type_allow_variants() {
        assert_eq!(parse_ace_type("Allow").unwrap(), AceType::Allow);
        assert_eq!(parse_ace_type("allow").unwrap(), AceType::Allow);
        assert_eq!(parse_ace_type("ALLOW").unwrap(), AceType::Allow);
    }

    #[test]
    fn parse_ace_type_deny_variants() {
        assert_eq!(parse_ace_type("Deny").unwrap(), AceType::Deny);
        assert_eq!(parse_ace_type("deny").unwrap(), AceType::Deny);
        assert_eq!(parse_ace_type("DENY").unwrap(), AceType::Deny);
    }

    #[test]
    fn parse_ace_type_invalid_errors() {
        assert!(parse_ace_type("Grant").is_err());
        assert!(parse_ace_type("").is_err());
        assert!(parse_ace_type("PERMIT").is_err());
    }

    // ── parse_inheritance ─────────────────────────────────────────────────────

    #[test]
    fn parse_inheritance_both_variants() {
        assert_eq!(
            parse_inheritance("BothInherit").unwrap(),
            InheritanceFlags::BOTH
        );
        assert_eq!(parse_inheritance("both").unwrap(), InheritanceFlags::BOTH);
        assert_eq!(parse_inheritance("BOTH").unwrap(), InheritanceFlags::BOTH);
    }

    #[test]
    fn parse_inheritance_container_variants() {
        assert_eq!(
            parse_inheritance("ContainerOnly").unwrap(),
            InheritanceFlags::CONTAINER_INHERIT
        );
        assert_eq!(
            parse_inheritance("container").unwrap(),
            InheritanceFlags::CONTAINER_INHERIT
        );
    }

    #[test]
    fn parse_inheritance_object_variants() {
        assert_eq!(
            parse_inheritance("ObjectOnly").unwrap(),
            InheritanceFlags::OBJECT_INHERIT
        );
        assert_eq!(
            parse_inheritance("object").unwrap(),
            InheritanceFlags::OBJECT_INHERIT
        );
    }

    #[test]
    fn parse_inheritance_none_variants() {
        assert_eq!(parse_inheritance("None").unwrap(), InheritanceFlags::NONE);
        assert_eq!(parse_inheritance("No").unwrap(), InheritanceFlags::NONE);
        assert_eq!(
            parse_inheritance("NoInherit").unwrap(),
            InheritanceFlags::NONE
        );
        assert_eq!(parse_inheritance("none").unwrap(), InheritanceFlags::NONE);
    }

    #[test]
    fn parse_inheritance_strips_hyphens() {
        // "No-Inherit" → "noinherit"
        assert_eq!(
            parse_inheritance("No-Inherit").unwrap(),
            InheritanceFlags::NONE
        );
        assert_eq!(
            parse_inheritance("Both-Inherit").unwrap(),
            InheritanceFlags::BOTH
        );
    }

    #[test]
    fn parse_inheritance_invalid_errors() {
        assert!(parse_inheritance("Recursive").is_err());
        assert!(parse_inheritance("").is_err());
    }

    // ── truncate ──────────────────────────────────────────────────────────────

    #[test]
    fn truncate_short_string_unchanged() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_exact_length_unchanged() {
        let s = "12345";
        assert_eq!(truncate(s, 5), "12345");
    }

    #[test]
    fn truncate_long_string_ellipsis() {
        let result = truncate("hello world", 6);
        assert!(result.ends_with('…'));
        // byte length ≤ max + 3 (UTF-8 ellipsis is 3 bytes)
        assert!(result.len() <= 6 + 3);
    }

    #[test]
    fn truncate_max_zero_gives_ellipsis() {
        let result = truncate("any", 0);
        // saturating_sub(1) → 0, so prefix is "", then "…"
        assert_eq!(result, "…");
    }

    // ── truncate_left ─────────────────────────────────────────────────────────

    #[test]
    fn truncate_left_short_unchanged() {
        assert_eq!(truncate_left("hello", 10), "hello");
    }

    #[test]
    fn truncate_left_long_keeps_tail() {
        let result = truncate_left("C:\\very\\long\\path\\file.txt", 10);
        assert!(result.starts_with('…'));
        assert!(result.ends_with("file.txt"));
    }
}
