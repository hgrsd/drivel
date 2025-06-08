use crate::schema::{NumberType, SchemaState, StringType};
use serde_json::{Map, Value};
use std::fmt;

type ObjectProperties = (
    std::collections::HashMap<String, SchemaState>,
    std::collections::HashMap<String, SchemaState>,
);

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
            ParseSchemaError::UnsupportedFeature(msg) => {
                write!(f, "Unsupported JSON Schema feature: {}", msg)
            }
            ParseSchemaError::ValidationFailed(msg) => write!(f, "Validation failed: {}", msg),
        }
    }
}

impl std::error::Error for ParseSchemaError {}

pub fn parse_json_schema(schema_json: &Value) -> Result<SchemaState, ParseSchemaError> {
    let schema_obj = schema_json
        .as_object()
        .ok_or_else(|| ParseSchemaError::InvalidSchema("Schema must be an object".to_string()))?;

    if let Some(any_of) = schema_obj.get("anyOf") {
        if let Some(nullable_schema) = try_parse_nullable_anyof_oneof(any_of)? {
            return Ok(nullable_schema);
        }
        return Err(ParseSchemaError::UnsupportedFeature(
            "anyOf patterns other than nullable not supported yet".to_string(),
        ));
    }

    if let Some(one_of) = schema_obj.get("oneOf") {
        if let Some(nullable_schema) = try_parse_nullable_anyof_oneof(one_of)? {
            return Ok(nullable_schema);
        }
        return Err(ParseSchemaError::UnsupportedFeature(
            "oneOf patterns other than nullable not supported yet".to_string(),
        ));
    }

    let type_field = schema_obj.get("type").ok_or_else(|| {
        ParseSchemaError::InvalidSchema(
            "Schema must have a 'type' field, 'anyOf', or 'oneOf'".to_string(),
        )
    })?;

    // Handle nullable types (arrays) vs single types (strings)
    if let Some(type_array) = type_field.as_array() {
        parse_nullable_type(schema_obj, type_array)
    } else if let Some(type_str) = type_field.as_str() {
        parse_single_type(schema_obj, type_str)
    } else {
        Err(ParseSchemaError::InvalidSchema(
            "Type field must be a string or array".to_string(),
        ))
    }
}

fn parse_single_type(
    schema_obj: &Map<String, Value>,
    type_str: &str,
) -> Result<SchemaState, ParseSchemaError> {
    match type_str {
        "string" => parse_string_type(schema_obj),
        "number" => parse_number_type(schema_obj, false),
        "integer" => parse_number_type(schema_obj, true),
        "boolean" => Ok(SchemaState::Boolean),
        "null" => Ok(SchemaState::Null),
        "object" => parse_object_type(schema_obj),
        "array" => parse_array_type(schema_obj),
        _ => Err(ParseSchemaError::UnsupportedFeature(format!(
            "Type '{}' not supported yet",
            type_str
        ))),
    }
}

fn parse_nullable_type(
    schema_obj: &Map<String, Value>,
    type_array: &[Value],
) -> Result<SchemaState, ParseSchemaError> {
    if type_array.len() != 2 {
        return Err(ParseSchemaError::UnsupportedFeature(
            "Only nullable types with exactly 2 elements are supported".to_string(),
        ));
    }

    let mut non_null_type = None;
    let mut has_null = false;

    for type_value in type_array {
        if let Some(type_str) = type_value.as_str() {
            if type_str == "null" {
                has_null = true;
            } else if non_null_type.is_none() {
                non_null_type = Some(type_str);
            } else {
                return Err(ParseSchemaError::UnsupportedFeature(
                    "Only one non-null type is supported in nullable types".to_string(),
                ));
            }
        } else {
            return Err(ParseSchemaError::InvalidSchema(
                "All type array elements must be strings".to_string(),
            ));
        }
    }

    if !has_null {
        return Err(ParseSchemaError::InvalidSchema(
            "Nullable type array must contain 'null'".to_string(),
        ));
    }

    let inner_type = non_null_type.ok_or_else(|| {
        ParseSchemaError::InvalidSchema(
            "Nullable type array must contain a non-null type".to_string(),
        )
    })?;

    let inner_schema = parse_single_type(schema_obj, inner_type)?;
    Ok(SchemaState::Nullable(Box::new(inner_schema)))
}

fn try_parse_nullable_anyof_oneof(
    schema_array: &Value,
) -> Result<Option<SchemaState>, ParseSchemaError> {
    let array = require_array(schema_array, "anyOf/oneOf")?;

    if array.len() != 2 {
        return Ok(None); // Not a simple nullable pattern
    }

    let mut null_schema = None;
    let mut type_schema = None;

    for item in array {
        let item_obj = require_object(item, "anyOf/oneOf items")?;

        if let Some(type_field) = item_obj.get("type") {
            if let Some(type_str) = type_field.as_str() {
                if type_str == "null" {
                    null_schema = Some(item);
                } else if type_schema.is_none() {
                    type_schema = Some(item);
                } else {
                    return Ok(None); // Multiple non-null types, not a simple nullable pattern
                }
            } else {
                return Ok(None); // Complex type, not a simple nullable pattern
            }
        } else {
            return Ok(None); // No type field, not a simple nullable pattern
        }
    }

    if null_schema.is_some() && type_schema.is_some() {
        let type_obj = type_schema.unwrap().as_object().unwrap();
        let type_field = type_obj.get("type").unwrap();
        let type_str = type_field.as_str().unwrap();

        let inner_schema = parse_single_type(type_obj, type_str)?;
        Ok(Some(SchemaState::Nullable(Box::new(inner_schema))))
    } else {
        Ok(None) // Not a nullable pattern
    }
}

fn parse_string_type(schema_obj: &Map<String, Value>) -> Result<SchemaState, ParseSchemaError> {
    let (min_length, max_length) = parse_string_length_constraints(schema_obj)?;

    if let Some(enum_value) = schema_obj.get("enum") {
        parse_string_enum(enum_value)
    } else if let Some(format_value) = schema_obj.get("format") {
        parse_string_with_format(format_value, min_length, max_length)
    } else {
        Ok(SchemaState::String(create_unknown_string_type(
            min_length, max_length,
        )))
    }
}

fn validate_min_max_constraint<T: PartialOrd>(
    min: Option<T>,
    max: Option<T>,
    error_message: &str,
) -> Result<(), ParseSchemaError> {
    if let (Some(min_val), Some(max_val)) = (min.as_ref(), max.as_ref()) {
        if min_val > max_val {
            return Err(ParseSchemaError::ValidationFailed(
                error_message.to_string(),
            ));
        }
    }
    Ok(())
}

fn parse_string_length_constraints(
    schema_obj: &Map<String, Value>,
) -> Result<(Option<usize>, Option<usize>), ParseSchemaError> {
    let min_length = parse_optional_usize_field(schema_obj, "minLength")?;
    let max_length = parse_optional_usize_field(schema_obj, "maxLength")?;

    validate_min_max_constraint(
        min_length,
        max_length,
        "minLength cannot be greater than maxLength",
    )?;

    Ok((min_length, max_length))
}

fn parse_string_with_format(
    format_value: &Value,
    min_length: Option<usize>,
    max_length: Option<usize>,
) -> Result<SchemaState, ParseSchemaError> {
    let format_str = require_string(format_value, "Format field")?;

    match format_str {
        "email" => Ok(SchemaState::String(StringType::Email)),
        "uuid" => Ok(SchemaState::String(StringType::UUID)),
        "date" => Ok(SchemaState::String(StringType::IsoDate)),
        "date-time" => Ok(SchemaState::String(StringType::DateTimeISO8601)),
        "uri" => Ok(SchemaState::String(StringType::Url)),
        "hostname" => Ok(SchemaState::String(StringType::Hostname)),
        _ => {
            // Warn about unsupported format but continue with constraints to avoid breaking parsing
            eprintln!(
                "Warning: Unsupported string format '{}', using basic string type",
                format_str
            );
            Ok(SchemaState::String(create_unknown_string_type(
                min_length, max_length,
            )))
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
    let enum_array = require_array(enum_value, "Enum field")?;

    // Validate that enum array is not empty
    if enum_array.is_empty() {
        return Err(ParseSchemaError::ValidationFailed(
            "enum array cannot be empty".to_string(),
        ));
    }

    let mut variants = std::collections::HashSet::new();

    for item in enum_array {
        let string_value = require_string(item, "All enum values")?;
        variants.insert(string_value.to_string());
    }

    Ok(SchemaState::String(StringType::Enum { variants }))
}

fn parse_number_type(
    schema_obj: &Map<String, Value>,
    is_integer: bool,
) -> Result<SchemaState, ParseSchemaError> {
    let (min_value, max_value) = parse_number_constraints(schema_obj)?;
    warn_about_unsupported_number_features(schema_obj);

    if is_integer {
        let min = min_value.map(|v| v as i64).unwrap_or(i64::MIN);
        let max = max_value.map(|v| v as i64).unwrap_or(i64::MAX);
        Ok(SchemaState::Number(NumberType::Integer { min, max }))
    } else {
        let min = min_value.unwrap_or(f64::NEG_INFINITY);
        let max = max_value.unwrap_or(f64::INFINITY);

        // Validate that finite ranges don't cause overflow in random generation
        if min.is_finite() && max.is_finite() {
            let range_size = max - min;
            if !range_size.is_finite() || range_size <= 0.0 {
                return Err(ParseSchemaError::ValidationFailed(
                    "Invalid floating point range: range too large or invalid".to_string(),
                ));
            }
        }

        Ok(SchemaState::Number(NumberType::Float { min, max }))
    }
}

fn parse_number_constraints(
    schema_obj: &Map<String, Value>,
) -> Result<(Option<f64>, Option<f64>), ParseSchemaError> {
    let mut min_value = parse_numeric_field(schema_obj, "minimum")?;
    let mut max_value = parse_numeric_field(schema_obj, "maximum")?;

    // Handle exclusive bounds by treating them as inclusive bounds (with warning)
    if let Some(exclusive_min) = parse_numeric_field(schema_obj, "exclusiveMinimum")? {
        if min_value.is_some() {
            return Err(ParseSchemaError::InvalidSchema(
                "Cannot specify both minimum and exclusiveMinimum".to_string(),
            ));
        }
        // Treat exclusive minimum as inclusive minimum (as indicated by warning)
        min_value = Some(exclusive_min);
    }

    if let Some(exclusive_max) = parse_numeric_field(schema_obj, "exclusiveMaximum")? {
        if max_value.is_some() {
            return Err(ParseSchemaError::InvalidSchema(
                "Cannot specify both maximum and exclusiveMaximum".to_string(),
            ));
        }
        // Treat exclusive maximum as inclusive maximum (as indicated by warning)
        max_value = Some(exclusive_max);
    }

    validate_min_max_constraint(
        min_value,
        max_value,
        "minimum cannot be greater than maximum",
    )?;

    Ok((min_value, max_value))
}

fn parse_numeric_field(
    schema_obj: &Map<String, Value>,
    field_name: &str,
) -> Result<Option<f64>, ParseSchemaError> {
    if let Some(value) = schema_obj.get(field_name) {
        let number = value.as_f64().ok_or_else(|| {
            ParseSchemaError::InvalidSchema(format!("{} must be a number", field_name))
        })?;
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
    let properties = get_object_field(schema_obj, "properties")?.unwrap_or(&empty_map);

    let required_names = parse_required_field_names(schema_obj)?;
    let (required_fields, optional_fields) = parse_object_properties(properties, &required_names)?;

    warn_about_unsupported_object_features(schema_obj);

    Ok(SchemaState::Object {
        required: required_fields,
        optional: optional_fields,
    })
}

fn parse_required_field_names(
    schema_obj: &Map<String, Value>,
) -> Result<std::collections::HashSet<String>, ParseSchemaError> {
    if let Some(required) = schema_obj.get("required") {
        let arr = require_array(required, "required")?;

        let mut names = std::collections::HashSet::new();
        for item in arr {
            let name = require_string(item, "required field names")?;
            names.insert(name.to_string());
        }
        Ok(names)
    } else {
        Ok(std::collections::HashSet::new())
    }
}

fn parse_object_properties(
    properties: &Map<String, Value>,
    required_names: &std::collections::HashSet<String>,
) -> Result<ObjectProperties, ParseSchemaError> {
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
    let items_schema = schema_obj.get("items").ok_or_else(|| {
        ParseSchemaError::InvalidSchema("Array schema must have an 'items' field".to_string())
    })?;

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

fn parse_array_constraints(
    schema_obj: &Map<String, Value>,
) -> Result<(usize, usize), ParseSchemaError> {
    let min_items = parse_optional_usize_field(schema_obj, "minItems")?.unwrap_or(0);
    let max_items =
        parse_optional_usize_field(schema_obj, "maxItems")?.unwrap_or(/* sane default */ 16);
    Ok((min_items, max_items))
}

fn parse_optional_usize_field(
    schema_obj: &Map<String, Value>,
    field_name: &str,
) -> Result<Option<usize>, ParseSchemaError> {
    if let Some(value) = schema_obj.get(field_name) {
        let number = value.as_u64().ok_or_else(|| {
            ParseSchemaError::InvalidSchema(format!("{} must be a number", field_name))
        })?;
        Ok(Some(number as usize))
    } else {
        Ok(None)
    }
}

// Functional validation combinators

fn get_object_field<'a>(
    obj: &'a Map<String, Value>,
    key: &str,
) -> Result<Option<&'a Map<String, Value>>, ParseSchemaError> {
    obj.get(key)
        .map(|v| {
            v.as_object().ok_or_else(|| {
                ParseSchemaError::InvalidSchema(format!("{} must be an object", key))
            })
        })
        .transpose()
}

fn require_array<'a>(value: &'a Value, context: &str) -> Result<&'a Vec<Value>, ParseSchemaError> {
    value
        .as_array()
        .ok_or_else(|| ParseSchemaError::InvalidSchema(format!("{} must be an array", context)))
}

fn require_object<'a>(
    value: &'a Value,
    context: &str,
) -> Result<&'a Map<String, Value>, ParseSchemaError> {
    value
        .as_object()
        .ok_or_else(|| ParseSchemaError::InvalidSchema(format!("{} must be an object", context)))
}

fn require_string<'a>(value: &'a Value, context: &str) -> Result<&'a str, ParseSchemaError> {
    value
        .as_str()
        .ok_or_else(|| ParseSchemaError::InvalidSchema(format!("{} must be a string", context)))
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
    use crate::schema::{NumberType, StringType};
    use serde_json::json;

    // Test utilities
    fn assert_string_parsing_success(result: Result<SchemaState, ParseSchemaError>) {
        match result {
            Ok(SchemaState::String(_)) => {}
            _ => panic!("Expected string schema to parse successfully"),
        }
    }

    fn assert_number_parsing_success(result: Result<SchemaState, ParseSchemaError>) {
        match result {
            Ok(SchemaState::Number(_)) => {}
            _ => panic!("Expected number schema to parse successfully"),
        }
    }

    fn assert_boolean_parsing_success(result: Result<SchemaState, ParseSchemaError>) {
        match result {
            Ok(SchemaState::Boolean) => {}
            _ => panic!("Expected boolean schema to parse successfully"),
        }
    }

    fn assert_null_parsing_success(result: Result<SchemaState, ParseSchemaError>) {
        match result {
            Ok(SchemaState::Null) => {}
            _ => panic!("Expected null schema to parse successfully"),
        }
    }

    fn assert_string_format(
        result: Result<SchemaState, ParseSchemaError>,
        expected_format: StringType,
    ) {
        match result {
            Ok(SchemaState::String(actual_format)) => {
                assert_eq!(
                    std::mem::discriminant(&actual_format),
                    std::mem::discriminant(&expected_format)
                );
            }
            _ => panic!("Expected string schema with specific format"),
        }
    }

    fn assert_string_constraints(
        result: Result<SchemaState, ParseSchemaError>,
        min_length: Option<usize>,
        max_length: Option<usize>,
    ) {
        match result {
            Ok(SchemaState::String(StringType::Unknown {
                min_length: actual_min,
                max_length: actual_max,
                ..
            })) => {
                assert_eq!(actual_min, min_length, "Min length mismatch");
                assert_eq!(actual_max, max_length, "Max length mismatch");
            }
            _ => panic!("Expected string schema with Unknown type and constraints"),
        }
    }

    fn assert_float_constraints(result: Result<SchemaState, ParseSchemaError>, min: f64, max: f64) {
        match result {
            Ok(SchemaState::Number(NumberType::Float {
                min: actual_min,
                max: actual_max,
            })) => {
                assert_eq!(actual_min, min, "Min value mismatch");
                assert_eq!(actual_max, max, "Max value mismatch");
            }
            _ => panic!("Expected float number schema with constraints"),
        }
    }

    fn assert_integer_constraints(
        result: Result<SchemaState, ParseSchemaError>,
        min: i64,
        max: i64,
    ) {
        match result {
            Ok(SchemaState::Number(NumberType::Integer {
                min: actual_min,
                max: actual_max,
            })) => {
                assert_eq!(actual_min, min, "Min value mismatch");
                assert_eq!(actual_max, max, "Max value mismatch");
            }
            _ => panic!("Expected integer number schema with constraints"),
        }
    }

    mod string_parsing {
        use super::*;

        #[test]
        fn parse_basic_schema() {
            let schema = json!({"type": "string"});
            let result = parse_json_schema(&schema);
            assert_string_parsing_success(result);
        }

        #[test]
        fn parse_with_email_format() {
            let schema = json!({"type": "string", "format": "email"});
            let result = parse_json_schema(&schema);
            assert_string_format(result, StringType::Email);
        }

        #[test]
        fn parse_with_uuid_format() {
            let schema = json!({"type": "string", "format": "uuid"});
            let result = parse_json_schema(&schema);
            assert_string_format(result, StringType::UUID);
        }

        #[test]
        fn parse_with_date_format() {
            let schema = json!({"type": "string", "format": "date"});
            let result = parse_json_schema(&schema);
            assert_string_format(result, StringType::IsoDate);
        }

        #[test]
        fn parse_with_unsupported_format() {
            let schema = json!({"type": "string", "format": "unsupported-format"});
            let result = parse_json_schema(&schema);
            match result {
                Ok(SchemaState::String(StringType::Unknown { .. })) => {}
                _ => panic!("Expected unsupported format to fall back to Unknown string type"),
            }
        }

        #[test]
        fn parse_with_length_constraints() {
            let schema = json!({"type": "string", "minLength": 5, "maxLength": 20});
            let result = parse_json_schema(&schema);
            assert_string_constraints(result, Some(5), Some(20));
        }

        #[test]
        fn parse_with_min_length_only() {
            let schema = json!({"type": "string", "minLength": 10});
            let result = parse_json_schema(&schema);
            assert_string_constraints(result, Some(10), None);
        }

        #[test]
        fn parse_with_max_length_only() {
            let schema = json!({"type": "string", "maxLength": 50});
            let result = parse_json_schema(&schema);
            assert_string_constraints(result, None, Some(50));
        }

        #[test]
        fn parse_enum_schema() {
            let schema = json!({"type": "string", "enum": ["foo", "bar", "baz"]});
            let result = parse_json_schema(&schema);
            match result {
                Ok(SchemaState::String(StringType::Enum { variants })) => {
                    assert_eq!(variants.len(), 3);
                    assert!(variants.contains("foo"));
                    assert!(variants.contains("bar"));
                    assert!(variants.contains("baz"));
                }
                _ => panic!("Expected string enum schema to parse to StringType::Enum"),
            }
        }

        #[test]
        fn parse_empty_enum() {
            let schema = json!({"type": "string", "enum": []});
            let result = parse_json_schema(&schema);
            assert!(result.is_err());
            if let Err(err) = result {
                assert!(err.to_string().contains("enum array cannot be empty"));
            }
        }
    }

    mod number_parsing {
        use super::*;

        #[test]
        fn parse_basic_number_schema() {
            let schema = json!({"type": "number"});
            let result = parse_json_schema(&schema);
            assert_number_parsing_success(result);
        }

        #[test]
        fn parse_basic_integer_schema() {
            let schema = json!({"type": "integer"});
            let result = parse_json_schema(&schema);
            assert_number_parsing_success(result);
        }

        #[test]
        fn parse_number_with_constraints() {
            let schema = json!({"type": "number", "minimum": 1.5, "maximum": 99.9});
            let result = parse_json_schema(&schema);
            assert_float_constraints(result, 1.5, 99.9);
        }

        #[test]
        fn parse_integer_with_constraints() {
            let schema = json!({"type": "integer", "minimum": 0, "maximum": 100});
            let result = parse_json_schema(&schema);
            assert_integer_constraints(result, 0, 100);
        }

        #[test]
        fn parse_number_without_constraints() {
            let schema = json!({"type": "number"});
            let result = parse_json_schema(&schema);
            assert_float_constraints(result, f64::NEG_INFINITY, f64::INFINITY);
        }

        #[test]
        fn parse_integer_without_constraints() {
            let schema = json!({"type": "integer"});
            let result = parse_json_schema(&schema);
            assert_integer_constraints(result, i64::MIN, i64::MAX);
        }

        #[test]
        fn parse_number_with_unsupported_constraints() {
            let schema = json!({"type": "number", "minimum": 5.0, "exclusiveMaximum": 10.0, "multipleOf": 2.5});
            let result = parse_json_schema(&schema);
            assert_float_constraints(result, 5.0, 10.0);
        }

        #[test]
        fn parse_number_with_exclusive_minimum() {
            let schema = json!({"type": "number", "exclusiveMinimum": 1.5});
            let result = parse_json_schema(&schema);
            assert_float_constraints(result, 1.5, f64::INFINITY);
        }

        #[test]
        fn parse_number_with_exclusive_maximum() {
            let schema = json!({"type": "number", "exclusiveMaximum": 99.9});
            let result = parse_json_schema(&schema);
            assert_float_constraints(result, f64::NEG_INFINITY, 99.9);
        }

        #[test]
        fn parse_number_with_exclusive_bounds() {
            let schema =
                json!({"type": "number", "exclusiveMinimum": 0.0, "exclusiveMaximum": 100.0});
            let result = parse_json_schema(&schema);
            assert_float_constraints(result, 0.0, 100.0);
        }

        #[test]
        fn parse_integer_with_exclusive_bounds() {
            let schema = json!({"type": "integer", "exclusiveMinimum": 5, "exclusiveMaximum": 20});
            let result = parse_json_schema(&schema);
            assert_integer_constraints(result, 5, 20);
        }

        #[test]
        fn parse_number_with_both_minimum_and_exclusive_minimum() {
            let schema = json!({"type": "number", "minimum": 5.0, "exclusiveMinimum": 10.0});
            let result = parse_json_schema(&schema);
            assert!(result.is_err());
            if let Err(err) = result {
                assert!(err
                    .to_string()
                    .contains("Cannot specify both minimum and exclusiveMinimum"));
            }
        }

        #[test]
        fn parse_number_with_both_maximum_and_exclusive_maximum() {
            let schema = json!({"type": "number", "maximum": 100.0, "exclusiveMaximum": 50.0});
            let result = parse_json_schema(&schema);
            assert!(result.is_err());
            if let Err(err) = result {
                assert!(err
                    .to_string()
                    .contains("Cannot specify both maximum and exclusiveMaximum"));
            }
        }

        #[test]
        fn parse_number_with_extreme_exclusive_maximum() {
            let schema = json!({"type": "number", "exclusiveMaximum": f64::MAX});
            let result = parse_json_schema(&schema);
            // Should parse successfully without crashing
            assert_float_constraints(result, f64::NEG_INFINITY, f64::MAX);
        }

        #[test]
        fn parse_number_with_extreme_exclusive_minimum() {
            let schema = json!({"type": "number", "exclusiveMinimum": f64::MIN});
            let result = parse_json_schema(&schema);
            // Should parse successfully without crashing
            assert_float_constraints(result, f64::MIN, f64::INFINITY);
        }

        #[test]
        fn parse_integer_with_extreme_exclusive_maximum() {
            let schema = json!({"type": "integer", "exclusiveMaximum": i64::MAX});
            let result = parse_json_schema(&schema);
            // Should parse successfully without crashing
            assert_integer_constraints(result, i64::MIN, i64::MAX);
        }

        #[test]
        fn parse_integer_with_extreme_exclusive_minimum() {
            let schema = json!({"type": "integer", "exclusiveMinimum": i64::MIN});
            let result = parse_json_schema(&schema);
            // Should parse successfully without crashing
            assert_integer_constraints(result, i64::MIN, i64::MAX);
        }
    }

    mod basic_types {
        use super::*;

        #[test]
        fn parse_boolean_schema() {
            let schema = json!({"type": "boolean"});
            let result = parse_json_schema(&schema);
            assert_boolean_parsing_success(result);
        }

        #[test]
        fn parse_null_schema() {
            let schema = json!({"type": "null"});
            let result = parse_json_schema(&schema);
            assert_null_parsing_success(result);
        }
    }

    mod complex_types {
        use super::*;

        #[test]
        fn parse_basic_object_schema() {
            let schema = json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "age": {"type": "integer"}
                },
                "required": ["name"]
            });
            let result = parse_json_schema(&schema);
            match result {
                Ok(SchemaState::Object { required, optional }) => {
                    assert!(required.contains_key("name"));
                    assert!(matches!(required.get("name"), Some(SchemaState::String(_))));
                    assert!(optional.contains_key("age"));
                    assert!(matches!(optional.get("age"), Some(SchemaState::Number(_))));
                }
                _ => panic!("Expected object schema to parse to SchemaState::Object"),
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
                            "id": {"type": "integer"},
                            "email": {"type": "string", "format": "email"}
                        },
                        "required": ["id"]
                    },
                    "active": {"type": "boolean"}
                },
                "required": ["user"]
            });
            let result = parse_json_schema(&schema);
            match result {
                Ok(SchemaState::Object { required, optional }) => {
                    assert!(required.contains_key("user"));
                    match required.get("user") {
                        Some(SchemaState::Object {
                            required: user_required,
                            optional: user_optional,
                        }) => {
                            assert!(user_required.contains_key("id"));
                            assert!(matches!(
                                user_required.get("id"),
                                Some(SchemaState::Number(_))
                            ));
                            assert!(user_optional.contains_key("email"));
                            assert!(matches!(
                                user_optional.get("email"),
                                Some(SchemaState::String(_))
                            ));
                        }
                        _ => panic!("Expected user field to be an object"),
                    }
                    assert!(optional.contains_key("active"));
                    assert!(matches!(optional.get("active"), Some(SchemaState::Boolean)));
                }
                _ => panic!("Expected nested object schema to parse to SchemaState::Object"),
            }
        }

        #[test]
        fn parse_basic_array_schema() {
            let schema = json!({
                "type": "array",
                "items": {"type": "string"},
                "minItems": 1,
                "maxItems": 10
            });
            let result = parse_json_schema(&schema);
            match result {
                Ok(SchemaState::Array {
                    min_length,
                    max_length,
                    schema: item_schema,
                }) => {
                    assert_eq!(min_length, 1);
                    assert_eq!(max_length, 10);
                    assert!(matches!(item_schema.as_ref(), SchemaState::String(_)));
                }
                _ => panic!("Expected array schema to parse to SchemaState::Array"),
            }
        }

        #[test]
        fn parse_array_without_constraints() {
            let schema = json!({
                "type": "array",
                "items": {"type": "integer"}
            });
            let result = parse_json_schema(&schema);
            match result {
                Ok(SchemaState::Array {
                    min_length,
                    max_length,
                    schema: item_schema,
                }) => {
                    assert_eq!(min_length, 0);
                    assert_eq!(max_length, 16);
                    assert!(matches!(item_schema.as_ref(), SchemaState::Number(_)));
                }
                _ => panic!("Expected array without constraints to parse with default bounds"),
            }
        }

        #[test]
        fn parse_nested_array_schema() {
            let schema = json!({
                "type": "array",
                "items": {
                    "type": "array",
                    "items": {"type": "string", "format": "email"},
                    "minItems": 2,
                    "maxItems": 5
                },
                "minItems": 1,
                "maxItems": 3
            });
            let result = parse_json_schema(&schema);
            match result {
                Ok(SchemaState::Array {
                    min_length,
                    max_length,
                    schema: item_schema,
                }) => {
                    assert_eq!(min_length, 1);
                    assert_eq!(max_length, 3);
                    match item_schema.as_ref() {
                        SchemaState::Array {
                            min_length: inner_min,
                            max_length: inner_max,
                            schema: inner_schema,
                        } => {
                            assert_eq!(*inner_min, 2);
                            assert_eq!(*inner_max, 5);
                            assert!(matches!(inner_schema.as_ref(), SchemaState::String(_)));
                        }
                        _ => panic!("Expected nested array structure"),
                    }
                }
                _ => panic!("Expected nested array schema to parse to SchemaState::Array"),
            }
        }

        #[test]
        fn parse_array_of_objects() {
            let schema = json!({
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "integer"},
                        "name": {"type": "string"}
                    },
                    "required": ["id"]
                }
            });
            let result = parse_json_schema(&schema);
            match result {
                Ok(SchemaState::Array {
                    schema: item_schema,
                    ..
                }) => match item_schema.as_ref() {
                    SchemaState::Object { required, optional } => {
                        assert!(required.contains_key("id"));
                        assert!(optional.contains_key("name"));
                    }
                    _ => panic!("Expected array items to be objects"),
                },
                _ => panic!("Expected array of objects to parse correctly"),
            }
        }
    }

    mod nullable_types {
        use super::*;

        #[test]
        fn parse_nullable_string() {
            let schema = json!({"type": ["string", "null"]});
            let result = parse_json_schema(&schema);
            match result {
                Ok(SchemaState::Nullable(inner)) => match inner.as_ref() {
                    SchemaState::String(_) => {}
                    _ => panic!("Expected nullable string to contain string type"),
                },
                _ => panic!("Expected nullable string to parse as SchemaState::Nullable"),
            }
        }

        #[test]
        fn parse_nullable_integer() {
            let schema = json!({"type": ["integer", "null"]});
            let result = parse_json_schema(&schema);
            match result {
                Ok(SchemaState::Nullable(inner)) => match inner.as_ref() {
                    SchemaState::Number(NumberType::Integer { .. }) => {}
                    _ => panic!("Expected nullable integer to contain integer type"),
                },
                _ => panic!("Expected nullable integer to parse as SchemaState::Nullable"),
            }
        }

        #[test]
        fn parse_nullable_array() {
            let schema = json!({
                "type": ["array", "null"],
                "items": {"type": "string"}
            });
            let result = parse_json_schema(&schema);
            match result {
                Ok(SchemaState::Nullable(inner)) => match inner.as_ref() {
                    SchemaState::Array { .. } => {}
                    _ => panic!("Expected nullable array to contain array type"),
                },
                _ => panic!("Expected nullable array to parse as SchemaState::Nullable"),
            }
        }

        #[test]
        fn parse_nullable_object() {
            let schema = json!({
                "type": ["object", "null"],
                "properties": {"name": {"type": "string"}}
            });
            let result = parse_json_schema(&schema);
            match result {
                Ok(SchemaState::Nullable(inner)) => match inner.as_ref() {
                    SchemaState::Object { .. } => {}
                    _ => panic!("Expected nullable object to contain object type"),
                },
                _ => panic!("Expected nullable object to parse as SchemaState::Nullable"),
            }
        }

        #[test]
        fn parse_nullable_reversed_order() {
            let schema = json!({"type": ["null", "string"]});
            let result = parse_json_schema(&schema);
            match result {
                Ok(SchemaState::Nullable(inner)) => match inner.as_ref() {
                    SchemaState::String(_) => {}
                    _ => panic!("Expected nullable string to contain string type"),
                },
                _ => panic!("Expected nullable string to parse as SchemaState::Nullable"),
            }
        }

        #[test]
        fn parse_nullable_anyof_pattern() {
            let schema = json!({
                "anyOf": [
                    {"type": "string"},
                    {"type": "null"}
                ]
            });
            let result = parse_json_schema(&schema);
            match result {
                Ok(SchemaState::Nullable(inner)) => match inner.as_ref() {
                    SchemaState::String(_) => {}
                    _ => panic!("Expected nullable string via anyOf to contain string type"),
                },
                _ => panic!("Expected anyOf nullable pattern to parse as SchemaState::Nullable"),
            }
        }

        #[test]
        fn parse_nullable_oneof_pattern() {
            let schema = json!({
                "oneOf": [
                    {"type": "string"},
                    {"type": "null"}
                ]
            });
            let result = parse_json_schema(&schema);
            match result {
                Ok(SchemaState::Nullable(inner)) => match inner.as_ref() {
                    SchemaState::String(_) => {}
                    _ => panic!("Expected nullable string via oneOf to contain string type"),
                },
                _ => panic!("Expected oneOf nullable pattern to parse as SchemaState::Nullable"),
            }
        }

        #[test]
        fn parse_nullable_anyof_with_constraints() {
            let schema = json!({
                "anyOf": [
                    {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 100
                    },
                    {"type": "null"}
                ]
            });
            let result = parse_json_schema(&schema);
            match result {
                Ok(SchemaState::Nullable(inner)) => match inner.as_ref() {
                    SchemaState::Number(NumberType::Integer { min, max }) => {
                        assert_eq!(*min, 1);
                        assert_eq!(*max, 100);
                    }
                    _ => panic!("Expected nullable integer with constraints"),
                },
                _ => panic!("Expected anyOf nullable integer with constraints to parse correctly"),
            }
        }

        #[test]
        fn parse_nullable_oneof_reversed_order() {
            let schema = json!({
                "oneOf": [
                    {"type": "null"},
                    {"type": "boolean"}
                ]
            });
            let result = parse_json_schema(&schema);
            match result {
                Ok(SchemaState::Nullable(inner)) => match inner.as_ref() {
                    SchemaState::Boolean => {}
                    _ => panic!("Expected nullable boolean"),
                },
                _ => panic!("Expected oneOf nullable boolean to parse correctly"),
            }
        }
    }

    mod error_handling {
        use super::*;

        #[test]
        fn object_with_invalid_properties_field() {
            let schema = json!({"type": "object", "properties": "not_an_object"});
            let result = parse_json_schema(&schema);
            assert!(result.is_err());
            if let Err(err) = result {
                assert!(err.to_string().contains("properties must be an object"));
            }
        }

        #[test]
        fn object_with_invalid_required_field() {
            let schema = json!({"type": "object", "required": "not_an_array"});
            let result = parse_json_schema(&schema);
            assert!(result.is_err());
            if let Err(err) = result {
                assert!(err.to_string().contains("required must be an array"));
            }
        }

        #[test]
        fn string_with_invalid_length_constraints() {
            let schema = json!({"type": "string", "minLength": 10, "maxLength": 5});
            let result = parse_json_schema(&schema);
            assert!(result.is_err());
            if let Err(err) = result {
                assert!(err
                    .to_string()
                    .contains("minLength cannot be greater than maxLength"));
            }
        }

        #[test]
        fn integer_with_invalid_range_constraints() {
            let schema = json!({"type": "integer", "minimum": 100, "maximum": 50});
            let result = parse_json_schema(&schema);
            assert!(result.is_err());
            if let Err(err) = result {
                assert!(err
                    .to_string()
                    .contains("minimum cannot be greater than maximum"));
            }
        }

        #[test]
        fn number_with_invalid_range_constraints() {
            let schema = json!({"type": "number", "minimum": 10.5, "maximum": 5.2});
            let result = parse_json_schema(&schema);
            assert!(result.is_err());
            if let Err(err) = result {
                assert!(err
                    .to_string()
                    .contains("minimum cannot be greater than maximum"));
            }
        }

        #[test]
        fn string_with_empty_enum() {
            let schema = json!({"type": "string", "enum": []});
            let result = parse_json_schema(&schema);
            assert!(result.is_err());
            if let Err(err) = result {
                assert!(err.to_string().contains("enum array cannot be empty"));
            }
        }
    }
}
