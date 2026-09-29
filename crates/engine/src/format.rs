//! The pinyin table naming contract (`contracts/dictionary/format.json`, version 1). The dictionary builder and the runtime must agree on it, so it lives in one place.

pub const FORMAT_VERSION: i32 = 1;
/// Tables are numbered by syllable count up to this; longer keys go to the overflow bucket.
pub const MAXIMUM_NUMBERED_SYLLABLES: usize = 7;
/// First letters that have tables. `i`, `u` and `v` start no syllable, so they have none.
pub const SHIPPED_INITIALS: &str = "abcdefghjklmnopqrstwxyz";
pub const TABLE_PREFIX: &str = "tbl_";
pub const OVERFLOW_BUCKET: &str = "others";

/// `tbl_<n>_<initial>`, or `tbl_others_<initial>` past seven syllables. `None` for no syllables or an initial outside `a..=z`. A shipped-set miss (`i`, `u`, `v`) still gets a name; the table simply does not exist, and readers treat that as no rows.
pub fn quanpin_table(syllables: usize, initial: u8) -> Option<String> {
    if syllables == 0 || !initial.is_ascii_lowercase() {
        return None;
    }
    let bucket = if syllables <= MAXIMUM_NUMBERED_SYLLABLES {
        syllables.to_string()
    } else {
        OVERFLOW_BUCKET.to_owned()
    };
    Some(format!("{TABLE_PREFIX}{bucket}_{}", initial as char))
}

/// The table for a segmented key: by segment count and the first letter of the first segment.
pub fn build_table_name(segments: &[String]) -> Option<String> {
    let first = segments.first()?.as_bytes().first()?;
    quanpin_table(segments.len(), *first)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_follow_the_contract() {
        assert_eq!(quanpin_table(2, b'n').as_deref(), Some("tbl_2_n"));
        assert_eq!(quanpin_table(7, b'a').as_deref(), Some("tbl_7_a"));
        assert_eq!(quanpin_table(8, b'z').as_deref(), Some("tbl_others_z"));
        assert_eq!(quanpin_table(0, b'a'), None);
        assert_eq!(quanpin_table(1, b'A'), None);
        assert_eq!(
            build_table_name(&["ni".into(), "hao".into()]).as_deref(),
            Some("tbl_2_n")
        );
        assert_eq!(build_table_name(&[]), None);
        assert_eq!(build_table_name(&[String::new()]), None);
    }
}
