use serde::Serialize;

#[derive(PartialEq, Eq, Debug, Serialize)]
pub enum SchemaState {
    Initial,
    Null,
    Nullable(Box<SchemaState>),
    String,
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
        (SchemaState::Initial, SchemaState::String) => SchemaState::String,
        (SchemaState::Initial, SchemaState::Boolean) => SchemaState::Boolean,
        (SchemaState::Initial, SchemaState::Number { float }) => SchemaState::Number { float },
        (SchemaState::Initial, SchemaState::Array(inner)) => SchemaState::Array(inner),
        (SchemaState::Initial, SchemaState::Object { required, optional }) => {
            SchemaState::Object { required, optional }
        }

        (SchemaState::String, SchemaState::String) => SchemaState::String,

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
                optional: first_optional,
            },
            SchemaState::Object {
                required: mut second_required,
                optional: second_optional,
            },
        ) => {
            let required_keys: std::collections::HashSet<String> = first_required
                .keys()
                .filter(|k| second_required.contains_key(*k))
                .cloned()
                .collect();

            let optional_keys: Vec<String> = first_optional
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
                    let first = first_required.remove(k);
                    let second = second_required.remove(k);
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
        serde_json::Value::String(_) => SchemaState::String,
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
    fn infers_string() {
        let input = json!("foo");
        let schema = infer_schema(&input);

        assert_eq!(schema, SchemaState::String)
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
                    ("string".to_string(), SchemaState::String),
                    ("int".to_string(), SchemaState::Number { float: false }),
                    ("float".to_string(), SchemaState::Number { float: true }),
                    ("bool".to_string(), SchemaState::Boolean),
                    (
                        "array".to_string(),
                        SchemaState::Array(Box::new(SchemaState::String))
                    ),
                    ("null".to_string(), SchemaState::Null),
                    (
                        "object".to_string(),
                        SchemaState::Object {
                            required: std::collections::HashMap::from_iter([(
                                "string".to_owned(),
                                SchemaState::String
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

        assert_eq!(schema, SchemaState::Array(Box::new(SchemaState::String)));
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
                    SchemaState::String
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
                SchemaState::String
            ))))
        );
    }
}
