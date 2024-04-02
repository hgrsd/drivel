use std::fmt::Display;

pub(crate) const STRING_VARIANTS_SMALL_DATA_BOUNDARY: usize = 10;

#[derive(PartialEq, Debug)]
pub enum StringType {
    Unknown {
        chars_seen: Vec<char>,
        strings_seen: std::collections::HashSet<String>,
        min_length: Option<usize>,
        max_length: Option<usize>,
    },
    IsoDate,
    DateTimeRFC2822,
    DateTimeISO8601,
    UUID,
}

impl Display for StringType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            StringType::Unknown {
                chars_seen: _,
                strings_seen,
                min_length,
                max_length,
            } => {
                if strings_seen.len() > 0
                    && strings_seen.len() <= STRING_VARIANTS_SMALL_DATA_BOUNDARY
                {
                    let as_vec = Vec::from_iter(strings_seen.iter().cloned());
                    format!("string (limited dataset: {})", &as_vec.join(","))
                } else {
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
            }
            StringType::IsoDate => "string (date - ISO 8601)".to_owned(),
            StringType::DateTimeRFC2822 => "string (datetime - RFC 2822)".to_owned(),
            StringType::DateTimeISO8601 => "string (datetime - ISO 8601)".to_owned(),
            StringType::UUID => "string (uuid)".to_owned(),
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
    ///         chars_seen: vec!['a', 'b', 'c'],
    ///         strings_seen: HashSet::from_iter([
    ///             "bac".to_string(),
    ///             "abc".to_string(),
    ///             "baa".to_string(),
    ///             "bcc".to_string()
    ///         ]),
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
