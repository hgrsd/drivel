use std::fmt::Display;

/// A trait for converting schema types to JSON Schema format.
///
/// This trait provides methods to convert drivel's internal schema representation
/// to standard JSON Schema documents that conform to the JSON Schema specification.
/// The generated schemas can be used for validation, documentation, or integration
/// with other systems that understand JSON Schema.
///
/// # JSON Schema Specification
///
/// The generated schemas conform to JSON Schema Draft 2020-12 specification.
/// See: https://json-schema.org/draft/2020-12/schema
///
/// # Examples
///
/// ## Basic usage with primitive types
///
/// ```
/// use drivel::{ToJsonSchema, SchemaState, StringType, NumberType};
/// use serde_json::json;
///
/// // String schema with length constraints
/// let string_schema = SchemaState::String(StringType::Unknown {
///     strings_seen: vec!["example".to_string()],
///     chars_seen: vec!['e', 'x', 'a', 'm', 'p', 'l', 'e'],
///     min_length: Some(3),
///     max_length: Some(50),
/// });
///
/// let json_schema = string_schema.to_json_schema();
/// assert_eq!(json_schema, json!({
///     "type": "string"
/// }));
///
/// // Number schema with range constraints
/// let number_schema = SchemaState::Number(NumberType::Integer { min: 1, max: 100 });
/// let json_schema = number_schema.to_json_schema();
/// assert_eq!(json_schema, json!({
///     "type": "integer"
/// }));
/// ```
///
/// ## Complete JSON Schema document
///
/// ```
/// use drivel::{ToJsonSchema, SchemaState};
/// use serde_json::json;
///
/// let schema = SchemaState::Boolean;
/// let document = schema.to_json_schema_document();
///
/// assert_eq!(document["$schema"], "https://json-schema.org/draft/2020-12/schema");
/// assert_eq!(document["title"], "Inferred Schema");
/// assert_eq!(document["type"], "boolean");
/// ```
///
/// ## Complex schemas with objects and arrays
///
/// ```
/// use drivel::{ToJsonSchema, SchemaState, StringType, NumberType};
/// use std::collections::HashMap;
/// use serde_json::json;
///
/// let mut required = HashMap::new();
/// required.insert("id".to_string(), SchemaState::Number(NumberType::Integer { min: 1, max: 1000 }));
/// required.insert("name".to_string(), SchemaState::String(StringType::Unknown {
///     strings_seen: vec!["test".to_string()],
///     chars_seen: vec!['t', 'e', 's', 't'],
///     min_length: Some(1),
///     max_length: Some(100),
/// }));
///
/// let object_schema = SchemaState::Object {
///     required,
///     optional: HashMap::new(),
/// };
///
/// let json_schema = object_schema.to_json_schema();
/// assert_eq!(json_schema["type"], "object");
/// assert!(json_schema["required"].as_array().unwrap().contains(&json!("id")));
/// assert!(json_schema["required"].as_array().unwrap().contains(&json!("name")));
/// ```
pub trait ToJsonSchema {
    /// Converts the schema to a JSON Schema value.
    ///
    /// This method returns a `serde_json::Value` representing the JSON Schema
    /// for this type. The returned value contains the core schema definition
    /// without the document metadata (like `$schema`, `title`, etc.).
    ///
    /// # Returns
    ///
    /// A `serde_json::Value` containing the JSON Schema representation.
    /// For primitive types, this includes type information and constraints.
    /// For complex types like objects and arrays, this includes nested schemas
    /// and structural information.
    ///
    /// # Examples
    ///
    /// ```
    /// use drivel::{ToJsonSchema, SchemaState, StringType};
    /// use serde_json::json;
    ///
    /// let schema = SchemaState::String(StringType::Email);
    /// let json_schema = schema.to_json_schema();
    ///
    /// assert_eq!(json_schema, json!({
    ///     "type": "string",
    ///     "format": "email"
    /// }));
    /// ```
    fn to_json_schema(&self) -> serde_json::Value;

    /// Converts the schema to a complete JSON Schema document.
    ///
    /// This method wraps the core schema from `to_json_schema()` in a complete
    /// JSON Schema document with proper metadata including `$schema`, `title`,
    /// and `description` fields. The result is a self-contained JSON Schema
    /// document that can be used for validation or published as API documentation.
    ///
    /// # Returns
    ///
    /// A `serde_json::Value` containing a complete JSON Schema document
    /// conforming to JSON Schema Draft 2020-12 specification.
    ///
    /// # Document Structure
    ///
    /// The generated document includes:
    /// - `$schema`: References JSON Schema Draft 2020-12
    /// - `title`: Set to "Inferred Schema"
    /// - `description`: Indicates the schema was inferred by drivel
    /// - All properties from the core schema
    ///
    /// # Examples
    ///
    /// ```
    /// use drivel::{ToJsonSchema, SchemaState, StringType};
    /// use serde_json::json;
    ///
    /// let schema = SchemaState::String(StringType::UUID);
    /// let document = schema.to_json_schema_document();
    ///
    /// assert_eq!(document["$schema"], "https://json-schema.org/draft/2020-12/schema");
    /// assert_eq!(document["title"], "Inferred Schema");
    /// assert_eq!(document["description"], "Schema inferred by drivel from sample data");
    /// assert_eq!(document["type"], "string");
    /// assert_eq!(document["format"], "uuid");
    /// ```
    fn to_json_schema_document(&self) -> serde_json::Value {
        let mut doc = serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
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

/// Represents specialized string types that can be inferred from sample data.
///
/// This enum captures semantic information about strings beyond the basic string type,
/// allowing for more precise schema generation and validation. The inference engine
/// analyzes string patterns to determine the most specific type that applies.
///
/// # Type Inference Strategy
///
/// String types are inferred in order of specificity:
/// 1. **Specialized formats** (UUID, Email, URL, etc.) - checked first
/// 2. **Date/time formats** - ISO dates and RFC formats
/// 3. **Enum detection** - when all values match a small set of distinct strings
/// 4. **Unknown** - fallback for general strings with length constraints
///
/// # Examples
///
/// ```
/// use drivel::{StringType, ToJsonSchema};
/// use serde_json::json;
/// use std::collections::HashSet;
///
/// // UUID detection
/// let uuid_type = StringType::UUID;
/// assert_eq!(uuid_type.to_json_schema(), json!({
///     "type": "string",
///     "format": "uuid"
/// }));
///
/// // Email detection
/// let email_type = StringType::Email;
/// assert_eq!(email_type.to_json_schema(), json!({
///     "type": "string",
///     "format": "email"
/// }));
///
/// // Enum detection
/// let enum_variants = ["red", "green", "blue"]
///     .iter()
///     .map(|s| s.to_string())
///     .collect::<HashSet<_>>();
/// let enum_type = StringType::Enum { variants: enum_variants };
/// let schema = enum_type.to_json_schema();
/// assert_eq!(schema["type"], "string");
/// assert!(schema["enum"].is_array());
/// ```
#[derive(PartialEq, Debug)]
pub enum StringType {
    /// General string type with length constraints and character analysis.
    ///
    /// This variant is used when no specific string format can be detected.
    /// It maintains tracking information for potential pattern detection and
    /// length constraints derived from the sample data.
    ///
    /// # Fields
    ///
    /// * `strings_seen` - Sample strings encountered during inference
    /// * `chars_seen` - Individual characters found across all samples
    /// * `min_length` - Minimum string length observed (None if no data)
    /// * `max_length` - Maximum string length observed (None if no data)
    ///
    /// # JSON Schema Output
    ///
    /// Generates a JSON Schema with `type: "string"` and optional `minLength`/`maxLength` constraints.
    Unknown {
        strings_seen: Vec<String>,
        chars_seen: Vec<char>,
        min_length: Option<usize>,
        max_length: Option<usize>,
    },

    /// ISO 8601 date format (YYYY-MM-DD).
    ///
    /// Detected when strings match the ISO 8601 date pattern.
    /// Maps to JSON Schema `format: "date"`.
    ///
    /// # Examples
    /// - "2023-12-25"
    /// - "1995-01-01"
    IsoDate,

    /// RFC 2822 datetime format.
    ///
    /// Detected when strings match RFC 2822 datetime patterns.
    /// Uses custom extension `x-drivel-type: "datetime-rfc2822"` since
    /// this format is not part of standard JSON Schema formats.
    ///
    /// # Examples
    /// - "Mon, 25 Dec 2023 10:30:00 +0000"
    /// - "Fri, 01 Jan 1995 12:00:00 GMT"
    DateTimeRFC2822,

    /// ISO 8601 datetime format with timezone information.
    ///
    /// Detected when strings match ISO 8601 datetime patterns.
    /// Maps to JSON Schema `format: "date-time"`.
    ///
    /// # Examples
    /// - "2023-12-25T10:30:00Z"
    /// - "1995-01-01T12:00:00+02:00"
    DateTimeISO8601,

    /// Universally Unique Identifier format.
    ///
    /// Detected when strings match UUID patterns (with or without hyphens).
    /// Maps to JSON Schema `format: "uuid"`.
    ///
    /// # Examples
    /// - "550e8400-e29b-41d4-a716-446655440000"
    /// - "6ba7b8109dad11d180b400c04fd430c8"
    UUID,

    /// Email address format.
    ///
    /// Detected when strings match email address patterns.
    /// Maps to JSON Schema `format: "email"`.
    ///
    /// # Examples
    /// - "user@example.com"
    /// - "test.email+tag@domain.org"
    Email,

    /// URL/URI format.
    ///
    /// Detected when strings match URL patterns with valid schemes.
    /// Maps to JSON Schema `format: "uri"`.
    ///
    /// # Examples
    /// - "https://example.com/path"
    /// - "ftp://files.example.com/document.pdf"
    Url,

    /// Hostname format.
    ///
    /// Detected when strings match hostname patterns.
    /// Maps to JSON Schema `format: "hostname"` with custom extension
    /// `x-drivel-type: "hostname"`.
    ///
    /// # Examples
    /// - "example.com"
    /// - "api.service.internal"
    Hostname,

    /// Enumerated string values with a finite set of variants.
    ///
    /// Detected when all observed string values belong to a small,
    /// distinct set that suggests enumeration rather than free-form text.
    /// The detection algorithm considers both the number of unique values
    /// and the ratio of unique values to total samples.
    ///
    /// # Fields
    ///
    /// * `variants` - Set of all distinct string values observed
    ///
    /// # JSON Schema Output
    ///
    /// Generates a JSON Schema with `type: "string"` and an `enum` array
    /// containing all observed variants.
    ///
    /// # Detection Criteria
    ///
    /// Strings are classified as enum when:
    /// - The number of unique values is small relative to total samples
    /// - The uniqueness ratio suggests a closed set rather than open text
    /// - Sample size is sufficient to make a confident determination
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

/// Represents numeric types with range constraints inferred from sample data.
///
/// This enum distinguishes between integer and floating-point numbers,
/// maintaining range information derived from the observed sample values.
/// The inference engine automatically determines the most appropriate
/// numeric type based on the presence of decimal points and fractional parts.
///
/// # Type Detection Strategy
///
/// Numbers are classified as:
/// 1. **Integer** - when all observed values are whole numbers
/// 2. **Float** - when any observed value contains a decimal point or fractional part
///
/// Range constraints (min/max) are determined from the actual values
/// encountered during schema inference, providing tight bounds for validation.
///
/// # JSON Schema Mapping
///
/// - `Integer` maps to JSON Schema `type: "integer"`
/// - `Float` maps to JSON Schema `type: "number"`
///
/// Both types include `minimum` and `maximum` constraints derived from sample data.
///
/// # Examples
///
/// ```
/// use drivel::{NumberType, ToJsonSchema};
/// use serde_json::json;
///
/// // Integer with range constraints
/// let int_type = NumberType::Integer { min: 1, max: 100 };
/// assert_eq!(int_type.to_json_schema(), json!({
///     "type": "integer"
/// }));
///
/// // Float with range constraints
/// let float_type = NumberType::Float { min: 0.5, max: 99.9 };
/// assert_eq!(float_type.to_json_schema(), json!({
///     "type": "number"
/// }));
/// ```
#[derive(PartialEq, Debug)]
pub enum NumberType {
    /// Integer numbers with minimum and maximum value constraints.
    ///
    /// This variant represents whole numbers (positive, negative, or zero)
    /// with range bounds derived from the observed sample data.
    /// All values in the sample set must be integers for this type to be selected.
    ///
    /// # Fields
    ///
    /// * `min` - Minimum integer value observed in the sample data
    /// * `max` - Maximum integer value observed in the sample data
    ///
    /// # JSON Schema Output
    ///
    /// Generates a JSON Schema with:
    /// - `type: "integer"`
    /// - `minimum: <min_value>`
    /// - `maximum: <max_value>`
    ///
    /// # Range Behavior
    ///
    /// When min equals max, the schema constrains the value to a single allowed integer.
    /// This effectively creates a constant value constraint.
    Integer { min: i64, max: i64 },

    /// Floating-point numbers with minimum and maximum value constraints.
    ///
    /// This variant represents decimal numbers with range bounds derived
    /// from the observed sample data. If any value in the sample contains
    /// a decimal point or fractional part, the entire field is classified as Float.
    ///
    /// # Fields
    ///
    /// * `min` - Minimum floating-point value observed in the sample data
    /// * `max` - Maximum floating-point value observed in the sample data
    ///
    /// # JSON Schema Output
    ///
    /// Generates a JSON Schema with:
    /// - `type: "number"`
    /// - `minimum: <min_value>`
    /// - `maximum: <max_value>`
    ///
    /// # Range Behavior
    ///
    /// When min equals max, the schema constrains the value to a single allowed number.
    /// This effectively creates a constant value constraint.
    ///
    /// # Precision Note
    ///
    /// Float ranges use f64 precision. Be aware of potential floating-point
    /// precision issues when working with very large numbers or high-precision decimals.
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

            let required_fields = required.iter().map(|(k, v)| {
                format!(
                    "{}\"{}\": {}",
                    indent_str,
                    k,
                    to_string_pretty_inner(v, depth + 1)
                )
            });

            let optional_fields = optional.iter().map(|(k, v)| {
                format!(
                    "{}\"{}\": optional {}",
                    indent_str,
                    k,
                    to_string_pretty_inner(v, depth + 1)
                )
            });

            let all_fields: Vec<String> = required_fields.chain(optional_fields).collect();
            let combined = all_fields.join(",\n");

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
            SchemaState::Array { schema, .. } => {
                serde_json::json!({
                    "type": "array",
                    "items": schema.to_json_schema()
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
            StringType::Unknown { .. } => {
                serde_json::json!({ "type": "string" })
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
            NumberType::Integer { .. } => serde_json::json!({
                "type": "integer"
            }),
            NumberType::Float { .. } => serde_json::json!({
                "type": "number"
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    mod test_helpers {
        use super::*;
        use std::collections::{HashMap, HashSet};

        pub fn unknown_string(min_length: Option<usize>, max_length: Option<usize>) -> StringType {
            StringType::Unknown {
                strings_seen: vec!["test".to_string()],
                chars_seen: vec!['t', 'e', 's', 't'],
                min_length,
                max_length,
            }
        }

        pub fn enum_string(variants: Vec<&str>) -> StringType {
            let variant_set = variants
                .iter()
                .map(|s| s.to_string())
                .collect::<HashSet<_>>();
            StringType::Enum {
                variants: variant_set,
            }
        }

        pub fn integer_range(min: i64, max: i64) -> NumberType {
            NumberType::Integer { min, max }
        }

        pub fn float_range(min: f64, max: f64) -> NumberType {
            NumberType::Float { min, max }
        }

        pub fn string_schema(string_type: StringType) -> SchemaState {
            SchemaState::String(string_type)
        }

        pub fn number_schema(number_type: NumberType) -> SchemaState {
            SchemaState::Number(number_type)
        }

        pub fn nullable_schema(inner: SchemaState) -> SchemaState {
            SchemaState::Nullable(Box::new(inner))
        }

        pub fn array_schema(
            min_length: usize,
            max_length: usize,
            item_schema: SchemaState,
        ) -> SchemaState {
            SchemaState::Array {
                min_length,
                max_length,
                schema: Box::new(item_schema),
            }
        }

        pub fn object_schema(
            required_fields: Vec<(&str, SchemaState)>,
            optional_fields: Vec<(&str, SchemaState)>,
        ) -> SchemaState {
            let required = required_fields
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect::<HashMap<_, _>>();

            let optional = optional_fields
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect::<HashMap<_, _>>();

            SchemaState::Object { required, optional }
        }

        pub fn assert_schema_equals(schema: &SchemaState, expected: serde_json::Value) {
            assert_eq!(schema.to_json_schema(), expected);
        }
    }

    mod json_schema_tests {
        use super::test_helpers::*;
        use super::*;

        mod basic_types {
            use super::*;

            #[test]
            fn boolean_to_json_schema() {
                assert_schema_equals(&SchemaState::Boolean, json!({ "type": "boolean" }));
            }

            #[test]
            fn null_to_json_schema() {
                assert_schema_equals(&SchemaState::Null, json!({ "type": "null" }));
            }

            #[test]
            fn initial_to_json_schema() {
                assert_schema_equals(&SchemaState::Initial, json!({}));
            }

            #[test]
            fn indefinite_to_json_schema() {
                assert_schema_equals(&SchemaState::Indefinite, json!({}));
            }
        }

        mod string_types {
            use super::*;

            #[test]
            fn unknown_string_with_length_constraints_to_json_schema() {
                let schema = string_schema(unknown_string(Some(3), Some(10)));
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "string"
                    }),
                );
            }

            #[test]
            fn unknown_string_no_constraints_to_json_schema() {
                let schema = string_schema(unknown_string(None, None));
                assert_schema_equals(&schema, json!({ "type": "string" }));
            }

            #[test]
            fn unknown_string_min_only_to_json_schema() {
                let schema = string_schema(unknown_string(Some(5), None));
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "string"
                    }),
                );
            }

            #[test]
            fn unknown_string_max_only_to_json_schema() {
                let schema = string_schema(unknown_string(None, Some(20)));
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "string"
                    }),
                );
            }

            #[test]
            fn uuid_string_to_json_schema() {
                let schema = string_schema(StringType::UUID);
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "string",
                        "format": "uuid"
                    }),
                );
            }

            #[test]
            fn email_string_to_json_schema() {
                let schema = string_schema(StringType::Email);
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "string",
                        "format": "email"
                    }),
                );
            }

            #[test]
            fn url_string_to_json_schema() {
                let schema = string_schema(StringType::Url);
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "string",
                        "format": "uri"
                    }),
                );
            }

            #[test]
            fn iso_date_string_to_json_schema() {
                let schema = string_schema(StringType::IsoDate);
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "string",
                        "format": "date"
                    }),
                );
            }

            #[test]
            fn datetime_iso8601_string_to_json_schema() {
                let schema = string_schema(StringType::DateTimeISO8601);
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "string",
                        "format": "date-time"
                    }),
                );
            }

            #[test]
            fn hostname_string_to_json_schema() {
                let schema = string_schema(StringType::Hostname);
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "string",
                        "format": "hostname",
                        "x-drivel-type": "hostname"
                    }),
                );
            }

            #[test]
            fn datetime_rfc2822_string_to_json_schema() {
                let schema = string_schema(StringType::DateTimeRFC2822);
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "string",
                        "x-drivel-type": "datetime-rfc2822",
                        "description": "RFC 2822 datetime format"
                    }),
                );
            }

            #[test]
            fn enum_string_multiple_variants_to_json_schema() {
                let schema = string_schema(enum_string(vec!["red", "green", "blue"]));
                let result = schema.to_json_schema();
                assert_eq!(result["type"], "string");
                assert!(result["enum"].is_array());
                let enum_values = result["enum"].as_array().unwrap();
                assert_eq!(enum_values.len(), 3);
            }

            #[test]
            fn enum_string_single_variant_to_json_schema() {
                let schema = string_schema(enum_string(vec!["only"]));
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "string",
                        "enum": ["only"]
                    }),
                );
            }

            #[test]
            fn enum_string_empty_variants_to_json_schema() {
                let schema = string_schema(enum_string(vec![]));
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "string",
                        "enum": []
                    }),
                );
            }
        }

        mod number_types {
            use super::*;

            #[test]
            fn integer_range_to_json_schema() {
                let schema = number_schema(integer_range(1, 100));
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "integer"
                    }),
                );
            }

            #[test]
            fn integer_single_value_to_json_schema() {
                let schema = number_schema(integer_range(42, 42));
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "integer"
                    }),
                );
            }

            #[test]
            fn integer_negative_range_to_json_schema() {
                let schema = number_schema(integer_range(-100, -10));
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "integer"
                    }),
                );
            }

            #[test]
            fn integer_zero_range_to_json_schema() {
                let schema = number_schema(integer_range(0, 0));
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "integer"
                    }),
                );
            }

            #[test]
            fn float_range_to_json_schema() {
                let schema = number_schema(float_range(1.5, 99.9));
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "number"
                    }),
                );
            }

            #[test]
            fn float_single_value_to_json_schema() {
                let schema = number_schema(float_range(3.14, 3.14));
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "number"
                    }),
                );
            }

            #[test]
            fn float_negative_range_to_json_schema() {
                let schema = number_schema(float_range(-99.9, -1.1));
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "number"
                    }),
                );
            }
        }

        mod nullable_types {
            use super::*;

            #[test]
            fn nullable_string_to_json_schema() {
                let schema = nullable_schema(string_schema(StringType::UUID));
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": ["string", "null"],
                        "format": "uuid"
                    }),
                );
            }

            #[test]
            fn nullable_integer_to_json_schema() {
                let schema = nullable_schema(number_schema(integer_range(1, 10)));
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": ["integer", "null"]
                    }),
                );
            }

            #[test]
            fn nullable_array_to_json_schema() {
                let schema = nullable_schema(array_schema(1, 3, string_schema(StringType::Email)));
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": ["array", "null"],
                        "items": {
                            "type": "string",
                            "format": "email"
                        }
                    }),
                );
            }

            #[test]
            fn nullable_object_to_json_schema() {
                let schema = nullable_schema(object_schema(
                    vec![("id", number_schema(integer_range(1, 100)))],
                    vec![],
                ));
                let result = schema.to_json_schema();
                assert_eq!(result["type"], json!(["object", "null"]));
                assert_eq!(result["properties"]["id"]["type"], "integer");
                assert_eq!(result["required"], json!(["id"]));
                assert_eq!(result["additionalProperties"], false);
            }
        }

        mod array_types {
            use super::*;

            #[test]
            fn array_with_constraints_to_json_schema() {
                let schema = array_schema(1, 5, string_schema(StringType::UUID));
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "array",
                        "items": {
                            "type": "string",
                            "format": "uuid"
                        }
                    }),
                );
            }

            #[test]
            fn empty_array_to_json_schema() {
                let schema = array_schema(0, 0, SchemaState::Boolean);
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "array",
                        "items": { "type": "boolean" }
                    }),
                );
            }

            #[test]
            fn array_of_objects_to_json_schema() {
                let schema = array_schema(
                    1,
                    10,
                    object_schema(
                        vec![("name", string_schema(unknown_string(Some(1), Some(50))))],
                        vec![],
                    ),
                );
                let result = schema.to_json_schema();
                assert_eq!(result["type"], "array");
                assert_eq!(result["items"]["type"], "object");
                assert_eq!(result["items"]["properties"]["name"]["type"], "string");
            }

            #[test]
            fn nested_array_to_json_schema() {
                let schema = array_schema(
                    1,
                    3,
                    array_schema(2, 4, number_schema(integer_range(1, 100))),
                );
                let result = schema.to_json_schema();
                assert_eq!(result["type"], "array");
                assert_eq!(result["items"]["type"], "array");
                assert_eq!(result["items"]["items"]["type"], "integer");
            }
        }

        mod object_types {
            use super::*;

            #[test]
            fn object_with_required_and_optional_to_json_schema() {
                let schema = object_schema(
                    vec![("id", number_schema(integer_range(1, 1000)))],
                    vec![("name", string_schema(unknown_string(Some(1), Some(50))))],
                );
                let result = schema.to_json_schema();

                assert_eq!(result["type"], "object");
                assert_eq!(result["additionalProperties"], false);
                assert_eq!(result["required"], json!(["id"]));
                assert_eq!(result["properties"]["id"]["type"], "integer");
                assert_eq!(result["properties"]["name"]["type"], "string");
            }

            #[test]
            fn empty_object_to_json_schema() {
                let schema = object_schema(vec![], vec![]);
                assert_schema_equals(
                    &schema,
                    json!({
                        "type": "object",
                        "additionalProperties": false,
                        "required": [],
                        "properties": {}
                    }),
                );
            }

            #[test]
            fn object_required_only_to_json_schema() {
                let schema = object_schema(
                    vec![
                        ("id", number_schema(integer_range(1, 100))),
                        ("status", SchemaState::Boolean),
                    ],
                    vec![],
                );
                let result = schema.to_json_schema();
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
                let schema = object_schema(
                    vec![],
                    vec![
                        ("description", string_schema(unknown_string(None, None))),
                        ("count", number_schema(integer_range(0, 10))),
                    ],
                );
                let result = schema.to_json_schema();
                assert_eq!(result["type"], "object");
                assert_eq!(result["required"], json!([]));
                assert_eq!(result["properties"]["description"]["type"], "string");
                assert_eq!(result["properties"]["count"]["type"], "integer");
            }

            #[test]
            fn nested_object_to_json_schema() {
                let inner_object = object_schema(
                    vec![("nested_id", number_schema(integer_range(1, 10)))],
                    vec![],
                );
                let schema = object_schema(vec![("inner", inner_object)], vec![]);
                let result = schema.to_json_schema();
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

        mod document_format {
            use super::*;

            #[test]
            fn json_schema_document_format() {
                let schema = SchemaState::Boolean;
                let document = schema.to_json_schema_document();

                assert_eq!(
                    document["$schema"],
                    "https://json-schema.org/draft/2020-12/schema"
                );
                assert_eq!(document["title"], "Inferred Schema");
                assert_eq!(
                    document["description"],
                    "Schema inferred by drivel from sample data"
                );
                assert_eq!(document["type"], "boolean");
            }
        }
    }
}
