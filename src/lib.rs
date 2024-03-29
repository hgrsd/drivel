#[macro_use]
extern crate lazy_static;

lazy_static! {
    static ref IsoDateRegex: regex::Regex = regex::Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();
    static ref IsoDateTimeRegex: regex::Regex = regex::Regex::new(
        r"^\d{4}-[01]\d-[0-3]\dT[0-2]\d:[0-5]\d:[0-5]\d\.\d+([+-][0-2]\d:[0-5]\d|Z)$"
    )
    .unwrap();
    static ref UUIDRegex: regex::Regex =
        regex::Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
            .unwrap();
}

#[derive(PartialEq, Eq, Debug)]
pub enum StringType {
    Unknown,
    IsoDate,
    IsoDateTime,
    UUID,
}

#[derive(PartialEq, Eq, Debug)]
pub enum SchemaState {
    Initial,
    Null,
    Nullable(Box<SchemaState>),
    String(StringType),
    Number {
        float: bool,
    },
    Boolean,
    Array(Box<SchemaState>),
    Object {
        required: std::collections::HashMap<String, SchemaState>,
        optional: std::collections::HashMap<String, SchemaState>,
    },
    Indefinite,
}

fn merge(initial: SchemaState, new: SchemaState) -> SchemaState {
    match (initial, new) {
        (SchemaState::Initial, SchemaState::Null) => SchemaState::Null,
        (SchemaState::Initial, SchemaState::String(x)) => SchemaState::String(x),
        (SchemaState::Initial, SchemaState::Boolean) => SchemaState::Boolean,
        (SchemaState::Initial, SchemaState::Number { float }) => SchemaState::Number { float },
        (SchemaState::Initial, SchemaState::Array(inner)) => SchemaState::Array(inner),
        (SchemaState::Initial, SchemaState::Object { required, optional }) => {
            SchemaState::Object { required, optional }
        }

        (SchemaState::String(first_type), SchemaState::String(second_type)) => {
            SchemaState::String(if first_type == second_type {
                first_type
            } else {
                StringType::Unknown
            })
        }

        (SchemaState::Number { float: true }, SchemaState::Number { float: _ }) => {
            SchemaState::Number { float: true }
        }
        (SchemaState::Number { float: _ }, SchemaState::Number { float: true }) => {
            SchemaState::Number { float: true }
        }
        (SchemaState::Number { float: false }, SchemaState::Number { float: false }) => {
            SchemaState::Number { float: false }
        }

        (SchemaState::Boolean, SchemaState::Boolean) => SchemaState::Boolean,

        (SchemaState::Array(first_schema), SchemaState::Array(second_schema)) => {
            SchemaState::Array(Box::new(merge(*first_schema, *second_schema)))
        }

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
                .iter()
                .map(|k| {
                    let first = first_required.remove(k);
                    let second = second_required.remove(k);
                    let merged = if first.is_some() && second.is_some() {
                        merge(first.unwrap(), second.unwrap())
                    } else {
                        first.unwrap_or(second.unwrap())
                    };
                    (k.clone(), merged)
                })
                .collect();

            let optional: std::collections::HashMap<String, SchemaState> = optional_keys
                .iter()
                .map(|k| {
                    let first = first_required
                        .remove(k)
                        .or_else(|| first_optional.remove(k));
                    let second = second_required
                        .remove(k)
                        .or_else(|| second_optional.remove(k));
                    let merged = if first.is_some() && second.is_some() {
                        merge(first.unwrap(), second.unwrap())
                    } else {
                        first.unwrap_or_else(|| second.unwrap())
                    };
                    (k.clone(), merged)
                })
                .collect();

            SchemaState::Object { required, optional }
        }

        (SchemaState::Null, SchemaState::Null) => SchemaState::Null,
        (SchemaState::Null, s) => SchemaState::Nullable(Box::new(s)),
        (s, SchemaState::Null) => SchemaState::Nullable(Box::new(s)),

        _ => SchemaState::Indefinite,
    }
}

fn infer_array_schema(values: &Vec<serde_json::Value>) -> SchemaState {
    values
        .iter()
        .map(infer_schema)
        .fold(SchemaState::Initial, merge)
}

pub fn infer_schema(json: &serde_json::Value) -> SchemaState {
    match json {
        serde_json::Value::Null => SchemaState::Null,
        serde_json::Value::String(value) => {
            let t = if IsoDateRegex.is_match(value) {
                StringType::IsoDate
            } else if IsoDateTimeRegex.is_match(value) {
                StringType::IsoDateTime
            } else if UUIDRegex.is_match(value) {
                StringType::UUID
            } else {
                StringType::Unknown
            };
            SchemaState::String(t)
        }
        serde_json::Value::Number(n) => SchemaState::Number { float: n.is_f64() },
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

        assert_eq!(schema, SchemaState::String(StringType::Unknown))
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

        assert_eq!(schema, SchemaState::Number { float: false })
    }

    #[test]
    fn infers_number_float() {
        let input = json!(42.0);
        let schema = infer_schema(&input);

        assert_eq!(schema, SchemaState::Number { float: true })
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
                        SchemaState::String(StringType::Unknown)
                    ),
                    ("int".to_string(), SchemaState::Number { float: false }),
                    ("float".to_string(), SchemaState::Number { float: true }),
                    ("bool".to_string(), SchemaState::Boolean),
                    (
                        "array".to_string(),
                        SchemaState::Array(Box::new(SchemaState::String(StringType::Unknown)))
                    ),
                    ("null".to_string(), SchemaState::Null),
                    (
                        "object".to_string(),
                        SchemaState::Object {
                            required: std::collections::HashMap::from_iter([(
                                "string".to_owned(),
                                SchemaState::String(StringType::Unknown)
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
        let input = json!(["foo", "bar"]);
        let schema = infer_schema(&input);

        assert_eq!(
            schema,
            SchemaState::Array(Box::new(SchemaState::String(StringType::Unknown)))
        );
    }

    #[test]
    fn infers_array_number() {
        let input = json!([100, 104]);
        let schema = infer_schema(&input);

        assert_eq!(
            schema,
            SchemaState::Array(Box::new(SchemaState::Number { float: false }))
        );
    }

    #[test]
    fn infers_array_number_float() {
        let input = json!([100, 104.5]);
        let schema = infer_schema(&input);

        assert_eq!(
            schema,
            SchemaState::Array(Box::new(SchemaState::Number { float: true }))
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
            }
        ]);
        let schema = infer_schema(&input);

        assert_eq!(
            schema,
            SchemaState::Array(Box::new(SchemaState::Object {
                required: std::collections::HashMap::from_iter([
                    (
                        "baz".to_owned(),
                        SchemaState::Nullable(Box::new(SchemaState::Number { float: false }))
                    ),
                    ("qux".to_owned(), SchemaState::Boolean),
                ]),
                optional: std::collections::HashMap::from_iter([(
                    "foo".to_owned(),
                    SchemaState::String(StringType::Unknown)
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
        let input = json!(["foo", null]);
        let schema = infer_schema(&input);

        assert_eq!(
            schema,
            SchemaState::Array(Box::new(SchemaState::Nullable(Box::new(
                SchemaState::String(StringType::Unknown)
            ))))
        );
    }
}
