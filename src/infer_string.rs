use crate::StringType;

lazy_static! {
    static ref ISO_DATE_REGEX: regex::Regex = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    static ref UUIDREGEX: regex::Regex =
        regex::Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
            .unwrap();
    static ref HOSTNAME_REGEX: regex::Regex =
        regex::Regex::new(r"^[a-zA-Z0-9\-]+\.[a-zA-Z]{2,}$").unwrap();
    static ref EMAIL_REGEX: regex::Regex =
        regex::Regex::new(r"[a-zA-Z0-9]+@[a-zA-Z0-9]+\.[a-zA-Z]{2,}$").unwrap();
}

pub(crate) fn infer_string_type(s: &str) -> StringType {
    if s.len() == 36 && UUIDREGEX.is_match(s) {
        return StringType::UUID;
    }

    if s.contains('@') && EMAIL_REGEX.is_match(s) {
        return StringType::Email;
    }

    if s.contains('.') {
        if url::Url::parse(s).is_ok() {
            return StringType::Url;
        }
        if HOSTNAME_REGEX.is_match(s) {
            return StringType::Hostname;
        }
    }

    if s.chars().take(1).all(|char| char.is_numeric()) {
        if ISO_DATE_REGEX.is_match(s) {
            return StringType::IsoDate;
        }
        if chrono::DateTime::parse_from_rfc3339(s).is_ok() {
            return StringType::DateTimeISO8601;
        }
    }

    if chrono::DateTime::parse_from_rfc2822(s).is_ok() {
        return StringType::DateTimeISO8601;
    }

    return StringType::Unknown {
        strings_seen: vec![s.to_owned()],
        chars_seen: s.chars().collect(),
        min_length: Some(s.len()),
        max_length: Some(s.len()),
    };
}
