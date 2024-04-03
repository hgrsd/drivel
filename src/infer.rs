use crate::{infer_string::infer_string_type, NumberType, SchemaState, StringType};

fn min<T: PartialOrd>(left: T, right: T) -> T {
    if left < right {
        left
    } else {
        right
    }
}

fn max<T: PartialOrd>(left: T, right: T) -> T {
    if left > right {
        left
    } else {
        right
    }
}

fn merge(initial: SchemaState, new: SchemaState) -> SchemaState {
    match (initial, new) {
        (SchemaState::Initial, new) => new,
        (SchemaState::Indefinite, s) | (s, SchemaState::Indefinite) => s,

        // --- String merging ---
        (
            SchemaState::String(StringType::Unknown {
                mut chars_seen,
                min_length,
                max_length,
            }),
            SchemaState::String(StringType::Unknown {
                chars_seen: second_chars_seen,
                min_length: second_min_length,
                max_length: second_max_length,
            }),
        ) => {
            let min_length = if min_length.is_some() && second_min_length.is_some() {
                Some(min(min_length.unwrap(), second_min_length.unwrap()))
            } else if min_length.is_some() {
                min_length
            } else {
                second_min_length
            };

            let max_length = if max_length.is_some() && second_max_length.is_some() {
                Some(max(max_length.unwrap(), second_max_length.unwrap()))
            } else if max_length.is_some() {
                max_length
            } else {
                second_max_length
            };

            chars_seen.extend(second_chars_seen);

            SchemaState::String(StringType::Unknown {
                chars_seen,
                min_length,
                max_length,
            })
        }

        (
            SchemaState::String(StringType::Unknown {
                chars_seen,
                min_length,
                max_length,
            }),
            SchemaState::String(_),
        )
        | (
            SchemaState::String(_),
            SchemaState::String(StringType::Unknown {
                chars_seen,
                min_length,
                max_length,
            }),
        ) => SchemaState::String(StringType::Unknown {
            chars_seen,
            min_length,
            max_length,
        }),

        (SchemaState::String(first_type), SchemaState::String(second_type)) => {
            if first_type == second_type {
                SchemaState::String(first_type)
            } else {
                SchemaState::String(StringType::Unknown {
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
            min: min(first_min, second_min),
            max: max(first_max, second_max),
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
            min: min(first_min, second_min as f64),
            max: max(first_max, second_max as f64),
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
            min: min(first_min as f64, second_min),
            max: max(first_max as f64, second_max),
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
            min: min(first_min, second_min),
            max: max(first_max, second_max),
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
            let min_length = min(min_length, second_min_length);
            let max_length = max(max_length, second_max_length);
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
                    let merged = if first.is_some() && second.is_some() {
                        merge(first.unwrap(), second.unwrap())
                    } else {
                        first.unwrap_or(second.unwrap())
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
                    let merged = if first.is_some() && second.is_some() {
                        merge(first.unwrap(), second.unwrap())
                    } else {
                        first.unwrap_or_else(|| second.unwrap())
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

/// Infer a schema, encoded as a SchemaState struct, from a JSON value.
/// This function will recursively traverse the given JSON structure and return a SchemaState struct.
///
/// # Example
///
/// ```
/// use serde_json::json;
/// use std::collections::{HashMap, HashSet};
/// use drivel::{infer_schema, SchemaState, StringType, NumberType};
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
///     infer_schema(input),
///     SchemaState::Object {
///         required: HashMap::from_iter([
///             ("name".to_string(), SchemaState::String(StringType::Unknown {
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
pub fn infer_schema(json: serde_json::Value) -> SchemaState {
    match json {
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
            schema: Box::new(infer_schema_from_iter(array.into_iter())),
        },
        serde_json::Value::Object(object) => SchemaState::Object {
            required: object
                .into_iter()
                .map(|(k, v)| (k, infer_schema(v)))
                .collect(),
            optional: std::collections::HashMap::new(),
        },
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
/// use drivel::{infer_schema_from_iter, SchemaState, StringType, NumberType};
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
/// // Infer the schema from the iterator of JSON values
/// let schema = infer_schema_from_iter(values.into_iter());
///
/// assert_eq!(
///     schema,
///     SchemaState::Object {
///         required: HashMap::from_iter([
///             ("name".to_string(), SchemaState::String(StringType::Unknown {
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
pub fn infer_schema_from_iter<'a>(values: impl Iterator<Item = serde_json::Value>) -> SchemaState {
    values
        .map(|value| infer_schema(value))
        .fold(SchemaState::Initial, merge)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn infers_null() {
        let input = json!(null);
        let schema = infer_schema(input);

        assert_eq!(schema, SchemaState::Null)
    }

    #[test]
    fn infers_string_unknown_type() {
        let input = json!("foo");
        let schema = infer_schema(input);

        assert_eq!(
            schema,
            SchemaState::String(StringType::Unknown {
                chars_seen: vec!['f', 'o', 'o'],
                min_length: Some(3),
                max_length: Some(3)
            })
        )
    }

    #[test]
    fn infers_string_iso_date() {
        let input = json!("2013-01-12");
        let schema = infer_schema(input);

        assert_eq!(schema, SchemaState::String(StringType::IsoDate))
    }

    #[test]
    fn infers_string_iso_date_time_rfc_2822() {
        let input = json!("Thu, 18 Mar 2021 10:37:31 +0000");
        let schema = infer_schema(input);

        assert_eq!(schema, SchemaState::String(StringType::DateTimeISO8601))
    }

    #[test]
    fn infers_string_iso_date_time_rfc_3339_offset() {
        let input = json!("2013-01-12T00:00:00.000+00:00");
        let schema = infer_schema(input);

        assert_eq!(schema, SchemaState::String(StringType::DateTimeISO8601))
    }

    #[test]
    fn infers_string_iso_date_time_rfc_3339_utc() {
        let input = json!("2013-01-12T00:00:00.000Z");
        let schema = infer_schema(input);

        assert_eq!(schema, SchemaState::String(StringType::DateTimeISO8601))
    }

    #[test]
    fn infers_string_uuid() {
        let input = json!("988c2c6d-df1b-4bb9-b837-6ba706c0b4ad");
        let schema = infer_schema(input);

        assert_eq!(schema, SchemaState::String(StringType::UUID))
    }

    #[test]
    fn infers_string_email() {
        let input = json!("test@example.com");
        let schema = infer_schema(input);

        assert_eq!(schema, SchemaState::String(StringType::Email))
    }

    #[test]
    fn infers_string_url() {
        let input = json!("https://somedomain.somehost.nl/somepage");
        let schema = infer_schema(input);

        assert_eq!(schema, SchemaState::String(StringType::Url))
    }

    #[test]
    fn infers_string_hostname() {
        let input = json!("somehost.com");
        let schema = infer_schema(input);

        assert_eq!(schema, SchemaState::String(StringType::Hostname))
    }

    #[test]
    fn infers_number() {
        let input = json!(42);
        let schema = infer_schema(input);

        assert_eq!(
            schema,
            SchemaState::Number(NumberType::Integer { min: 42, max: 42 })
        )
    }

    #[test]
    fn infers_number_float() {
        let input = json!(42.0);
        let schema = infer_schema(input);

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
        let schema = infer_schema(input);

        assert_eq!(schema, SchemaState::Boolean)
    }

    #[test]
    fn infers_boolean_false() {
        let input = json!(false);
        let schema = infer_schema(input);

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
        let schema = infer_schema(input);

        assert_eq!(
            schema,
            SchemaState::Object {
                required: std::collections::HashMap::from_iter([
                    (
                        "string".to_string(),
                        SchemaState::String(StringType::Unknown {
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
        let schema = infer_schema(input);

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
        let schema = infer_schema(input);

        assert_eq!(
            schema,
            SchemaState::Array {
                min_length: 2,
                max_length: 2,
                schema: Box::new(SchemaState::String(StringType::Unknown {
                    chars_seen: vec!['f', 'o', 'o', 'b', 'a', 'r', 'b', 'a', 'r'],
                    min_length: Some(3),
                    max_length: Some(6)
                }))
            }
        );
    }

    #[test]
    fn infers_array_string_mixed() {
        let input = json!(["48f41410-2d97-4d54-8bfa-aa4e22acca01", "barbar"]);
        let schema = infer_schema(input);

        assert_eq!(
            schema,
            SchemaState::Array {
                min_length: 2,
                max_length: 2,
                schema: Box::new(SchemaState::String(StringType::Unknown {
                    chars_seen: vec!['b', 'a', 'r', 'b', 'a', 'r'],
                    min_length: Some(6),
                    max_length: Some(6),
                }))
            }
        );
    }

    #[test]
    fn infers_array_number() {
        let input = json!([100, 104]);
        let schema = infer_schema(input);

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
        );
    }

    #[test]
    fn infers_array_number_float() {
        let input = json!([100, 104.5]);
        let schema = infer_schema(input);

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
        let schema = infer_schema(input);

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
        let schema = infer_schema(input);

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
    fn infers_nested_array() {
        let input = json!([[true, false], [false]]);
        let schema = infer_schema(input);

        assert_eq!(
            schema,
            SchemaState::Array {
                min_length: 2,
                max_length: 2,
                schema: Box::new(SchemaState::Array {
                    min_length: 1,
                    max_length: 2,
                    schema: Box::new(SchemaState::Boolean)
                })
            }
        );
    }

    #[test]
    fn infers_nullable_array() {
        let input_1 = json!(["foo", null]);
        let schema_1 = infer_schema(input_1);

        let input_2 = json!([null, "foo"]);
        let schema_2 = infer_schema(input_2);

        assert_eq!(
            schema_1,
            SchemaState::Array {
                min_length: 2,
                max_length: 2,
                schema: Box::new(SchemaState::Nullable(Box::new(SchemaState::String(
                    StringType::Unknown {
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
        let schema = infer_schema_from_iter(input.into_iter());
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
                        chars_seen: vec!['b', 'a', 'r', 'b', 'a', 'r', 'b', 'a', 'r'],
                        min_length: Some(3),
                        max_length: Some(6)
                    })
                )])
            }
        );
    }
}
