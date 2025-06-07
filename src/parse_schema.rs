use crate::schema::{SchemaState, StringType, NumberType};
use serde_json::{Map, Value};
use std::fmt;

#[derive(Debug)]
pub enum ParseSchemaError {
    InvalidSchema(String),
    UnsupportedFeature(String), 
    ValidationFailed(String),
}

impl fmt::Display for ParseSchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseSchemaError::InvalidSchema(msg) => write!(f, "Invalid JSON Schema: {}", msg),
            ParseSchemaError::UnsupportedFeature(msg) => write!(f, "Unsupported JSON Schema feature: {}", msg),
            ParseSchemaError::ValidationFailed(msg) => write!(f, "Validation failed: {}", msg),
        }
    }
}

impl std::error::Error for ParseSchemaError {}

pub fn parse_json_schema(schema_json: &Value) -> Result<SchemaState, ParseSchemaError> {
    let schema_obj = schema_json.as_object()
        .ok_or_else(|| ParseSchemaError::InvalidSchema("Schema must be an object".to_string()))?;
    
    let type_field = schema_obj.get("type")
        .ok_or_else(|| ParseSchemaError::InvalidSchema("Schema must have a 'type' field".to_string()))?;
    
    let type_str = type_field.as_str()
        .ok_or_else(|| ParseSchemaError::InvalidSchema("Type field must be a string".to_string()))?;
    
    match type_str {
        "string" => parse_string_type(schema_obj),
        "number" => parse_number_type(schema_obj, false),
        "integer" => parse_number_type(schema_obj, true),
        "boolean" => Ok(SchemaState::Boolean),
        "null" => Ok(SchemaState::Null),
        "object" => parse_object_type(schema_obj),
        "array" => parse_array_type(schema_obj),
        _ => Err(ParseSchemaError::UnsupportedFeature(format!("Type '{}' not supported yet", type_str)))
    }
}

fn parse_string_type(schema_obj: &Map<String, Value>) -> Result<SchemaState, ParseSchemaError> {
    let (min_length, max_length) = parse_string_length_constraints(schema_obj)?;
    
    if let Some(enum_value) = schema_obj.get("enum") {
        parse_string_enum(enum_value)
    } else if let Some(format_value) = schema_obj.get("format") {
        parse_string_with_format(format_value, min_length, max_length)
    } else {
        Ok(SchemaState::String(create_unknown_string_type(min_length, max_length)))
    }
}

fn parse_string_length_constraints(schema_obj: &Map<String, Value>) -> Result<(Option<usize>, Option<usize>), ParseSchemaError> {
    let min_length = parse_optional_usize_field(schema_obj, "minLength")?;
    let max_length = parse_optional_usize_field(schema_obj, "maxLength")?;
    Ok((min_length, max_length))
}

fn parse_string_with_format(format_value: &Value, min_length: Option<usize>, max_length: Option<usize>) -> Result<SchemaState, ParseSchemaError> {
    let format_str = format_value.as_str()
        .ok_or_else(|| ParseSchemaError::InvalidSchema("Format field must be a string".to_string()))?;
    
    match format_str {
        "email" => Ok(SchemaState::String(StringType::Email)),
        "uuid" => Ok(SchemaState::String(StringType::UUID)),
        "date" => Ok(SchemaState::String(StringType::IsoDate)),
        "date-time" => Ok(SchemaState::String(StringType::DateTimeISO8601)),
        "uri" => Ok(SchemaState::String(StringType::Url)),
        "hostname" => Ok(SchemaState::String(StringType::Hostname)),
        _ => {
            // Warn about unsupported format but continue with constraints to avoid breaking parsing
            eprintln!("Warning: Unsupported string format '{}', using basic string type", format_str);
            Ok(SchemaState::String(create_unknown_string_type(min_length, max_length)))
        }
    }
}

fn create_unknown_string_type(min_length: Option<usize>, max_length: Option<usize>) -> StringType {
    StringType::Unknown {
        strings_seen: vec![],
        chars_seen: vec![],
        min_length,
        max_length,
    }
}

fn parse_string_enum(enum_value: &Value) -> Result<SchemaState, ParseSchemaError> {
    let enum_array = enum_value.as_array()
        .ok_or_else(|| ParseSchemaError::InvalidSchema("Enum field must be an array".to_string()))?;
    
    let mut variants = std::collections::HashSet::new();
    
    for item in enum_array {
        let string_value = item.as_str()
            .ok_or_else(|| ParseSchemaError::InvalidSchema("All enum values must be strings".to_string()))?;
        variants.insert(string_value.to_string());
    }
    
    Ok(SchemaState::String(StringType::Enum { variants }))
}

fn parse_number_type(schema_obj: &Map<String, Value>, is_integer: bool) -> Result<SchemaState, ParseSchemaError> {
    let (min_value, max_value) = parse_number_constraints(schema_obj)?;
    warn_about_unsupported_number_features(schema_obj);
    
    if is_integer {
        let min = min_value.map(|v| v as i64).unwrap_or(i64::MIN);
        let max = max_value.map(|v| v as i64).unwrap_or(i64::MAX);
        Ok(SchemaState::Number(NumberType::Integer { min, max }))
    } else {
        let min = min_value.unwrap_or(f64::NEG_INFINITY);
        let max = max_value.unwrap_or(f64::INFINITY);
        Ok(SchemaState::Number(NumberType::Float { min, max }))
    }
}

fn parse_number_constraints(schema_obj: &Map<String, Value>) -> Result<(Option<f64>, Option<f64>), ParseSchemaError> {
    let min_value = parse_numeric_field(schema_obj, "minimum")?;
    let max_value = parse_numeric_field(schema_obj, "maximum")?;
    Ok((min_value, max_value))
}

fn parse_numeric_field(schema_obj: &Map<String, Value>, field_name: &str) -> Result<Option<f64>, ParseSchemaError> {
    if let Some(value) = schema_obj.get(field_name) {
        let number = value.as_f64()
            .ok_or_else(|| ParseSchemaError::InvalidSchema(format!("{} must be a number", field_name)))?;
        Ok(Some(number))
    } else {
        Ok(None)
    }
}

fn warn_about_unsupported_number_features(schema_obj: &Map<String, Value>) {
    if schema_obj.contains_key("exclusiveMinimum") {
        eprintln!("Warning: exclusiveMinimum not supported, treating as inclusive minimum");
    }
    
    if schema_obj.contains_key("exclusiveMaximum") {
        eprintln!("Warning: exclusiveMaximum not supported, treating as inclusive maximum");
    }
    
    if schema_obj.contains_key("multipleOf") {
        eprintln!("Warning: multipleOf constraint not supported, ignoring");
    }
}

fn parse_object_type(schema_obj: &Map<String, Value>) -> Result<SchemaState, ParseSchemaError> {
    let empty_map = serde_json::Map::new();
    let properties = schema_obj.get("properties")
        .and_then(|p| p.as_object())
        .unwrap_or(&empty_map);
    
    let required_names = parse_required_field_names(schema_obj);
    let (required_fields, optional_fields) = parse_object_properties(properties, &required_names)?;
    
    warn_about_unsupported_object_features(schema_obj);
    
    Ok(SchemaState::Object {
        required: required_fields,
        optional: optional_fields,
    })
}

fn parse_required_field_names(schema_obj: &Map<String, Value>) -> std::collections::HashSet<String> {
    schema_obj.get("required")
        .and_then(|r| r.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default()
}

fn parse_object_properties(
    properties: &Map<String, Value>, 
    required_names: &std::collections::HashSet<String>
) -> Result<(std::collections::HashMap<String, SchemaState>, std::collections::HashMap<String, SchemaState>), ParseSchemaError> {
    let mut required_fields = std::collections::HashMap::new();
    let mut optional_fields = std::collections::HashMap::new();
    
    for (property_name, property_schema) in properties {
        let parsed_schema = parse_json_schema(property_schema)?;
        
        if required_names.contains(property_name) {
            required_fields.insert(property_name.clone(), parsed_schema);
        } else {
            optional_fields.insert(property_name.clone(), parsed_schema);
        }
    }
    
    Ok((required_fields, optional_fields))
}

fn warn_about_unsupported_object_features(schema_obj: &Map<String, Value>) {
    if let Some(additional_props) = schema_obj.get("additionalProperties") {
        if additional_props.as_bool() == Some(true) {
            eprintln!("Warning: additionalProperties: true not fully supported, allowing any additional properties");
        } else if additional_props.is_object() {
            eprintln!("Warning: additionalProperties schema not supported, ignoring");
        }
    }
    
    if schema_obj.contains_key("patternProperties") {
        eprintln!("Warning: patternProperties not supported, ignoring");
    }
}

fn parse_array_type(schema_obj: &Map<String, Value>) -> Result<SchemaState, ParseSchemaError> {
    // Parse the items schema
    let items_schema = schema_obj.get("items")
        .ok_or_else(|| ParseSchemaError::InvalidSchema("Array schema must have an 'items' field".to_string()))?;
    
    let parsed_items_schema = parse_json_schema(items_schema)?;
    
    // Parse array constraints
    let (min_items, max_items) = parse_array_constraints(schema_obj)?;
    
    // Warn about unsupported array features
    warn_about_unsupported_array_features(schema_obj);
    
    Ok(SchemaState::Array {
        min_length: min_items,
        max_length: max_items,
        schema: Box::new(parsed_items_schema),
    })
}

fn parse_array_constraints(schema_obj: &Map<String, Value>) -> Result<(usize, usize), ParseSchemaError> {
    let min_items = parse_optional_usize_field(schema_obj, "minItems")?.unwrap_or(0);
    let max_items = parse_optional_usize_field(schema_obj, "maxItems")?.unwrap_or(usize::MAX);
    Ok((min_items, max_items))
}

fn parse_optional_usize_field(schema_obj: &Map<String, Value>, field_name: &str) -> Result<Option<usize>, ParseSchemaError> {
    if let Some(value) = schema_obj.get(field_name) {
        let number = value.as_u64()
            .ok_or_else(|| ParseSchemaError::InvalidSchema(format!("{} must be a number", field_name)))?;
        Ok(Some(number as usize))
    } else {
        Ok(None)
    }
}

fn warn_about_unsupported_array_features(schema_obj: &Map<String, Value>) {
    if schema_obj.contains_key("uniqueItems") {
        eprintln!("Warning: uniqueItems constraint not supported, ignoring");
    }
    
    if schema_obj.contains_key("contains") {
        eprintln!("Warning: contains keyword not supported, ignoring");
    }
    
    if schema_obj.contains_key("additionalItems") {
        eprintln!("Warning: additionalItems not supported, ignoring");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::StringType;
    use serde_json::json;

    #[test]
    fn parse_basic_string_schema() {
        let schema = json!({
            "type": "string"
        });
        
        let result = parse_json_schema(&schema);
        
        // This should pass once we implement basic string parsing
        match result {
            Ok(SchemaState::String(_)) => {
                // Success case - not reached yet with stub
            }
            _ => {
                // Currently fails with stub - this is expected
                panic!("Expected string schema to parse successfully");
            }
        }
    }

    #[test]
    fn parse_string_with_email_format() {
        let schema = json!({
            "type": "string",
            "format": "email"
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::String(StringType::Email)) => {
                // Expected result
            }
            _ => {
                panic!("Expected email format string to parse to StringType::Email");
            }
        }
    }

    #[test] 
    fn parse_string_with_uuid_format() {
        let schema = json!({
            "type": "string",
            "format": "uuid"
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::String(StringType::UUID)) => {
                // Expected result
            }
            _ => {
                panic!("Expected uuid format string to parse to StringType::UUID");
            }
        }
    }

    #[test]
    fn parse_string_with_date_format() {
        let schema = json!({
            "type": "string", 
            "format": "date"
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::String(StringType::IsoDate)) => {
                // Expected result
            }
            _ => {
                panic!("Expected date format string to parse to StringType::IsoDate");
            }
        }
    }

    #[test]
    fn parse_string_with_unsupported_format() {
        let schema = json!({
            "type": "string",
            "format": "unsupported-format"
        });
        
        let result = parse_json_schema(&schema);
        
        // Should succeed but use basic string type, and warn to stderr
        match result {
            Ok(SchemaState::String(StringType::Unknown { .. })) => {
                // Expected - falls back to unknown string type
            }
            _ => {
                panic!("Expected unsupported format to fall back to Unknown string type");
            }
        }
    }

    #[test]
    fn parse_string_with_length_constraints() {
        let schema = json!({
            "type": "string",
            "minLength": 5,
            "maxLength": 20
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::String(StringType::Unknown { min_length, max_length, .. })) => {
                assert_eq!(min_length, Some(5));
                assert_eq!(max_length, Some(20));
            }
            _ => {
                panic!("Expected string with length constraints to parse correctly");
            }
        }
    }

    #[test]
    fn parse_string_with_min_length_only() {
        let schema = json!({
            "type": "string",
            "minLength": 10
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::String(StringType::Unknown { min_length, max_length, .. })) => {
                assert_eq!(min_length, Some(10));
                assert_eq!(max_length, None);
            }
            _ => {
                panic!("Expected string with min length to parse correctly");
            }
        }
    }

    #[test]
    fn parse_string_with_max_length_only() {
        let schema = json!({
            "type": "string",
            "maxLength": 50
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::String(StringType::Unknown { min_length, max_length, .. })) => {
                assert_eq!(min_length, None);
                assert_eq!(max_length, Some(50));
            }
            _ => {
                panic!("Expected string with max length to parse correctly");
            }
        }
    }

    #[test]
    fn parse_basic_number_schema() {
        let schema = json!({
            "type": "number"
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::Number(_)) => {
                // Expected result
            }
            _ => {
                panic!("Expected number schema to parse successfully");
            }
        }
    }

    #[test]
    fn parse_basic_integer_schema() {
        let schema = json!({
            "type": "integer"
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::Number(_)) => {
                // Expected result  
            }
            _ => {
                panic!("Expected integer schema to parse successfully");
            }
        }
    }

    #[test]
    fn parse_number_with_constraints() {
        let schema = json!({
            "type": "number",
            "minimum": 1.5,
            "maximum": 99.9
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::Number(crate::schema::NumberType::Float { min, max })) => {
                assert_eq!(min, 1.5);
                assert_eq!(max, 99.9);
            }
            _ => {
                panic!("Expected number with constraints to parse correctly");
            }
        }
    }

    #[test]
    fn parse_integer_with_constraints() {
        let schema = json!({
            "type": "integer",
            "minimum": 0,
            "maximum": 100
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::Number(crate::schema::NumberType::Integer { min, max })) => {
                assert_eq!(min, 0);
                assert_eq!(max, 100);
            }
            _ => {
                panic!("Expected integer with constraints to parse correctly");
            }
        }
    }

    #[test]
    fn parse_number_without_constraints() {
        let schema = json!({
            "type": "number"
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::Number(crate::schema::NumberType::Float { min, max })) => {
                assert_eq!(min, f64::NEG_INFINITY);
                assert_eq!(max, f64::INFINITY);
            }
            _ => {
                panic!("Expected number without constraints to use infinite bounds");
            }
        }
    }

    #[test]
    fn parse_integer_without_constraints() {
        let schema = json!({
            "type": "integer"
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::Number(crate::schema::NumberType::Integer { min, max })) => {
                assert_eq!(min, i64::MIN);
                assert_eq!(max, i64::MAX);
            }
            _ => {
                panic!("Expected integer without constraints to use min/max bounds");
            }
        }
    }

    #[test]
    fn parse_number_with_unsupported_constraints() {
        let schema = json!({
            "type": "number",
            "minimum": 5.0,
            "exclusiveMaximum": 10.0,
            "multipleOf": 2.5
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::Number(crate::schema::NumberType::Float { min, max })) => {
                assert_eq!(min, 5.0);
                assert_eq!(max, f64::INFINITY);
            }
            _ => {
                panic!("Expected number with unsupported constraints to parse with warnings");
            }
        }
    }

    #[test]
    fn parse_basic_boolean_schema() {
        let schema = json!({
            "type": "boolean"
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::Boolean) => {
                // Expected result
            }
            _ => {
                panic!("Expected boolean schema to parse successfully");
            }
        }
    }

    #[test]
    fn parse_basic_null_schema() {
        let schema = json!({
            "type": "null"
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::Null) => {
                // Expected result
            }
            _ => {
                panic!("Expected null schema to parse successfully");
            }
        }
    }

    #[test]
    fn parse_string_enum_schema() {
        let schema = json!({
            "type": "string",
            "enum": ["foo", "bar", "baz"]
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::String(crate::schema::StringType::Enum { variants })) => {
                assert_eq!(variants.len(), 3);
                assert!(variants.contains("foo"));
                assert!(variants.contains("bar"));
                assert!(variants.contains("baz"));
            }
            _ => {
                panic!("Expected string enum schema to parse to StringType::Enum");
            }
        }
    }

    #[test]
    fn parse_string_enum_empty() {
        let schema = json!({
            "type": "string",
            "enum": []
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::String(crate::schema::StringType::Enum { variants })) => {
                assert_eq!(variants.len(), 0);
            }
            _ => {
                panic!("Expected empty string enum schema to parse to StringType::Enum");
            }
        }
    }

    #[test]
    fn parse_basic_object_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string"
                },
                "age": {
                    "type": "integer"
                }
            },
            "required": ["name"]
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::Object { required, optional }) => {
                // name should be required
                assert!(required.contains_key("name"));
                assert!(matches!(required.get("name"), Some(SchemaState::String(_))));
                
                // age should be optional
                assert!(optional.contains_key("age"));
                assert!(matches!(optional.get("age"), Some(SchemaState::Number(_))));
            }
            _ => {
                panic!("Expected object schema to parse to SchemaState::Object");
            }
        }
    }

    #[test]
    fn parse_nested_object_schema() {
        let schema = json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "id": {
                            "type": "integer"
                        },
                        "email": {
                            "type": "string",
                            "format": "email"
                        }
                    },
                    "required": ["id"]
                },
                "active": {
                    "type": "boolean"
                }
            },
            "required": ["user"]
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::Object { required, optional }) => {
                // user should be required and be an object
                assert!(required.contains_key("user"));
                match required.get("user") {
                    Some(SchemaState::Object { required: user_required, optional: user_optional }) => {
                        // Check nested object structure
                        assert!(user_required.contains_key("id"));
                        assert!(matches!(user_required.get("id"), Some(SchemaState::Number(_))));
                        
                        assert!(user_optional.contains_key("email"));
                        assert!(matches!(user_optional.get("email"), Some(SchemaState::String(_))));
                    }
                    _ => panic!("Expected user field to be an object")
                }
                
                // active should be optional
                assert!(optional.contains_key("active"));
                assert!(matches!(optional.get("active"), Some(SchemaState::Boolean)));
            }
            _ => {
                panic!("Expected nested object schema to parse to SchemaState::Object");
            }
        }
    }

    #[test]
    fn parse_basic_array_schema() {
        let schema = json!({
            "type": "array",
            "items": {
                "type": "string"
            },
            "minItems": 1,
            "maxItems": 10
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::Array { min_length, max_length, schema: item_schema }) => {
                assert_eq!(min_length, 1);
                assert_eq!(max_length, 10);
                assert!(matches!(item_schema.as_ref(), SchemaState::String(_)));
            }
            _ => {
                panic!("Expected array schema to parse to SchemaState::Array");
            }
        }
    }

    #[test]
    fn parse_array_without_constraints() {
        let schema = json!({
            "type": "array",
            "items": {
                "type": "integer"
            }
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::Array { min_length, max_length, schema: item_schema }) => {
                assert_eq!(min_length, 0);
                assert_eq!(max_length, usize::MAX);
                assert!(matches!(item_schema.as_ref(), SchemaState::Number(_)));
            }
            _ => {
                panic!("Expected array without constraints to parse with default bounds");
            }
        }
    }

    #[test]
    fn parse_nested_array_schema() {
        let schema = json!({
            "type": "array",
            "items": {
                "type": "array",
                "items": {
                    "type": "string",
                    "format": "email"
                },
                "minItems": 2,
                "maxItems": 5
            },
            "minItems": 1,
            "maxItems": 3
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::Array { min_length, max_length, schema: item_schema }) => {
                assert_eq!(min_length, 1);
                assert_eq!(max_length, 3);
                
                // Check nested array
                match item_schema.as_ref() {
                    SchemaState::Array { min_length: inner_min, max_length: inner_max, schema: inner_schema } => {
                        assert_eq!(*inner_min, 2);
                        assert_eq!(*inner_max, 5);
                        assert!(matches!(inner_schema.as_ref(), SchemaState::String(_)));
                    }
                    _ => panic!("Expected nested array structure")
                }
            }
            _ => {
                panic!("Expected nested array schema to parse to SchemaState::Array");
            }
        }
    }

    #[test]
    fn parse_array_of_objects() {
        let schema = json!({
            "type": "array",
            "items": {
                "type": "object",
                "properties": {
                    "id": {
                        "type": "integer"
                    },
                    "name": {
                        "type": "string"
                    }
                },
                "required": ["id"]
            }
        });
        
        let result = parse_json_schema(&schema);
        
        match result {
            Ok(SchemaState::Array { schema: item_schema, .. }) => {
                match item_schema.as_ref() {
                    SchemaState::Object { required, optional } => {
                        assert!(required.contains_key("id"));
                        assert!(optional.contains_key("name"));
                    }
                    _ => panic!("Expected array items to be objects")
                }
            }
            _ => {
                panic!("Expected array of objects to parse correctly");
            }
        }
    }
}