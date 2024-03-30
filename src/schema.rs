use std::fmt::Display;

#[derive(PartialEq, Debug)]
pub enum StringType {
    Unknown {
        min_length: Option<usize>,
        max_length: Option<usize>,
    },
    IsoDate,
    IsoDateTime,
    UUID,
}

impl Display for StringType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            StringType::Unknown {
                min_length,
                max_length,
            } => {
                let length = match (min_length, max_length) {
                    (Some(min), Some(max)) => format!("({}-{})", min, max),
                    (Some(min), None) => format!("({}-?)", min),
                    (None, Some(max)) => format!("(?-{})", max),
                    (None, None) => format!("(length unknown)"),
                };
                format!("string {}", length)
            }
            StringType::IsoDate => "string (iso date)".to_owned(),
            StringType::IsoDateTime => "string (iso datetime)".to_owned(),
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
            NumberType::Integer { min, max } => format!("int ({}-{})", min, max),
            NumberType::Float { min, max } => format!("float ({}-{})", min, max),
        };
        write!(f, "{}", text)
    }
}

#[derive(PartialEq, Debug)]
pub enum SchemaState {
    Initial,
    Null,
    Nullable(Box<SchemaState>),
    String(StringType),
    Number(NumberType),
    Boolean,
    Array(Box<SchemaState>),
    Object {
        required: std::collections::HashMap<String, SchemaState>,
        optional: std::collections::HashMap<String, SchemaState>,
    },
    Indefinite,
}

impl SchemaState {
    pub fn to_string_pretty(&self, depth: usize) -> String {
        match self {
            SchemaState::Initial | SchemaState::Indefinite => "unknown".to_string(),
            SchemaState::Null => "null".to_string(),
            SchemaState::Nullable(state) => format!("nullable {}", state.to_string_pretty(depth)),
            SchemaState::String(string_type) => format!("{}", string_type),
            SchemaState::Number(number_type) => format!("{}", number_type),
            SchemaState::Boolean => "boolean".to_string(),
            SchemaState::Array(state) => {
                let indent = 2 + 2 * depth;
                let indent_str = " ".repeat(indent);
                let indent_str_close = " ".repeat(indent - 2);
                format!(
                    "[\n{}{}\n{}]",
                    indent_str,
                    state.to_string_pretty(depth + 1),
                    indent_str_close
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
                            v.to_string_pretty(depth + 1)
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
                            v.to_string_pretty(depth + 1)
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
}
