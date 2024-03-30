use crate::{NumberType, SchemaState, StringType};

lazy_static! {
    static ref ISO_DATE_REGEX: regex::Regex = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    static ref ISO_DATE_TIME_REGEX: regex::Regex = regex::Regex::new(
        r"^\d{4}-[01]\d-[0-3]\dT[0-2]\d:[0-5]\d:[0-5]\d\.\d+([+-][0-2]\d:[0-5]\d|Z)$"
    )
    .unwrap();
    static ref UUIDREGEX: regex::Regex =
        regex::Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
            .unwrap();
}

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
        // Initial state; merge with any state yields that state
        (SchemaState::Initial, SchemaState::Null) => SchemaState::Null,
        (SchemaState::Initial, SchemaState::String(x)) => SchemaState::String(x),
        (SchemaState::Initial, SchemaState::Boolean) => SchemaState::Boolean,
        (SchemaState::Initial, SchemaState::Number(x)) => SchemaState::Number(x),
        (SchemaState::Initial, SchemaState::Array(inner)) => SchemaState::Array(inner),
        (SchemaState::Initial, SchemaState::Object { required, optional }) => {
            SchemaState::Object { required, optional }
        }

        // --- String merging ---
        (
            SchemaState::String(StringType::Unknown {
                min_length,
                max_length,
            }),
            SchemaState::String(StringType::Unknown {
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

            SchemaState::String(StringType::Unknown {
                min_length,
                max_length,
            })
        }
        (
            SchemaState::String(StringType::Unknown {
                min_length,
                max_length,
            }),
            SchemaState::String(_),
        ) => SchemaState::String(StringType::Unknown {
            min_length,
            max_length,
        }),
        (
            SchemaState::String(_),
            SchemaState::String(StringType::Unknown {
                min_length,
                max_length,
            }),
        ) => SchemaState::String(StringType::Unknown {
            min_length,
            max_length,
        }),
        (SchemaState::String(StringType::UUID), SchemaState::String(StringType::UUID)) => {
            SchemaState::String(StringType::UUID)
        }
        (SchemaState::String(StringType::IsoDate), SchemaState::String(StringType::IsoDate)) => {
            SchemaState::String(StringType::IsoDate)
        }
        (
            SchemaState::String(StringType::IsoDateTime),
            SchemaState::String(StringType::IsoDateTime),
        ) => SchemaState::String(StringType::IsoDateTime),
        (SchemaState::String(_), SchemaState::String(_)) => {
            SchemaState::String(StringType::Unknown {
                min_length: None,
                max_length: None,
            })
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
        (SchemaState::Array(first_schema), SchemaState::Array(second_schema)) => {
            SchemaState::Array(Box::new(merge(*first_schema, *second_schema)))
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
        (s, SchemaState::Null) => SchemaState::Nullable(Box::new(s)),
        (SchemaState::Null, s) => SchemaState::Nullable(Box::new(s)),
        (SchemaState::Nullable(first_inner), SchemaState::Nullable(second_inner)) => {
            SchemaState::Nullable(Box::new(merge(*first_inner, *second_inner)))
        }
        (SchemaState::Nullable(first_inner), second) => {
            SchemaState::Nullable(Box::new(merge(*first_inner, second)))
        }
        (first, SchemaState::Nullable(second_inner)) => {
            SchemaState::Nullable(Box::new(merge(first, *second_inner)))
        }

        // --- Fallback ---
        _ => SchemaState::Indefinite,
    }
}

fn infer_array_schema(values: &[serde_json::Value]) -> SchemaState {
    values
        .iter()
        .map(infer_schema)
        .fold(SchemaState::Initial, merge)
}

pub fn infer_schema(json: &serde_json::Value) -> SchemaState {
    match json {
        serde_json::Value::Null => SchemaState::Null,
        serde_json::Value::String(value) => {
            let t = if ISO_DATE_REGEX.is_match(value) {
                StringType::IsoDate
            } else if ISO_DATE_TIME_REGEX.is_match(value) {
                StringType::IsoDateTime
            } else if UUIDREGEX.is_match(value) {
                StringType::UUID
            } else {
                StringType::Unknown {
                    min_length: Some(value.len()),
                    max_length: Some(value.len()),
                }
            };
            SchemaState::String(t)
        }
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
        serde_json::Value::Array(array) => SchemaState::Array(Box::new(infer_array_schema(array))),
        serde_json::Value::Object(object) => SchemaState::Object {
            required: object
                .iter()
                .map(|(k, v)| (k.clone(), infer_schema(v)))
                .collect(),
            optional: std::collections::HashMap::new(),
        },
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn infers_null() {
        let input = json!(null);
        let schema = infer_schema(&input);

        assert_eq!(schema, SchemaState::Null)
    }

    #[test]
    fn infers_string_unknown_type() {
        let input = json!("foo");
        let schema = infer_schema(&input);

        assert_eq!(
            schema,
            SchemaState::String(StringType::Unknown {
                min_length: Some(3),
                max_length: Some(3)
            })
        )
    }

    #[test]
    fn infers_string_iso_date() {
        let input = json!("2013-01-12");
        let schema = infer_schema(&input);

        assert_eq!(schema, SchemaState::String(StringType::IsoDate))
    }

    #[test]
    fn infers_string_iso_date_time() {
        let input = json!("2013-01-12T00:00:00.000Z");
        let schema = infer_schema(&input);

        assert_eq!(schema, SchemaState::String(StringType::IsoDateTime))
    }

    #[test]
    fn infers_string_uuid() {
        let input = json!("988c2c6d-df1b-4bb9-b837-6ba706c0b4ad");
        let schema = infer_schema(&input);

        assert_eq!(schema, SchemaState::String(StringType::UUID))
    }

    #[test]
    fn infers_number() {
        let input = json!(42);
        let schema = infer_schema(&input);

        assert_eq!(
            schema,
            SchemaState::Number(NumberType::Integer { min: 42, max: 42 })
        )
    }

    #[test]
    fn infers_number_float() {
        let input = json!(42.0);
        let schema = infer_schema(&input);

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
        let schema = infer_schema(&input);

        assert_eq!(schema, SchemaState::Boolean)
    }

    #[test]
    fn infers_boolean_false() {
        let input = json!(false);
        let schema = infer_schema(&input);

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
        let schema = infer_schema(&input);

        assert_eq!(
            schema,
            SchemaState::Object {
                required: std::collections::HashMap::from_iter([
                    (
                        "string".to_string(),
                        SchemaState::String(StringType::Unknown {
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
                        SchemaState::Array(Box::new(SchemaState::String(StringType::Unknown {
                            min_length: Some(3),
                            max_length: Some(3)
                        })))
                    ),
                    ("null".to_string(), SchemaState::Null),
                    (
                        "object".to_string(),
                        SchemaState::Object {
                            required: std::collections::HashMap::from_iter([(
                                "string".to_owned(),
                                SchemaState::String(StringType::Unknown {
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
        let schema = infer_schema(&input);

        assert_eq!(schema, SchemaState::Array(Box::new(SchemaState::Null)));
    }

    #[test]
    fn infers_array_string() {
        let input = json!(["foo", "barbar"]);
        let schema = infer_schema(&input);

        assert_eq!(
            schema,
            SchemaState::Array(Box::new(SchemaState::String(StringType::Unknown {
                min_length: Some(3),
                max_length: Some(6)
            })))
        );
    }

    #[test]
    fn infers_array_number() {
        let input = json!([100, 104]);
        let schema = infer_schema(&input);

        assert_eq!(
            schema,
            SchemaState::Array(Box::new(SchemaState::Number(NumberType::Integer {
                min: 100,
                max: 104
            })))
        );
    }

    #[test]
    fn infers_array_number_float() {
        let input = json!([100, 104.5]);
        let schema = infer_schema(&input);

        assert_eq!(
            schema,
            SchemaState::Array(Box::new(SchemaState::Number(NumberType::Float {
                min: 100.0,
                max: 104.5
            })))
        );
    }

    #[test]
    fn infers_array_boolean() {
        let input = json!([true, false]);
        let schema = infer_schema(&input);

        assert_eq!(schema, SchemaState::Array(Box::new(SchemaState::Boolean)));
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
        let schema = infer_schema(&input);

        assert_eq!(
            schema,
            SchemaState::Array(Box::new(SchemaState::Object {
                required: std::collections::HashMap::from_iter([
                    (
                        "baz".to_owned(),
                        SchemaState::Nullable(Box::new(SchemaState::Number(NumberType::Integer {
                            min: 10,
                            max: 20,
                        })))
                    ),
                    ("qux".to_owned(), SchemaState::Boolean),
                ]),
                optional: std::collections::HashMap::from_iter([(
                    "foo".to_owned(),
                    SchemaState::String(StringType::Unknown {
                        min_length: Some(3),
                        max_length: Some(6)
                    })
                )])
            }))
        )
    }

    #[test]
    fn infers_nested_array() {
        let input = json!([[true, false], [false]]);
        let schema = infer_schema(&input);

        assert_eq!(
            schema,
            SchemaState::Array(Box::new(SchemaState::Array(Box::new(SchemaState::Boolean))))
        );
    }

    #[test]
    fn infers_nullable_array() {
        let input_1 = json!(["foo", null]);
        let schema_1 = infer_schema(&input_1);

        let input_2 = json!([null, "foo"]);
        let schema_2 = infer_schema(&input_2);

        assert_eq!(
            schema_1,
            SchemaState::Array(Box::new(SchemaState::Nullable(Box::new(
                SchemaState::String(StringType::Unknown {
                    min_length: Some(3),
                    max_length: Some(3)
                })
            ))))
        );

        assert_eq!(schema_1, schema_2)
    }
}
