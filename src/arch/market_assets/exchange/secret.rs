pub(crate) const REDACTED_SECRET: &str = "[REDACTED]";

pub(crate) fn redact_identifier(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();

    match chars.len() {
        0 => String::new(),
        1..=8 => REDACTED_SECRET.to_string(),
        9..=14 => format!(
            "{}...{}",
            chars[..3].iter().collect::<String>(),
            chars[chars.len() - 3..].iter().collect::<String>()
        ),
        _ => format!(
            "{}...{}",
            chars[..6].iter().collect::<String>(),
            chars[chars.len() - 4..].iter().collect::<String>()
        ),
    }
}

pub(crate) fn redact_secret() -> &'static str {
    REDACTED_SECRET
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_identifier_masks_short_ids() {
        assert_eq!(redact_identifier("52955084"), REDACTED_SECRET);
    }

    #[test]
    fn redact_identifier_keeps_head_and_tail_for_medium_api_keys() {
        assert_eq!(redact_identifier("apikey12345"), "api...345");
    }

    #[test]
    fn redact_identifier_keeps_head_and_tail_for_addresses() {
        assert_eq!(
            redact_identifier("0x1234567890abcdef1234567890abcdef12345678"),
            "0x1234...5678"
        );
    }
}
