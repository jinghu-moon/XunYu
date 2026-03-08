#[cfg(feature = "redirect")]
use super::model::{RedirectOnConflict, RedirectUnmatched};

#[cfg(feature = "redirect")]
impl TryFrom<String> for RedirectOnConflict {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let raw = value.trim();
        if raw.is_empty() {
            return Ok(Self::default());
        }
        let v = raw.to_ascii_lowercase();
        match v.as_str() {
            "rename_new" => Ok(Self::RenameNew),
            "rename_date" => Ok(Self::RenameDate),
            "rename_existing" => Ok(Self::RenameExisting),
            "hash_dedup" => Ok(Self::HashDedup),
            "skip" => Ok(Self::Skip),
            "overwrite" => Ok(Self::Overwrite),
            "trash" => Ok(Self::Trash),
            "ask" => Ok(Self::Ask),
            _ => {
                let opts = [
                    "rename_new",
                    "rename_date",
                    "rename_existing",
                    "hash_dedup",
                    "skip",
                    "overwrite",
                    "trash",
                    "ask",
                ];
                let mut msg = format!("Unsupported on_conflict value: {raw}.");
                if let Some(s) = crate::suggest::did_you_mean(raw, &opts) {
                    msg.push_str(&format!(" Did you mean: \"{s}\"?"));
                }
                msg.push_str(" Valid options: rename_new | rename_date | rename_existing | hash_dedup | skip | overwrite | trash | ask");
                Err(msg)
            }
        }
    }
}

#[cfg(feature = "redirect")]
impl From<RedirectOnConflict> for String {
    fn from(value: RedirectOnConflict) -> Self {
        value.as_str().to_string()
    }
}

#[cfg(feature = "redirect")]
impl std::fmt::Display for RedirectOnConflict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(feature = "redirect")]
impl TryFrom<String> for RedirectUnmatched {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let raw = value.trim();
        if raw.is_empty() {
            return Ok(Self::Skip);
        }
        if raw.eq_ignore_ascii_case("skip") {
            return Ok(Self::Skip);
        }
        let rest = raw
            .strip_prefix("archive:")
            .or_else(|| raw.strip_prefix("ARCHIVE:"))
            .ok_or_else(|| format!("Unsupported unmatched action: {raw}"))?;
        let mut parts = rest.splitn(2, ':');
        let age_expr = parts
            .next()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("Invalid unmatched archive action: {raw}"))?
            .to_string();
        let dest = parts
            .next()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("Invalid unmatched archive action: {raw}"))?
            .to_string();
        Ok(Self::Archive { age_expr, dest })
    }
}

#[cfg(feature = "redirect")]
impl From<RedirectUnmatched> for String {
    fn from(value: RedirectUnmatched) -> Self {
        match value {
            RedirectUnmatched::Skip => "skip".to_string(),
            RedirectUnmatched::Archive { age_expr, dest } => format!("archive:{age_expr}:{dest}"),
        }
    }
}

#[cfg(feature = "redirect")]
impl std::fmt::Display for RedirectUnmatched {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_raw_string())
    }
}
