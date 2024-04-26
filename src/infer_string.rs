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
    if ISO_DATE_REGEX.is_match(s) {
        StringType::IsoDate
    } else if let Ok(_) = chrono::DateTime::parse_from_rfc2822(s) {
        StringType::DateTimeISO8601
    } else if let Ok(_) = chrono::DateTime::parse_from_rfc3339(s) {
        StringType::DateTimeISO8601
    } else if UUIDREGEX.is_match(s) {
        StringType::UUID
    } else if EMAIL_REGEX.is_match(s) {
        StringType::Email
    } else if let Ok(_) = url::Url::parse(s) {
        StringType::Url
    } else if HOSTNAME_REGEX.is_match(s) {
        StringType::Hostname
    } else {
        StringType::Unknown {
            strings_seen: std::collections::HashSet::from_iter([s.to_owned()]),
            chars_seen: s.chars().collect(),
            min_length: Some(s.len()),
            max_length: Some(s.len()),
        }
    }
}
