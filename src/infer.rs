use crate::{infer_string::infer_string_type, NumberType, SchemaState, StringType};
use rayon::prelude::*;
use std::cmp;

pub struct EnumInference {
    /// The maximum ratio of unique values to total values in a collection of strings for it to be considered an enum.
    pub max_unique_ratio: f64,
    /// The minimum number of values in a collection of strings for enum inference to be applied.
    pub min_sample_size: usize,
}

pub struct InferenceOptions {
    pub enum_inference: Option<EnumInference>,
}

fn merge(initial: SchemaState, new: SchemaState) -> SchemaState {
    match (initial, new) {
        (SchemaState::Initial, s)
        | (s, SchemaState::Initial)
        | (SchemaState::Indefinite, s)
        | (s, SchemaState::Indefinite) => s,

        // --- String merging ---
        (
            SchemaState::String(StringType::Unknown {
                mut strings_seen,
                mut chars_seen,
                min_length,
                max_length,
            }),
            SchemaState::String(StringType::Unknown {
                strings_seen: second_strings_seen,
                chars_seen: second_chars_seen,
                min_length: second_min_length,
                max_length: second_max_length,
            }),
        ) => {
            let min_length = match (min_length, second_min_length) {
                (Some(min_length), Some(second_min_length)) => {
                    Some(cmp::min(min_length, second_min_length))
                }
                (Some(min_length), None) => Some(min_length),
                (None, Some(second_min_length)) => Some(second_min_length),
                _ => None,
            };

            let max_length = match (max_length, second_max_length) {
                (Some(max_length), Some(second_max_length)) => {
                    Some(cmp::max(max_length, second_max_length))
                }
                (Some(max_length), None) => Some(max_length),
                (None, Some(second_max_length)) => Some(second_max_length),
                _ => None,
            };

            chars_seen.extend(second_chars_seen);
            strings_seen.extend(second_strings_seen);

            SchemaState::String(StringType::Unknown {
                strings_seen,
                chars_seen,
                min_length,
                max_length,
            })
        }

        (s @ SchemaState::String(StringType::Unknown { .. }), SchemaState::String(_))
        | (SchemaState::String(_), s @ SchemaState::String(StringType::Unknown { .. })) => s,

        (SchemaState::String(first_type), SchemaState::String(second_type)) => {
            if first_type == second_type {
                SchemaState::String(first_type)
            } else {
                SchemaState::String(StringType::Unknown {
                    strings_seen: vec![],
                    chars_seen: vec![],
                    min_length: None,
                    max_length: None,
                })
            }
        }

        // --- Number merging ---
        (
            SchemaState::Number(NumberType::Float {
                min: first_min,
                max: first_max,
            }),
            SchemaState::Number(NumberType::Float {
                min: second_min,
                max: second_max,
            }),
        ) => SchemaState::Number(NumberType::Float {
            min: first_min.min(second_min),
            max: first_max.max(second_max),
        }),

        (
            SchemaState::Number(NumberType::Float {
                min: first_min,
                max: first_max,
            }),
            SchemaState::Number(NumberType::Integer {
                min: second_min,
                max: second_max,
            }),
        ) => SchemaState::Number(NumberType::Float {
            min: first_min.min(second_min as f64),
            max: first_max.max(second_max as f64),
        }),

        (
            SchemaState::Number(NumberType::Integer {
                min: first_min,
                max: first_max,
            }),
            SchemaState::Number(NumberType::Float {
                min: second_min,
                max: second_max,
            }),
        ) => SchemaState::Number(NumberType::Float {
            min: (first_min as f64).min(second_min),
            max: (first_max as f64).max(second_max),
        }),

        (
            SchemaState::Number(NumberType::Integer {
                min: first_min,
                max: first_max,
            }),
            SchemaState::Number(NumberType::Integer {
                min: second_min,
                max: second_max,
            }),
        ) => SchemaState::Number(NumberType::Integer {
            min: cmp::min(first_min, second_min),
            max: cmp::max(first_max, second_max),
        }),

        // --- Boolean merging ---
        (SchemaState::Boolean, SchemaState::Boolean) => SchemaState::Boolean,

        // --- Array merging ---
        (
            SchemaState::Array {
                min_length,
                max_length,
                schema,
            },
            SchemaState::Array {
                min_length: second_min_length,
                max_length: second_max_length,
                schema: second_schema,
            },
        ) => {
            let min_length = cmp::min(min_length, second_min_length);
            let max_length = cmp::max(max_length, second_max_length);
            let schema = Box::new(merge(*schema, *second_schema));
            SchemaState::Array {
                min_length,
                max_length,
                schema,
            }
        }

        // --- Object merging ---
        (
            SchemaState::Object {
                required: mut first_required,
                optional: mut first_optional,
            },
            SchemaState::Object {
                required: mut second_required,
                optional: mut second_optional,
            },
        ) => {
            let required_keys: std::collections::HashSet<String> = first_required
                .keys()
                .filter(|k| second_required.contains_key(*k))
                .cloned()
                .collect();

            let optional_keys: std::collections::HashSet<String> = first_optional
                .keys()
                .chain(second_optional.keys())
                .chain(
                    first_required
                        .keys()
                        .chain(second_required.keys())
                        .filter(|key| !required_keys.contains(*key)),
                )
                .cloned()
                .collect();

            let required: std::collections::HashMap<String, SchemaState> = required_keys
                .into_iter()
                .map(|k| {
                    let first = first_required.remove(&k);
                    let second = second_required.remove(&k);
                    let merged = match (first, second) {
                        (Some(first), Some(second)) => merge(first, second),
                        (Some(first), None) => first,
                        (None, Some(second)) => second,
                        _ => unreachable!(),
                    };
                    (k, merged)
                })
                .collect();

            let optional: std::collections::HashMap<String, SchemaState> = optional_keys
                .into_iter()
                .map(|k| {
                    let first = first_required
                        .remove(&k)
                        .or_else(|| first_optional.remove(&k));
                    let second = second_required
                        .remove(&k)
                        .or_else(|| second_optional.remove(&k));
                    let merged = match (first, second) {
                        (Some(first), Some(second)) => merge(first, second),
                        (Some(first), None) => first,
                        (None, Some(second)) => second,
                        _ => unreachable!(),
                    };
                    (k, merged)
                })
                .collect();

            SchemaState::Object { required, optional }
        }

        // --- Null(able) merging ---
        (SchemaState::Null, SchemaState::Null) => SchemaState::Null,

        (SchemaState::Null, SchemaState::Nullable(inner))
        | (SchemaState::Nullable(inner), SchemaState::Null) => SchemaState::Nullable(inner),

        (non_null_type, SchemaState::Null) => SchemaState::Nullable(Box::new(non_null_type)),
        (SchemaState::Null, non_null_type) => SchemaState::Nullable(Box::new(non_null_type)),

        (SchemaState::Nullable(first_inner), SchemaState::Nullable(second_inner)) => {
            SchemaState::Nullable(Box::new(merge(*first_inner, *second_inner)))
        }

        (SchemaState::Nullable(inner), non_nullable_type) => {
            SchemaState::Nullable(Box::new(merge(*inner, non_nullable_type)))
        }
        (non_nullable_type, SchemaState::Nullable(inner)) => {
            SchemaState::Nullable(Box::new(merge(non_nullable_type, *inner)))
        }

        // --- Fallback ---
        _ => SchemaState::Indefinite,
    }
}

fn apply_enum_inner(s: StringType, opts: &EnumInference) -> StringType {
    match &s {
        StringType::Unknown { strings_seen, .. } => {
            if strings_seen.len() < opts.min_sample_size {
                return s;
            }

            let variants = strings_seen
                .iter()
                .cloned()
                .collect::<std::collections::HashSet<_>>();

            let unique_ratio = variants.len() as f64 / strings_seen.len() as f64;
            if unique_ratio > opts.max_unique_ratio {
                return s;
            }

            StringType::Enum { variants }
        }
        _ => s,
    }
}

fn apply_enum_recursive(s: SchemaState, opts: &EnumInference) -> SchemaState {
    match s {
        SchemaState::String(s) => SchemaState::String(apply_enum_inner(s, opts)),
        SchemaState::Array {
            min_length,
            max_length,
            schema,
        } => SchemaState::Array {
            min_length,
            max_length,
            schema: Box::new(apply_enum_recursive(*schema, opts)),
        },
        SchemaState::Object { required, optional } => SchemaState::Object {
            required: required
                .into_iter()
                .map(|(k, v)| (k, apply_enum_recursive(v, opts)))
                .collect(),
            optional: optional
                .into_iter()
                .map(|(k, v)| (k, apply_enum_recursive(v, opts)))
                .collect(),
        },
        SchemaState::Nullable(inner) => {
            SchemaState::Nullable(Box::new(apply_enum_recursive(*inner, opts)))
        }
        _ => s,
    }
}

/// Infer a schema, encoded as a SchemaState struct, from a JSON value.
/// This function will recursively traverse the given JSON structure and return a SchemaState struct.
///
/// # Example
///
/// ```
/// use serde_json::json;
/// use std::collections::{HashMap, HashSet};
/// use drivel::{infer_schema, SchemaState, StringType, NumberType, InferenceOptions};
///
/// let opts = InferenceOptions {
///     enum_inference: None
/// };
///
/// // Define a JSON value
/// let input = json!({
///     "name": "John",
///     "age": 30,
///     "is_student": false,
///     "grades": [85, 92, 78]
/// });
///
/// assert_eq!(
///     infer_schema(input, &opts),
///     SchemaState::Object {
///         required: HashMap::from_iter([
///             ("name".to_string(), SchemaState::String(StringType::Unknown {
///                 strings_seen: vec!["John".to_string()],
///                 chars_seen: vec!['J', 'o', 'h', 'n'],
///                 min_length: Some(4),
///                 max_length: Some(4)
///             })),
///             ("age".to_string(), SchemaState::Number(NumberType::Integer { min: 30, max: 30 })),
///             ("is_student".to_string(), SchemaState::Boolean),
///             ("grades".to_string(), SchemaState::Array {
///                 min_length: 3,
///                 max_length: 3,
///                 schema: Box::new(SchemaState::Number(NumberType::Integer { min: 78, max: 92 }))
///             }),
///         ]),
///         optional: HashMap::new()
///     }
/// );
/// ```
pub fn infer_schema(json: serde_json::Value, options: &InferenceOptions) -> SchemaState {
    let inferred = match json {
        serde_json::Value::Null => SchemaState::Null,
        serde_json::Value::String(value) => SchemaState::String(infer_string_type(&value)),
        serde_json::Value::Number(n) => SchemaState::Number(if n.is_f64() {
            NumberType::Float {
                min: n.as_f64().unwrap(),
                max: n.as_f64().unwrap(),
            }
        } else {
            NumberType::Integer {
                min: n.as_i64().unwrap(),
                max: n.as_i64().unwrap(),
            }
        }),
        serde_json::Value::Bool(_) => SchemaState::Boolean,
        serde_json::Value::Array(array) => SchemaState::Array {
            min_length: array.len(),
            max_length: array.len(),
            schema: Box::new(infer_schema_from_iter(array, options)),
        },
        serde_json::Value::Object(object) => SchemaState::Object {
            required: object
                .into_iter()
                .map(|(k, v)| (k, infer_schema(v, options)))
                .collect(),
            optional: std::collections::HashMap::new(),
        },
    };

    if let Some(enum_opts) = &options.enum_inference {
        apply_enum_recursive(inferred, enum_opts)
    } else {
        inferred
    }
}

/// Infer a schema, encoded as a SchemaState struct, from an iterator of JSON values.
///
/// This function iterates over a collection of JSON values and infers the schema by
/// merging schemas inferred from individual JSON values. The resulting schema reflects
/// the combined schema of all JSON values in the iterator.
///
/// # Example
///
/// ```
/// use serde_json::json;
/// use std::collections::{HashMap, HashSet};
/// use drivel::{infer_schema_from_iter, SchemaState, StringType, NumberType, InferenceOptions};
///
/// // Define a collection of JSON values
/// let values = vec![
///     json!({
///         "name": "Alice",
///         "age": 30,
///         "is_student": true
///     }),
///     json!({
///         "name": "Bob",
///         "age": 25,
///         "is_student": false
///     })
/// ];
///
/// let opts = InferenceOptions {
///     enum_inference: None
/// };
///
/// // Infer the schema from the iterator of JSON values
/// let schema = infer_schema_from_iter(values, &opts);
///
/// assert_eq!(
///     schema,
///     SchemaState::Object {
///         required: HashMap::from_iter([
///             ("name".to_string(), SchemaState::String(StringType::Unknown {
///                 strings_seen: vec!["Alice".to_string(), "Bob".to_string()],
///                 chars_seen: vec!['A', 'l', 'i', 'c', 'e', 'B', 'o', 'b'],
///                 min_length: Some(3),
///                 max_length: Some(5)
///             })),
///             ("age".to_string(), SchemaState::Number(NumberType::Integer { min: 25, max: 30 })),
///             ("is_student".to_string(), SchemaState::Boolean),
///         ]),
///         optional: HashMap::new()
///     }
/// );
/// ```
pub fn infer_schema_from_iter(
    values: Vec<serde_json::Value>,
    options: &InferenceOptions,
) -> SchemaState {
    values
        .into_par_iter()
        .map(|value| infer_schema(value, options))
        .reduce(|| SchemaState::Initial, merge)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn no_enum_options() -> InferenceOptions {
        InferenceOptions {
            enum_inference: None,
        }
    }

    #[test]
    fn infers_null() {
        let input = json!(null);
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(schema, SchemaState::Null)
    }

    #[test]
    fn infers_string_unknown_type() {
        let input = json!("foo");
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(
            schema,
            SchemaState::String(StringType::Unknown {
                strings_seen: vec!["foo".to_owned()],
                chars_seen: vec!['f', 'o', 'o'],
                min_length: Some(3),
                max_length: Some(3)
            })
        )
    }

    #[test]
    fn infers_string_iso_date() {
        let input = json!("2013-01-12");
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(schema, SchemaState::String(StringType::IsoDate))
    }

    #[test]
    fn infers_string_iso_date_time_rfc_2822() {
        let input = json!("Thu, 18 Mar 2021 10:37:31 +0000");
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(schema, SchemaState::String(StringType::DateTimeRFC2822))
    }

    #[test]
    fn infers_string_iso_date_time_rfc_3339_offset() {
        let input = json!("2013-01-12T00:00:00.000+00:00");
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(schema, SchemaState::String(StringType::DateTimeISO8601))
    }

    #[test]
    fn infers_string_iso_date_time_rfc_3339_utc() {
        let input = json!("2013-01-12T00:00:00.000Z");
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(schema, SchemaState::String(StringType::DateTimeISO8601))
    }

    #[test]
    fn infers_string_uuid() {
        let input = json!("988c2c6d-df1b-4bb9-b837-6ba706c0b4ad");
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(schema, SchemaState::String(StringType::UUID))
    }

    #[test]
    fn infers_string_email() {
        let input = json!("test@example.com");
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(schema, SchemaState::String(StringType::Email))
    }

    #[test]
    fn infers_string_url() {
        let input = json!("https://somedomain.somehost.nl/somepage");
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(schema, SchemaState::String(StringType::Url))
    }

    #[test]
    fn infers_string_hostname() {
        let input = json!("somehost.com");
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(schema, SchemaState::String(StringType::Hostname))
    }

    #[test]
    fn infers_number() {
        let input = json!(42);
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(
            schema,
            SchemaState::Number(NumberType::Integer { min: 42, max: 42 })
        )
    }

    #[test]
    fn infers_number_float() {
        let input = json!(42.0);
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(
            schema,
            SchemaState::Number(NumberType::Float {
                min: 42.0,
                max: 42.0
            })
        )
    }

    #[test]
    fn infers_boolean_true() {
        let input = json!(true);
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(schema, SchemaState::Boolean)
    }

    #[test]
    fn infers_boolean_false() {
        let input = json!(false);
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(schema, SchemaState::Boolean)
    }

    #[test]
    fn infers_object() {
        let input = json!({
            "string": "foo",
            "int": 10,
            "float": 10.4,
            "bool": false,
            "array": ["baz"],
            "null": null,
            "object": {
                "string": "foo"
            }
        });
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(
            schema,
            SchemaState::Object {
                required: std::collections::HashMap::from_iter([
                    (
                        "string".to_string(),
                        SchemaState::String(StringType::Unknown {
                            strings_seen: vec!["foo".to_owned()],
                            chars_seen: vec!['f', 'o', 'o'],
                            min_length: Some(3),
                            max_length: Some(3)
                        })
                    ),
                    (
                        "int".to_string(),
                        SchemaState::Number(NumberType::Integer { min: 10, max: 10 })
                    ),
                    (
                        "float".to_string(),
                        SchemaState::Number(NumberType::Float {
                            min: 10.4,
                            max: 10.4
                        })
                    ),
                    ("bool".to_string(), SchemaState::Boolean),
                    (
                        "array".to_string(),
                        SchemaState::Array {
                            min_length: 1,
                            max_length: 1,
                            schema: Box::new(SchemaState::String(StringType::Unknown {
                                strings_seen: vec!["baz".to_owned()],
                                chars_seen: vec!['b', 'a', 'z'],
                                min_length: Some(3),
                                max_length: Some(3)
                            }))
                        }
                    ),
                    ("null".to_string(), SchemaState::Null),
                    (
                        "object".to_string(),
                        SchemaState::Object {
                            required: std::collections::HashMap::from_iter([(
                                "string".to_owned(),
                                SchemaState::String(StringType::Unknown {
                                    strings_seen: vec!["foo".to_owned()],
                                    chars_seen: vec!['f', 'o', 'o'],
                                    min_length: Some(3),
                                    max_length: Some(3)
                                })
                            )]),
                            optional: std::collections::HashMap::new(),
                        }
                    ),
                ]),
                optional: std::collections::HashMap::new()
            }
        )
    }

    #[test]
    fn infers_array_null() {
        let input = json!([null, null]);
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(
            schema,
            SchemaState::Array {
                min_length: 2,
                max_length: 2,
                schema: Box::new(SchemaState::Null)
            }
        );
    }

    #[test]
    fn infers_array_string() {
        let input = json!(["foo", "barbar"]);
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(
            schema,
            SchemaState::Array {
                min_length: 2,
                max_length: 2,
                schema: Box::new(SchemaState::String(StringType::Unknown {
                    strings_seen: vec!["foo".to_owned(), "barbar".to_owned()],
                    chars_seen: vec!['f', 'o', 'o', 'b', 'a', 'r', 'b', 'a', 'r'],
                    min_length: Some(3),
                    max_length: Some(6)
                }))
            }
        );
    }

    #[test]
    fn infers_array_string_enum() {
        let input = json!(["foo", "barbar", "barbar", "foo"]);

        let enum_opts = EnumInference {
            max_unique_ratio: 0.5,
            min_sample_size: 2,
        };
        let options = InferenceOptions {
            enum_inference: Some(enum_opts),
        };

        let schema = infer_schema(input, &options);

        assert_eq!(
            schema,
            SchemaState::Array {
                min_length: 4,
                max_length: 4,
                schema: Box::new(SchemaState::String(StringType::Enum {
                    variants: vec!["foo".to_owned(), "barbar".to_owned()]
                        .into_iter()
                        .collect()
                }))
            }
        );
    }

    #[test]
    fn infers_array_string_enum_uniq_ratio_too_high() {
        let input = json!(["foo", "barbar", "foo", "barbar"]);

        let enum_opts = EnumInference {
            max_unique_ratio: 0.4, // 2 unique values out of 4 = unique ratio of 0.5
            min_sample_size: 2,
        };
        let options = InferenceOptions {
            enum_inference: Some(enum_opts),
        };

        let schema = infer_schema(input, &options);

        assert_eq!(
            schema,
            SchemaState::Array {
                min_length: 4,
                max_length: 4,
                schema: Box::new(SchemaState::String(StringType::Unknown {
                    strings_seen: vec![
                        "foo".to_owned(),
                        "barbar".to_owned(),
                        "foo".to_owned(),
                        "barbar".to_owned()
                    ],
                    chars_seen: vec![
                        'f', 'o', 'o', 'b', 'a', 'r', 'b', 'a', 'r', 'f', 'o', 'o', 'b', 'a', 'r',
                        'b', 'a', 'r'
                    ],
                    min_length: Some(3),
                    max_length: Some(6)
                }))
            }
        );
    }

    #[test]
    fn infers_array_string_enum_sample_size_too_small() {
        let input = json!(["foo", "barbar", "foo", "barbar"]);

        let enum_opts = EnumInference {
            max_unique_ratio: 0.5,
            min_sample_size: 5, // sample size too small (4 vs 5)
        };
        let options = InferenceOptions {
            enum_inference: Some(enum_opts),
        };

        let schema = infer_schema(input, &options);

        assert_eq!(
            schema,
            SchemaState::Array {
                min_length: 4,
                max_length: 4,
                schema: Box::new(SchemaState::String(StringType::Unknown {
                    strings_seen: vec![
                        "foo".to_owned(),
                        "barbar".to_owned(),
                        "foo".to_owned(),
                        "barbar".to_owned()
                    ],
                    chars_seen: vec![
                        'f', 'o', 'o', 'b', 'a', 'r', 'b', 'a', 'r', 'f', 'o', 'o', 'b', 'a', 'r',
                        'b', 'a', 'r'
                    ],
                    min_length: Some(3),
                    max_length: Some(6)
                }))
            }
        );
    }

    #[test]
    fn infers_array_string_mixed() {
        let input = json!(["48f41410-2d97-4d54-8bfa-aa4e22acca01", "barbar"]);
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(
            schema,
            SchemaState::Array {
                min_length: 2,
                max_length: 2,
                schema: Box::new(SchemaState::String(StringType::Unknown {
                    strings_seen: vec!["barbar".to_owned()],
                    chars_seen: vec!['b', 'a', 'r', 'b', 'a', 'r'],
                    min_length: Some(6),
                    max_length: Some(6),
                }))
            }
        )
    }

    #[test]
    fn infers_array_number() {
        let input = json!([100, 104]);
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(
            schema,
            SchemaState::Array {
                min_length: 2,
                max_length: 2,
                schema: Box::new(SchemaState::Number(NumberType::Integer {
                    min: 100,
                    max: 104
                }))
            }
        )
    }

    #[test]
    fn infers_array_number_float() {
        let input = json!([100, 104.5]);
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(
            schema,
            SchemaState::Array {
                min_length: 2,
                max_length: 2,
                schema: Box::new(SchemaState::Number(NumberType::Float {
                    min: 100.0,
                    max: 104.5
                }))
            }
        );
    }

    #[test]
    fn infers_array_boolean() {
        let input = json!([true, false]);
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(
            schema,
            SchemaState::Array {
                min_length: 2,
                max_length: 2,
                schema: Box::new(SchemaState::Boolean)
            }
        );
    }

    #[test]
    fn infers_array_object() {
        let input = json!([
            {
                "foo": "bar",
                "baz": 10,
                "qux": true
            },
            {
                "baz": null,
                "qux": false
            },
            {
                "foo": "barbar",
                "baz": 20,
                "qux": true
            },
        ]);
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(
            schema,
            SchemaState::Array {
                min_length: 3,
                max_length: 3,
                schema: Box::new(SchemaState::Object {
                    required: std::collections::HashMap::from_iter([
                        (
                            "baz".to_owned(),
                            SchemaState::Nullable(Box::new(SchemaState::Number(
                                NumberType::Integer { min: 10, max: 20 }
                            )))
                        ),
                        ("qux".to_owned(), SchemaState::Boolean),
                    ]),
                    optional: std::collections::HashMap::from_iter([(
                        "foo".to_owned(),
                        SchemaState::String(StringType::Unknown {
                            strings_seen: vec!["bar".to_owned(), "barbar".to_owned()],
                            chars_seen: vec!['b', 'a', 'r', 'b', 'a', 'r', 'b', 'a', 'r'],
                            min_length: Some(3),
                            max_length: Some(6)
                        })
                    )])
                })
            }
        )
    }

    #[test]
    fn infers_array_object_enum() {
        let input = json!([
            {
                "foo": "bar",
            },
            {
                "foo": "bar",
            },
            {
                "foo": "baz",
            },
            {
                "foo": "bar",
            }
        ]);
        let enun_opts = EnumInference {
            max_unique_ratio: 0.5,
            min_sample_size: 2,
        };
        let options = InferenceOptions {
            enum_inference: Some(enun_opts),
        };
        let schema = infer_schema(input, &options);

        assert_eq!(
            schema,
            SchemaState::Array {
                min_length: 4,
                max_length: 4,
                schema: Box::new(SchemaState::Object {
                    required: std::collections::HashMap::from_iter([(
                        "foo".to_owned(),
                        SchemaState::String(StringType::Enum {
                            variants: vec!["bar".to_owned(), "baz".to_owned()]
                                .into_iter()
                                .collect()
                        })
                    )]),
                    optional: std::collections::HashMap::new()
                })
            }
        )
    }

    #[test]
    fn infers_nested_array() {
        let input = json!([[true, false], [false]]);
        let options = no_enum_options();
        let schema = infer_schema(input, &options);

        assert_eq!(
            schema,
            SchemaState::Array {
                min_length: 2,
                max_length: 2,
                schema: Box::new(SchemaState::Array {
                    min_length: 1,
                    max_length: 2,
                    schema: Box::new(SchemaState::Boolean),
                }),
            }
        )
    }

    #[test]
    fn infers_nullable_array() {
        let input_1 = json!(["foo", null]);
        let options = no_enum_options();
        let schema_1 = infer_schema(input_1, &options);

        let input_2 = json!([null, "foo"]);
        let schema_2 = infer_schema(input_2, &options);

        assert_eq!(
            schema_1,
            SchemaState::Array {
                min_length: 2,
                max_length: 2,
                schema: Box::new(SchemaState::Nullable(Box::new(SchemaState::String(
                    StringType::Unknown {
                        strings_seen: vec!["foo".to_owned()],
                        chars_seen: vec!['f', 'o', 'o'],
                        min_length: Some(3),
                        max_length: Some(3)
                    }
                ))))
            }
        );

        assert_eq!(schema_1, schema_2)
    }

    #[test]
    fn infers_from_iter() {
        let input = vec![
            json!({
                "foo": "bar",
                "baz": 10,
                "qux": true
            }),
            json!({
                "baz": null,
                "qux": false
            }),
            json!({
                "foo": "barbar",
                "baz": 20,
                "qux": true
            }),
        ];
        let options = no_enum_options();
        let schema = infer_schema_from_iter(input, &options);
        assert_eq!(
            schema,
            SchemaState::Object {
                required: std::collections::HashMap::from_iter([
                    (
                        "baz".to_owned(),
                        SchemaState::Nullable(Box::new(SchemaState::Number(NumberType::Integer {
                            min: 10,
                            max: 20
                        })))
                    ),
                    ("qux".to_owned(), SchemaState::Boolean),
                ]),
                optional: std::collections::HashMap::from_iter([(
                    "foo".to_owned(),
                    SchemaState::String(StringType::Unknown {
                        strings_seen: vec!["bar".to_owned(), "barbar".to_owned()],
                        chars_seen: vec!['b', 'a', 'r', 'b', 'a', 'r', 'b', 'a', 'r'],
                        min_length: Some(3),
                        max_length: Some(6)
                    })
                )])
            }
        );
    }
}
