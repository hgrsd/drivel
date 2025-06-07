use std::fmt::Display;

pub trait ToJsonSchema {
    fn to_json_schema(&self) -> serde_json::Value;

    fn to_json_schema_document(&self) -> serde_json::Value {
        let mut doc = serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": "https://example.com/schema",
            "title": "Inferred Schema",
            "description": "Schema inferred by drivel from sample data"
        });

        if let serde_json::Value::Object(schema_obj) = self.to_json_schema() {
            if let serde_json::Value::Object(doc_obj) = &mut doc {
                doc_obj.extend(schema_obj);
            }
        }

        doc
    }
}

#[derive(PartialEq, Debug)]
pub enum StringType {
    Unknown {
        strings_seen: Vec<String>,
        chars_seen: Vec<char>,
        min_length: Option<usize>,
        max_length: Option<usize>,
    },
    IsoDate,
    DateTimeRFC2822,
    DateTimeISO8601,
    UUID,
    Email,
    Url,
    Hostname,
    Enum {
        variants: std::collections::HashSet<String>,
    },
}

impl Display for StringType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            StringType::Unknown {
                strings_seen: _,
                chars_seen: _,
                min_length,
                max_length,
            } => {
                let length = match (min_length, max_length) {
                    (Some(min), Some(max)) => {
                        if min != max {
                            format!("({}-{})", min, max)
                        } else {
                            format!("({})", min)
                        }
                    }
                    (Some(min), None) => format!("({}-?)", min),
                    (None, Some(max)) => format!("(?-{})", max),
                    (None, None) => "(length unknown)".to_string(),
                };
                format!("string {}", length)
            }
            StringType::IsoDate => "string (date - ISO 8601)".to_owned(),
            StringType::DateTimeRFC2822 => "string (datetime - RFC 2822)".to_owned(),
            StringType::DateTimeISO8601 => "string (datetime - ISO 8601)".to_owned(),
            StringType::UUID => "string (uuid)".to_owned(),
            StringType::Email => "string (email)".to_owned(),
            StringType::Hostname => "string (hostname)".to_owned(),
            StringType::Url => "string (url)".to_owned(),
            StringType::Enum { variants } => {
                let variants_vec = variants.iter().cloned().collect::<Vec<_>>();
                let formatted = variants_vec.join(", ");
                format!("string (enum: {})", formatted)
            }
        };
        write!(f, "{}", text)
    }
}

#[derive(PartialEq, Debug)]
pub enum NumberType {
    Integer { min: i64, max: i64 },
    Float { min: f64, max: f64 },
}

impl Display for NumberType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            NumberType::Integer { min, max } => {
                if min != max {
                    format!("int ({}-{})", min, max)
                } else {
                    format!("int ({})", min)
                }
            }
            NumberType::Float { min, max } => {
                if min != max {
                    format!("float ({}-{})", min, max)
                } else {
                    format!("float ({})", min)
                }
            }
        };
        write!(f, "{}", text)
    }
}

/// The SchemaState enum is a recursive data structure that describes the schema of a given JSON structure.
///
/// There are a few notable differences with the data types from the JSON specification:
/// - The SchemaState enum has Initial and Indefinite variants. These encode two possible results of the
///   schema inference process that have no equivalents in the JSON specification.
/// - The String and Number types have an inner type that specialises the more generic types. This is to
///   add some further semantics to the data type, provided `drivel` is able to infer these semantics.
#[derive(PartialEq, Debug)]
pub enum SchemaState {
    /// Initial state.
    Initial,
    /// Represents a null value.
    Null,
    /// Represents a nullable value with an inner schema.
    Nullable(Box<SchemaState>),
    /// Represents a string value with specified string type.
    String(StringType),
    /// Represents a number value with specified number type.
    Number(NumberType),
    /// Represents a boolean value.
    Boolean,
    /// Represents an array with specified minimum and maximum lengths and a schema for its elements.
    Array {
        /// Minimum length of the array.
        min_length: usize,
        /// Maximum length of the array.
        max_length: usize,
        /// Schema for the elements of the array.
        schema: Box<SchemaState>,
    },
    /// Represents an object with required and optional fields and their corresponding schemas.
    Object {
        /// Required fields and their schemas.
        required: std::collections::HashMap<String, SchemaState>,
        /// Optional fields and their schemas.
        optional: std::collections::HashMap<String, SchemaState>,
    },
    /// Represents an indefinite state.
    Indefinite,
}

fn to_string_pretty_inner(schema_state: &SchemaState, depth: usize) -> String {
    match schema_state {
        SchemaState::Initial | SchemaState::Indefinite => "unknown".to_string(),
        SchemaState::Null => "null".to_string(),
        SchemaState::Nullable(state) => {
            format!("nullable {}", to_string_pretty_inner(state, depth))
        }
        SchemaState::String(string_type) => format!("{}", string_type),
        SchemaState::Number(number_type) => format!("{}", number_type),
        SchemaState::Boolean => "boolean".to_string(),
        SchemaState::Array {
            min_length,
            max_length,
            schema,
        } => {
            let indent = 2 + 2 * depth;
            let indent_str = " ".repeat(indent);
            let indent_str_close = " ".repeat(indent - 2);
            let length = if min_length != max_length {
                format!("({}-{})", min_length, max_length)
            } else {
                format!("({})", min_length)
            };
            format!(
                "[\n{}{}\n{}] {}",
                indent_str,
                to_string_pretty_inner(schema, depth + 1),
                indent_str_close,
                length
            )
        }
        SchemaState::Object { required, optional } => {
            let indent = 2 + 2 * depth;
            let indent_str = " ".repeat(indent);
            let indent_str_close = " ".repeat(indent - 2);
            let mut combined = String::new();
            for (k, v) in required {
                combined.push_str(
                    format!(
                        "{}\"{}\": {},\n",
                        indent_str,
                        k,
                        to_string_pretty_inner(v, depth + 1)
                    )
                    .as_str(),
                );
            }

            for (k, v) in optional {
                combined.push_str(
                    format!(
                        "{}\"{}\": optional {},\n",
                        indent_str,
                        k,
                        to_string_pretty_inner(v, depth + 1)
                    )
                    .as_str(),
                );
            }
            combined.pop(); // removes last \n
            combined.pop(); // removes trailing comma

            format!("{{\n{}\n{}}}", combined, indent_str_close)
        }
    }
}

impl SchemaState {
    /// Returns a formatted string representation of the schema state with indentation for readability.
    ///
    /// This method recursively traverses the schema state and constructs a formatted string representation
    /// with proper indentation to visually represent the hierarchical structure of the schema.
    ///
    /// # Examples
    ///
    /// ```
    /// use drivel::{SchemaState, StringType, NumberType};
    /// use std::collections::{HashMap, HashSet};
    /// use std::iter::FromIterator;
    ///
    /// let required = HashMap::from_iter(vec![
    ///     ("name".to_string(), SchemaState::String(StringType::Unknown {
    ///         strings_seen: vec!["abc".to_string()],
    ///         chars_seen: vec!['a', 'b', 'c'],
    ///         min_length: Some(1),
    ///         max_length: Some(10),
    ///     }))
    /// ]);
    ///
    /// let optional = HashMap::from_iter(vec![
    ///     ("age".to_string(), SchemaState::Number(NumberType::Integer { min: 0, max: 120 }))
    /// ]);
    ///
    /// let schema = SchemaState::Object {
    ///     required,
    ///     optional,
    /// };
    ///
    /// println!("{}", schema.to_string_pretty());
    /// ```
    ///
    /// Output:
    ///
    /// ```text
    /// {
    ///   "name": string (1-10),
    ///   "age": optional int (0-120)
    /// }
    /// ```
    pub fn to_string_pretty(&self) -> String {
        to_string_pretty_inner(self, 0)
    }
}

impl ToJsonSchema for SchemaState {
    fn to_json_schema(&self) -> serde_json::Value {
        match self {
            SchemaState::Boolean => serde_json::json!({ "type": "boolean" }),
            SchemaState::Null => serde_json::json!({ "type": "null" }),
            SchemaState::Initial | SchemaState::Indefinite => serde_json::json!({}),
            SchemaState::String(string_type) => string_type.to_json_schema(),
            SchemaState::Number(number_type) => number_type.to_json_schema(),
            SchemaState::Nullable(inner) => {
                let mut inner_schema = inner.to_json_schema();

                // Convert single type to array with null
                if let Some(type_value) = inner_schema.get("type") {
                    if let Some(type_str) = type_value.as_str() {
                        inner_schema["type"] = serde_json::json!([type_str, "null"]);
                    }
                }

                inner_schema
            }
            SchemaState::Array {
                min_length,
                max_length,
                schema,
            } => {
                serde_json::json!({
                    "type": "array",
                    "items": schema.to_json_schema(),
                    "minItems": min_length,
                    "maxItems": max_length
                })
            }
            SchemaState::Object { required, optional } => {
                let mut properties = serde_json::Map::new();
                let mut required_fields = Vec::new();

                // Add required fields
                for (key, schema) in required {
                    properties.insert(key.clone(), schema.to_json_schema());
                    required_fields.push(key.clone());
                }

                // Add optional fields
                for (key, schema) in optional {
                    properties.insert(key.clone(), schema.to_json_schema());
                }

                serde_json::json!({
                    "type": "object",
                    "properties": properties,
                    "required": required_fields,
                    "additionalProperties": false
                })
            }
        }
    }
}

impl ToJsonSchema for StringType {
    fn to_json_schema(&self) -> serde_json::Value {
        match self {
            StringType::Unknown {
                min_length,
                max_length,
                ..
            } => {
                let mut schema = serde_json::json!({ "type": "string" });
                if let Some(min) = min_length {
                    schema["minLength"] = serde_json::Value::Number((*min).into());
                }
                if let Some(max) = max_length {
                    schema["maxLength"] = serde_json::Value::Number((*max).into());
                }
                schema
            }
            StringType::UUID => serde_json::json!({
                "type": "string",
                "format": "uuid"
            }),
            StringType::Email => serde_json::json!({
                "type": "string",
                "format": "email"
            }),
            StringType::Url => serde_json::json!({
                "type": "string",
                "format": "uri"
            }),
            StringType::IsoDate => serde_json::json!({
                "type": "string",
                "format": "date"
            }),
            StringType::DateTimeISO8601 => serde_json::json!({
                "type": "string",
                "format": "date-time"
            }),
            StringType::Hostname => serde_json::json!({
                "type": "string",
                "format": "hostname",
                "x-drivel-type": "hostname"
            }),
            StringType::DateTimeRFC2822 => serde_json::json!({
                "type": "string",
                "x-drivel-type": "datetime-rfc2822",
                "description": "RFC 2822 datetime format"
            }),
            StringType::Enum { variants } => {
                let enum_values: Vec<&String> = variants.iter().collect();
                serde_json::json!({
                    "type": "string",
                    "enum": enum_values
                })
            }
        }
    }
}

impl ToJsonSchema for NumberType {
    fn to_json_schema(&self) -> serde_json::Value {
        match self {
            NumberType::Integer { min, max } => serde_json::json!({
                "type": "integer",
                "minimum": min,
                "maximum": max
            }),
            NumberType::Float { min, max } => serde_json::json!({
                "type": "number",
                "minimum": min,
                "maximum": max
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn basic_types_to_json_schema() {
        // Test Boolean
        let boolean_schema = SchemaState::Boolean;
        assert_eq!(
            boolean_schema.to_json_schema(),
            json!({ "type": "boolean" })
        );

        // Test Null
        let null_schema = SchemaState::Null;
        assert_eq!(null_schema.to_json_schema(), json!({ "type": "null" }));

        // Test Initial state (should be empty schema)
        let initial_schema = SchemaState::Initial;
        assert_eq!(initial_schema.to_json_schema(), json!({}));

        // Test Indefinite state (should be empty schema)
        let indefinite_schema = SchemaState::Indefinite;
        assert_eq!(indefinite_schema.to_json_schema(), json!({}));
    }

    #[test]
    fn json_schema_document_format() {
        let boolean_schema = SchemaState::Boolean;
        let document = boolean_schema.to_json_schema_document();

        assert_eq!(
            document["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
        assert_eq!(document["$id"], "https://example.com/schema");
        assert_eq!(document["title"], "Inferred Schema");
        assert_eq!(
            document["description"],
            "Schema inferred by drivel from sample data"
        );
        assert_eq!(document["type"], "boolean");
    }


    #[test]
    fn string_types_to_json_schema() {
        // Test basic string with length constraints
        let unknown_string = SchemaState::String(StringType::Unknown {
            strings_seen: vec!["test".to_string()],
            chars_seen: vec!['t', 'e', 's', 't'],
            min_length: Some(3),
            max_length: Some(10),
        });
        assert_eq!(
            unknown_string.to_json_schema(),
            json!({
                "type": "string",
                "minLength": 3,
                "maxLength": 10
            })
        );

        // Test UUID
        let uuid_string = SchemaState::String(StringType::UUID);
        assert_eq!(
            uuid_string.to_json_schema(),
            json!({
                "type": "string",
                "format": "uuid"
            })
        );

        // Test Email
        let email_string = SchemaState::String(StringType::Email);
        assert_eq!(
            email_string.to_json_schema(),
            json!({
                "type": "string",
                "format": "email"
            })
        );
    }

    #[test]
    fn string_no_length_constraints_to_json_schema() {
        let no_constraints = SchemaState::String(StringType::Unknown {
            strings_seen: vec!["test".to_string()],
            chars_seen: vec!['t', 'e', 's', 't'],
            min_length: None,
            max_length: None,
        });
        assert_eq!(no_constraints.to_json_schema(), json!({ "type": "string" }));
    }

    #[test]
    fn string_min_length_only_to_json_schema() {
        let min_only = SchemaState::String(StringType::Unknown {
            strings_seen: vec!["test".to_string()],
            chars_seen: vec!['t', 'e', 's', 't'],
            min_length: Some(5),
            max_length: None,
        });
        assert_eq!(
            min_only.to_json_schema(),
            json!({
                "type": "string",
                "minLength": 5
            })
        );
    }

    #[test]
    fn string_max_length_only_to_json_schema() {
        let max_only = SchemaState::String(StringType::Unknown {
            strings_seen: vec!["test".to_string()],
            chars_seen: vec!['t', 'e', 's', 't'],
            min_length: None,
            max_length: Some(20),
        });
        assert_eq!(
            max_only.to_json_schema(),
            json!({
                "type": "string",
                "maxLength": 20
            })
        );
    }

    #[test]
    fn string_format_types_to_json_schema() {
        assert_eq!(
            SchemaState::String(StringType::Url).to_json_schema(),
            json!({ "type": "string", "format": "uri" })
        );

        assert_eq!(
            SchemaState::String(StringType::IsoDate).to_json_schema(),
            json!({ "type": "string", "format": "date" })
        );

        assert_eq!(
            SchemaState::String(StringType::DateTimeISO8601).to_json_schema(),
            json!({ "type": "string", "format": "date-time" })
        );

        assert_eq!(
            SchemaState::String(StringType::Hostname).to_json_schema(),
            json!({
                "type": "string",
                "format": "hostname",
                "x-drivel-type": "hostname"
            })
        );

        assert_eq!(
            SchemaState::String(StringType::DateTimeRFC2822).to_json_schema(),
            json!({
                "type": "string",
                "x-drivel-type": "datetime-rfc2822",
                "description": "RFC 2822 datetime format"
            })
        );
    }

    #[test]
    fn string_enum_multiple_variants_to_json_schema() {
        let mut enum_variants = std::collections::HashSet::new();
        enum_variants.insert("red".to_string());
        enum_variants.insert("green".to_string());
        enum_variants.insert("blue".to_string());

        let enum_string = SchemaState::String(StringType::Enum {
            variants: enum_variants,
        });
        let result = enum_string.to_json_schema();
        assert_eq!(result["type"], "string");
        assert!(result["enum"].is_array());
        let enum_values = result["enum"].as_array().unwrap();
        assert_eq!(enum_values.len(), 3);
    }

    #[test]
    fn string_enum_empty_variants_to_json_schema() {
        let empty_enum = SchemaState::String(StringType::Enum {
            variants: std::collections::HashSet::new(),
        });
        let result = empty_enum.to_json_schema();
        assert_eq!(result["type"], "string");
        assert_eq!(result["enum"], json!([]));
    }

    #[test]
    fn string_enum_single_variant_to_json_schema() {
        let mut single_variant = std::collections::HashSet::new();
        single_variant.insert("only".to_string());
        let single_enum = SchemaState::String(StringType::Enum {
            variants: single_variant,
        });
        let result = single_enum.to_json_schema();
        assert_eq!(result["type"], "string");
        assert_eq!(result["enum"], json!(["only"]));
    }

    #[test]
    fn number_types_to_json_schema() {
        // Test integer
        let integer = SchemaState::Number(NumberType::Integer { min: 1, max: 100 });
        assert_eq!(
            integer.to_json_schema(),
            json!({
                "type": "integer",
                "minimum": 1,
                "maximum": 100
            })
        );

        // Test float
        let float = SchemaState::Number(NumberType::Float {
            min: 1.5,
            max: 99.9,
        });
        assert_eq!(
            float.to_json_schema(),
            json!({
                "type": "number",
                "minimum": 1.5,
                "maximum": 99.9
            })
        );
    }

    #[test]
    fn number_integer_single_value_to_json_schema() {
        let single_value_int = SchemaState::Number(NumberType::Integer { min: 42, max: 42 });
        assert_eq!(
            single_value_int.to_json_schema(),
            json!({
                "type": "integer",
                "minimum": 42,
                "maximum": 42
            })
        );
    }

    #[test]
    fn number_integer_negative_range_to_json_schema() {
        let negative_int = SchemaState::Number(NumberType::Integer {
            min: -100,
            max: -10,
        });
        assert_eq!(
            negative_int.to_json_schema(),
            json!({
                "type": "integer",
                "minimum": -100,
                "maximum": -10
            })
        );
    }

    #[test]
    fn number_integer_zero_range_to_json_schema() {
        let zero_range = SchemaState::Number(NumberType::Integer { min: 0, max: 0 });
        assert_eq!(
            zero_range.to_json_schema(),
            json!({
                "type": "integer",
                "minimum": 0,
                "maximum": 0
            })
        );
    }

    #[test]
    fn number_float_single_value_to_json_schema() {
        let single_value_float = SchemaState::Number(NumberType::Float {
            min: 3.14,
            max: 3.14,
        });
        assert_eq!(
            single_value_float.to_json_schema(),
            json!({
                "type": "number",
                "minimum": 3.14,
                "maximum": 3.14
            })
        );
    }

    #[test]
    fn number_float_negative_range_to_json_schema() {
        let negative_float = SchemaState::Number(NumberType::Float {
            min: -99.9,
            max: -1.1,
        });
        assert_eq!(
            negative_float.to_json_schema(),
            json!({
                "type": "number",
                "minimum": -99.9,
                "maximum": -1.1
            })
        );
    }

    #[test]
    fn nullable_types_to_json_schema() {
        // Test nullable string
        let nullable_string =
            SchemaState::Nullable(Box::new(SchemaState::String(StringType::UUID)));
        assert_eq!(
            nullable_string.to_json_schema(),
            json!({
                "type": ["string", "null"],
                "format": "uuid"
            })
        );

        // Test nullable integer
        let nullable_int =
            SchemaState::Nullable(Box::new(SchemaState::Number(NumberType::Integer {
                min: 1,
                max: 10,
            })));
        assert_eq!(
            nullable_int.to_json_schema(),
            json!({
                "type": ["integer", "null"],
                "minimum": 1,
                "maximum": 10
            })
        );
    }

    #[test]
    fn nullable_array_to_json_schema() {
        let nullable_array = SchemaState::Nullable(Box::new(SchemaState::Array {
            min_length: 1,
            max_length: 3,
            schema: Box::new(SchemaState::String(StringType::Email)),
        }));
        assert_eq!(
            nullable_array.to_json_schema(),
            json!({
                "type": ["array", "null"],
                "items": {
                    "type": "string",
                    "format": "email"
                },
                "minItems": 1,
                "maxItems": 3
            })
        );
    }

    #[test]
    fn nullable_object_to_json_schema() {
        use std::collections::HashMap;
        let mut required = HashMap::new();
        required.insert(
            "id".to_string(),
            SchemaState::Number(NumberType::Integer { min: 1, max: 100 }),
        );

        let nullable_object = SchemaState::Nullable(Box::new(SchemaState::Object {
            required,
            optional: HashMap::new(),
        }));
        let result = nullable_object.to_json_schema();
        assert_eq!(result["type"], json!(["object", "null"]));
        assert_eq!(result["properties"]["id"]["type"], "integer");
        assert_eq!(result["required"], json!(["id"]));
        assert_eq!(result["additionalProperties"], false);
    }

    #[test]
    fn array_types_to_json_schema() {
        let array_schema = SchemaState::Array {
            min_length: 1,
            max_length: 5,
            schema: Box::new(SchemaState::String(StringType::UUID)),
        };
        assert_eq!(
            array_schema.to_json_schema(),
            json!({
                "type": "array",
                "items": {
                    "type": "string",
                    "format": "uuid"
                },
                "minItems": 1,
                "maxItems": 5
            })
        );
    }

    #[test]
    fn array_empty_to_json_schema() {
        let empty_array = SchemaState::Array {
            min_length: 0,
            max_length: 0,
            schema: Box::new(SchemaState::Boolean),
        };
        assert_eq!(
            empty_array.to_json_schema(),
            json!({
                "type": "array",
                "items": { "type": "boolean" },
                "minItems": 0,
                "maxItems": 0
            })
        );
    }

    #[test]
    fn array_of_objects_to_json_schema() {
        use std::collections::HashMap;
        let mut required = HashMap::new();
        required.insert(
            "name".to_string(),
            SchemaState::String(StringType::Unknown {
                strings_seen: vec!["test".to_string()],
                chars_seen: vec!['t', 'e', 's', 't'],
                min_length: Some(1),
                max_length: Some(50),
            }),
        );

        let array_of_objects = SchemaState::Array {
            min_length: 1,
            max_length: 10,
            schema: Box::new(SchemaState::Object {
                required,
                optional: HashMap::new(),
            }),
        };
        let result = array_of_objects.to_json_schema();
        assert_eq!(result["type"], "array");
        assert_eq!(result["minItems"], 1);
        assert_eq!(result["maxItems"], 10);
        assert_eq!(result["items"]["type"], "object");
        assert_eq!(result["items"]["properties"]["name"]["type"], "string");
    }

    #[test]
    fn array_nested_to_json_schema() {
        let nested_array = SchemaState::Array {
            min_length: 1,
            max_length: 3,
            schema: Box::new(SchemaState::Array {
                min_length: 2,
                max_length: 4,
                schema: Box::new(SchemaState::Number(NumberType::Integer {
                    min: 1,
                    max: 100,
                })),
            }),
        };
        let result = nested_array.to_json_schema();
        assert_eq!(result["type"], "array");
        assert_eq!(result["items"]["type"], "array");
        assert_eq!(result["items"]["items"]["type"], "integer");
        assert_eq!(result["minItems"], 1);
        assert_eq!(result["maxItems"], 3);
        assert_eq!(result["items"]["minItems"], 2);
        assert_eq!(result["items"]["maxItems"], 4);
    }

    #[test]
    fn object_types_to_json_schema() {
        use std::collections::HashMap;

        let mut required = HashMap::new();
        required.insert(
            "id".to_string(),
            SchemaState::Number(NumberType::Integer { min: 1, max: 1000 }),
        );

        let mut optional = HashMap::new();
        optional.insert(
            "name".to_string(),
            SchemaState::String(StringType::Unknown {
                strings_seen: vec!["test".to_string()],
                chars_seen: vec!['t', 'e', 's', 't'],
                min_length: Some(1),
                max_length: Some(50),
            }),
        );

        let object_schema = SchemaState::Object { required, optional };
        let result = object_schema.to_json_schema();

        assert_eq!(result["type"], "object");
        assert_eq!(result["additionalProperties"], false);
        assert_eq!(result["required"], json!(["id"]));
        assert_eq!(result["properties"]["id"]["type"], "integer");
        assert_eq!(result["properties"]["name"]["type"], "string");
    }

    #[test]
    fn object_empty_to_json_schema() {
        use std::collections::HashMap;
        let empty_object = SchemaState::Object {
            required: HashMap::new(),
            optional: HashMap::new(),
        };
        let result = empty_object.to_json_schema();
        assert_eq!(result["type"], "object");
        assert_eq!(result["additionalProperties"], false);
        assert_eq!(result["required"], json!([]));
        assert_eq!(result["properties"], json!({}));
    }

    #[test]
    fn object_required_only_to_json_schema() {
        use std::collections::HashMap;
        let mut required = HashMap::new();
        required.insert(
            "id".to_string(),
            SchemaState::Number(NumberType::Integer { min: 1, max: 100 }),
        );
        required.insert("status".to_string(), SchemaState::Boolean);

        let required_only_object = SchemaState::Object {
            required,
            optional: HashMap::new(),
        };
        let result = required_only_object.to_json_schema();
        assert_eq!(result["type"], "object");
        assert_eq!(result["required"].as_array().unwrap().len(), 2);
        assert!(result["required"]
            .as_array()
            .unwrap()
            .contains(&json!("id")));
        assert!(result["required"]
            .as_array()
            .unwrap()
            .contains(&json!("status")));
    }

    #[test]
    fn object_optional_only_to_json_schema() {
        use std::collections::HashMap;
        let mut optional = HashMap::new();
        optional.insert(
            "description".to_string(),
            SchemaState::String(StringType::Unknown {
                strings_seen: vec!["test".to_string()],
                chars_seen: vec!['t', 'e', 's', 't'],
                min_length: None,
                max_length: None,
            }),
        );
        optional.insert(
            "count".to_string(),
            SchemaState::Number(NumberType::Integer { min: 0, max: 10 }),
        );

        let optional_only_object = SchemaState::Object {
            required: HashMap::new(),
            optional,
        };
        let result = optional_only_object.to_json_schema();
        assert_eq!(result["type"], "object");
        assert_eq!(result["required"], json!([]));
        assert_eq!(result["properties"]["description"]["type"], "string");
        assert_eq!(result["properties"]["count"]["type"], "integer");
    }

    #[test]
    fn object_nested_to_json_schema() {
        use std::collections::HashMap;
        let mut inner_required = HashMap::new();
        inner_required.insert(
            "nested_id".to_string(),
            SchemaState::Number(NumberType::Integer { min: 1, max: 10 }),
        );

        let mut outer_required = HashMap::new();
        outer_required.insert(
            "inner".to_string(),
            SchemaState::Object {
                required: inner_required,
                optional: HashMap::new(),
            },
        );

        let nested_object = SchemaState::Object {
            required: outer_required,
            optional: HashMap::new(),
        };
        let result = nested_object.to_json_schema();
        assert_eq!(result["type"], "object");
        assert_eq!(result["properties"]["inner"]["type"], "object");
        assert_eq!(
            result["properties"]["inner"]["properties"]["nested_id"]["type"],
            "integer"
        );
        assert_eq!(
            result["properties"]["inner"]["required"],
            json!(["nested_id"])
        );
    }
}
