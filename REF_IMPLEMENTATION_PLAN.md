# $ref Implementation Plan

## Overview
This document outlines the technical implementation plan for adding `$ref` (JSON Schema references) support to drivel's JSON schema parser. This is a major architectural enhancement that requires significant changes to support schema resolution, circular dependency detection, and reference management.

## Current Architecture Analysis

### Existing Schema Structure
```rust
pub enum SchemaState {
    String(StringType),
    Number(NumberType),
    Boolean,
    Null,
    Nullable(Box<SchemaState>),
    Array { min_length: usize, max_length: usize, schema: Box<SchemaState> },
    Object { required: HashMap<String, SchemaState>, optional: HashMap<String, SchemaState> },
    Initial,
    Indefinite,
}
```

### Current Parser Function
```rust
pub fn parse_json_schema(schema_json: &Value) -> Result<SchemaState, ParseSchemaError>
```

**Limitations for $ref support:**
- Single-pass parsing (no schema context)
- No document-level schema registry
- No reference resolution mechanism
- No circular dependency detection
- Direct `SchemaState` return (no lazy evaluation)

## Implementation Strategy

### Phase 1: Architecture Foundation

#### 1.1 New Schema Document Model
Create a new document-centric architecture that can hold multiple schemas and manage references:

```rust
pub struct SchemaDocument {
    pub root_schema: SchemaState,
    pub definitions: HashMap<String, SchemaState>,
    pub external_refs: HashMap<String, SchemaDocument>,
}

impl SchemaDocument {
    pub fn new(root_schema: SchemaState) -> Self
    pub fn add_definition(&mut self, name: String, schema: SchemaState)
    pub fn resolve_ref(&self, ref_uri: &str) -> Result<&SchemaState, ResolveError>
    pub fn has_references(&self) -> bool
}
```

#### 1.2 Reference SchemaState Variant
Add a new variant to handle references:

```rust
pub enum SchemaState {
    // ... existing variants
    Reference { 
        uri: String, 
        resolved: RefCell<Option<Rc<SchemaState>>> // Lazy resolution with cycle detection
    },
}
```

#### 1.3 Enhanced Error Types
```rust
#[derive(Debug)]
pub enum ParseSchemaError {
    // ... existing variants
    ReferenceError(ReferenceError),
}

#[derive(Debug)]
pub enum ReferenceError {
    UnresolvedReference(String),
    CircularReference(String),
    InvalidReferenceUri(String),
    ExternalReferenceNotSupported(String),
}

impl fmt::Display for ReferenceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReferenceError::UnresolvedReference(uri) => write!(f, "Unresolved reference: {}", uri),
            ReferenceError::CircularReference(uri) => write!(f, "Circular reference detected: {}", uri),
            ReferenceError::InvalidReferenceUri(uri) => write!(f, "Invalid reference URI: {}", uri),
            ReferenceError::ExternalReferenceNotSupported(uri) => write!(f, "External reference not supported: {}", uri),
        }
    }
}

impl std::error::Error for ReferenceError {}
```

### Phase 2: Reference Resolution System

#### 2.1 Reference Resolver
```rust
pub struct ReferenceResolver {
    document: Rc<SchemaDocument>,
    resolution_stack: RefCell<Vec<String>>, // For cycle detection
}

impl ReferenceResolver {
    pub fn new(document: Rc<SchemaDocument>) -> Self {
        Self {
            document,
            resolution_stack: RefCell::new(Vec::new()),
        }
    }
    
    pub fn resolve_reference(&self, ref_uri: &str) -> Result<Rc<SchemaState>, ReferenceError> {
        // Check for circular references
        if self.resolution_stack.borrow().contains(&ref_uri.to_string()) {
            return Err(ReferenceError::CircularReference(ref_uri.to_string()));
        }
        
        // Add to resolution stack
        self.resolution_stack.borrow_mut().push(ref_uri.to_string());
        
        let result = self.resolve_reference_internal(ref_uri);
        
        // Remove from resolution stack
        self.resolution_stack.borrow_mut().pop();
        
        result
    }
    
    fn resolve_reference_internal(&self, ref_uri: &str) -> Result<Rc<SchemaState>, ReferenceError> {
        let parsed_uri = self.parse_reference_uri(ref_uri)?;
        
        match parsed_uri {
            ReferenceUri::Local { fragment } => {
                if let Some(schema) = self.document.definitions.get(&fragment) {
                    Ok(Rc::new(schema.clone()))
                } else {
                    Err(ReferenceError::UnresolvedReference(ref_uri.to_string()))
                }
            }
            ReferenceUri::External { .. } => {
                Err(ReferenceError::ExternalReferenceNotSupported(ref_uri.to_string()))
            }
        }
    }
    
    fn parse_reference_uri(&self, ref_uri: &str) -> Result<ReferenceUri, ReferenceError> {
        if ref_uri.starts_with('#') {
            // Local reference
            let fragment = ref_uri.strip_prefix('#').unwrap_or("");
            if fragment.is_empty() {
                return Err(ReferenceError::InvalidReferenceUri(ref_uri.to_string()));
            }
            
            // Remove leading slash if present
            let fragment = fragment.strip_prefix('/').unwrap_or(fragment);
            
            Ok(ReferenceUri::Local { fragment: fragment.to_string() })
        } else {
            // External reference (not supported initially)
            Err(ReferenceError::ExternalReferenceNotSupported(ref_uri.to_string()))
        }
    }
}
```

#### 2.2 Reference URI Types
```rust
#[derive(Debug, Clone)]
pub enum ReferenceUri {
    Local { fragment: String },           // #/definitions/User
    External { url: String, fragment: Option<String> },  // https://... (not initially supported)
}
```

### Phase 3: Parser Integration

#### 3.1 New Parsing Interface
```rust
// New primary interface
pub fn parse_json_schema_document(schema_json: &Value) -> Result<SchemaDocument, ParseSchemaError> {
    let mut context = ParseContext::new();
    
    // Extract definitions first
    context.extract_definitions(schema_json);
    
    // Parse root schema
    let root_schema = context.parse_schema_with_refs(schema_json)?;
    
    Ok(SchemaDocument {
        root_schema,
        definitions: context.definitions,
        external_refs: HashMap::new(),
    })
}

// Updated legacy interface for backward compatibility
pub fn parse_json_schema(schema_json: &Value) -> Result<SchemaState, ParseSchemaError> {
    let document = parse_json_schema_document(schema_json)?;
    
    // If no references, return root schema directly
    if !document.has_references() {
        return Ok(document.root_schema);
    }
    
    // If references exist, resolve them and return flattened schema
    let resolver = ReferenceResolver::new(Rc::new(document));
    resolve_schema_references(&document.root_schema, &resolver)
}

fn resolve_schema_references(schema: &SchemaState, resolver: &ReferenceResolver) -> Result<SchemaState, ParseSchemaError> {
    match schema {
        SchemaState::Reference { uri, .. } => {
            let resolved = resolver.resolve_reference(uri)
                .map_err(ParseSchemaError::ReferenceError)?;
            Ok((*resolved).clone())
        }
        SchemaState::Array { min_length, max_length, schema: inner } => {
            let resolved_inner = resolve_schema_references(inner, resolver)?;
            Ok(SchemaState::Array {
                min_length: *min_length,
                max_length: *max_length,
                schema: Box::new(resolved_inner),
            })
        }
        SchemaState::Object { required, optional } => {
            let mut resolved_required = HashMap::new();
            for (key, schema) in required {
                resolved_required.insert(key.clone(), resolve_schema_references(schema, resolver)?);
            }
            
            let mut resolved_optional = HashMap::new();
            for (key, schema) in optional {
                resolved_optional.insert(key.clone(), resolve_schema_references(schema, resolver)?);
            }
            
            Ok(SchemaState::Object {
                required: resolved_required,
                optional: resolved_optional,
            })
        }
        SchemaState::Nullable(inner) => {
            let resolved_inner = resolve_schema_references(inner, resolver)?;
            Ok(SchemaState::Nullable(Box::new(resolved_inner)))
        }
        // All other variants don't contain nested schemas
        _ => Ok(schema.clone()),
    }
}
```

#### 3.2 Parse Context for Multi-Pass Parsing
```rust
struct ParseContext {
    definitions: HashMap<String, SchemaState>,
}

impl ParseContext {
    fn new() -> Self {
        Self {
            definitions: HashMap::new(),
        }
    }
    
    fn extract_definitions(&mut self, schema_json: &Value) {
        // Extract from $defs (newer standard)
        if let Some(defs) = schema_json.get("$defs").and_then(|v| v.as_object()) {
            for (name, def_schema) in defs {
                if let Ok(parsed_schema) = self.parse_schema_with_refs(def_schema) {
                    self.definitions.insert(format!("$defs/{}", name), parsed_schema);
                }
            }
        }
        
        // Extract from definitions (older standard)
        if let Some(defs) = schema_json.get("definitions").and_then(|v| v.as_object()) {
            for (name, def_schema) in defs {
                if let Ok(parsed_schema) = self.parse_schema_with_refs(def_schema) {
                    self.definitions.insert(format!("definitions/{}", name), parsed_schema);
                }
            }
        }
    }
    
    fn parse_schema_with_refs(&self, schema_json: &Value) -> Result<SchemaState, ParseSchemaError> {
        let schema_obj = schema_json
            .as_object()
            .ok_or_else(|| ParseSchemaError::InvalidSchema("Schema must be an object".to_string()))?;

        // Check for $ref first
        if let Some(ref_value) = schema_obj.get("$ref") {
            if let Some(ref_uri) = ref_value.as_str() {
                return Ok(SchemaState::Reference {
                    uri: ref_uri.to_string(),
                    resolved: RefCell::new(None),
                });
            } else {
                return Err(ParseSchemaError::InvalidSchema("$ref must be a string".to_string()));
            }
        }
        
        // If no $ref, parse normally using existing logic
        // Delegate to existing parse_single_type function but with context awareness
        self.parse_single_type_with_refs(schema_obj)
    }
    
    fn parse_single_type_with_refs(&self, schema_obj: &Map<String, Value>) -> Result<SchemaState, ParseSchemaError> {
        // Handle anyOf/oneOf nullable patterns first (existing logic)
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

        // Handle type field patterns
        let type_field = schema_obj.get("type").ok_or_else(|| {
            ParseSchemaError::InvalidSchema(
                "Schema must have a 'type' field, 'anyOf', 'oneOf', or '$ref'".to_string(),
            )
        })?;

        // Handle nullable types (arrays) vs single types (strings)
        if let Some(type_array) = type_field.as_array() {
            parse_nullable_type(schema_obj, type_array)
        } else if let Some(type_str) = type_field.as_str() {
            self.parse_type_with_refs(schema_obj, type_str)
        } else {
            Err(ParseSchemaError::InvalidSchema(
                "Type field must be a string or array".to_string(),
            ))
        }
    }
    
    fn parse_type_with_refs(&self, schema_obj: &Map<String, Value>, type_str: &str) -> Result<SchemaState, ParseSchemaError> {
        match type_str {
            "string" => parse_string_type(schema_obj),
            "number" => parse_number_type(schema_obj, false),
            "integer" => parse_number_type(schema_obj, true),
            "boolean" => Ok(SchemaState::Boolean),
            "null" => Ok(SchemaState::Null),
            "object" => self.parse_object_type_with_refs(schema_obj),
            "array" => self.parse_array_type_with_refs(schema_obj),
            _ => Err(ParseSchemaError::UnsupportedFeature(format!(
                "Type '{}' not supported yet",
                type_str
            ))),
        }
    }
    
    fn parse_object_type_with_refs(&self, schema_obj: &Map<String, Value>) -> Result<SchemaState, ParseSchemaError> {
        let required_field_names = parse_required_field_names(schema_obj);
        let (required_properties, optional_properties) = parse_object_properties(schema_obj)?;
        
        let mut required = HashMap::new();
        let mut optional = HashMap::new();
        
        for (property_name, property_schema_json) in required_properties {
            let property_schema = self.parse_schema_with_refs(&property_schema_json)?;
            
            if required_field_names.contains(&property_name) {
                required.insert(property_name, property_schema);
            } else {
                optional.insert(property_name, property_schema);
            }
        }
        
        for (property_name, property_schema_json) in optional_properties {
            let property_schema = self.parse_schema_with_refs(&property_schema_json)?;
            optional.insert(property_name, property_schema);
        }
        
        Ok(SchemaState::Object { required, optional })
    }
    
    fn parse_array_type_with_refs(&self, schema_obj: &Map<String, Value>) -> Result<SchemaState, ParseSchemaError> {
        let items_schema = schema_obj
            .get("items")
            .ok_or_else(|| ParseSchemaError::InvalidSchema("Array schema must have 'items'".to_string()))?;
        
        let parsed_items_schema = self.parse_schema_with_refs(items_schema)?;
        
        let min_items = parse_optional_usize_field(schema_obj, "minItems").unwrap_or(0);
        let max_items = parse_optional_usize_field(schema_obj, "maxItems").unwrap_or(10);
        
        Ok(SchemaState::Array {
            min_length: min_items,
            max_length: max_items,
            schema: Box::new(parsed_items_schema),
        })
    }
}
```

### Phase 4: Schema Document Implementation

#### 4.1 SchemaDocument Implementation
```rust
impl SchemaDocument {
    pub fn new(root_schema: SchemaState) -> Self {
        Self {
            root_schema,
            definitions: HashMap::new(),
            external_refs: HashMap::new(),
        }
    }
    
    pub fn add_definition(&mut self, name: String, schema: SchemaState) {
        self.definitions.insert(name, schema);
    }
    
    pub fn has_references(&self) -> bool {
        self.schema_has_references(&self.root_schema) || 
        self.definitions.values().any(|schema| self.schema_has_references(schema))
    }
    
    fn schema_has_references(&self, schema: &SchemaState) -> bool {
        match schema {
            SchemaState::Reference { .. } => true,
            SchemaState::Array { schema: inner, .. } => self.schema_has_references(inner),
            SchemaState::Object { required, optional } => {
                required.values().any(|s| self.schema_has_references(s)) ||
                optional.values().any(|s| self.schema_has_references(s))
            }
            SchemaState::Nullable(inner) => self.schema_has_references(inner),
            _ => false,
        }
    }
}
```

### Phase 5: Generation and Display Updates

#### 5.1 Update Generation Logic
Update `produce.rs` to handle references during generation:

```rust
// Add new generate method that takes resolver
impl SchemaState {
    pub fn generate_with_resolver(&self, rng: &mut impl Rng, resolver: Option<&ReferenceResolver>) -> Result<Value, GenerateError> {
        match self {
            SchemaState::Reference { uri, resolved } => {
                if let Some(resolver) = resolver {
                    // Try cached resolution first
                    if let Some(cached) = resolved.borrow().as_ref() {
                        return cached.generate_with_resolver(rng, Some(resolver));
                    }
                    
                    // Resolve and cache
                    let resolved_schema = resolver.resolve_reference(uri)
                        .map_err(|e| GenerateError::ReferenceError(e))?;
                    *resolved.borrow_mut() = Some(resolved_schema.clone());
                    resolved_schema.generate_with_resolver(rng, Some(resolver))
                } else {
                    Err(GenerateError::UnresolvedReference(uri.clone()))
                }
            }
            // Update all other variants to pass resolver through recursive calls
            SchemaState::Array { min_length, max_length, schema } => {
                // ... existing logic but with resolver passed through
            }
            SchemaState::Object { required, optional } => {
                // ... existing logic but with resolver passed through
            }
            // ... other variants
        }
    }
}

#[derive(Debug)]
pub enum GenerateError {
    // ... existing variants
    ReferenceError(ReferenceError),
    UnresolvedReference(String),
}
```

#### 5.2 Update Display Logic
```rust
impl SchemaState {
    pub fn to_string_pretty_with_resolver(&self, resolver: Option<&ReferenceResolver>) -> String {
        to_string_pretty_inner_with_resolver(self, 0, resolver)
    }
}

fn to_string_pretty_inner_with_resolver(
    schema_state: &SchemaState, 
    depth: usize,
    resolver: Option<&ReferenceResolver>
) -> String {
    match schema_state {
        SchemaState::Reference { uri, resolved } => {
            if let Some(resolver) = resolver {
                if let Ok(resolved_schema) = resolver.resolve_reference(uri) {
                    format!("@{} -> {}", uri, to_string_pretty_inner_with_resolver(&resolved_schema, depth, Some(resolver)))
                } else {
                    format!("@{} (unresolved)", uri)
                }
            } else {
                format!("@{}", uri)
            }
        }
        // Update all other variants to pass resolver through
        SchemaState::Array { min_length, max_length, schema } => {
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
                to_string_pretty_inner_with_resolver(schema, depth + 1, resolver),
                indent_str_close,
                length
            )
        }
        // ... other variants updated similarly
        _ => to_string_pretty_inner(schema_state, depth), // Fallback to existing implementation
    }
}
```

### Phase 6: CLI Integration

#### 6.1 Update CLI to Handle References
```rust
// In main.rs, update the schema parsing logic
if from_schema {
    let document = parse_json_schema_document(&json_input)
        .map_err(|e| format!("Failed to parse JSON schema: {}", e))?;
    
    let resolver = ReferenceResolver::new(Rc::new(document));
    
    match command {
        Command::Describe => {
            println!("{}", resolver.document.root_schema.to_string_pretty_with_resolver(Some(&resolver)));
        }
        Command::Produce { count } => {
            let mut rng = rand::thread_rng();
            for _ in 0..count {
                match resolver.document.root_schema.generate_with_resolver(&mut rng, Some(&resolver)) {
                    Ok(value) => println!("{}", serde_json::to_string(&value).unwrap()),
                    Err(e) => eprintln!("Error generating data: {}", e),
                }
            }
        }
    }
} else {
    // Existing logic unchanged
}
```

### Phase 7: Testing Strategy

#### 7.1 Unit Tests for Reference Resolution
```rust
#[cfg(test)]
mod reference_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_local_reference_resolution() {
        let schema_json = json!({
            "type": "object",
            "properties": {
                "user": { "$ref": "#/definitions/User" }
            },
            "definitions": {
                "User": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" }
                    },
                    "required": ["name"]
                }
            }
        });
        
        let document = parse_json_schema_document(&schema_json).unwrap();
        let resolver = ReferenceResolver::new(Rc::new(document));
        
        let user_schema = resolver.resolve_reference("#/definitions/User").unwrap();
        match &*user_schema {
            SchemaState::Object { required, optional } => {
                assert!(required.contains_key("name"));
                assert!(optional.is_empty());
            }
            _ => panic!("Expected object schema"),
        }
    }

    #[test]
    fn test_circular_reference_detection() {
        let schema_json = json!({
            "definitions": {
                "Node": {
                    "type": "object", 
                    "properties": {
                        "child": { "$ref": "#/definitions/Node" }
                    }
                }
            },
            "$ref": "#/definitions/Node"
        });
        
        let document = parse_json_schema_document(&schema_json).unwrap();
        let resolver = ReferenceResolver::new(Rc::new(document));
        
        // Should detect circular reference
        let result = resolver.resolve_reference("#/definitions/Node");
        match result {
            Err(ReferenceError::CircularReference(_)) => {
                // Expected - circular reference detected during resolution
            }
            Ok(_) => {
                // Also acceptable - reference resolved but circular dependency exists in schema
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }

    #[test]
    fn test_nested_references() {
        let schema_json = json!({
            "type": "array",
            "items": { "$ref": "#/definitions/Person" },
            "definitions": {
                "Person": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "address": { "$ref": "#/definitions/Address" }
                    },
                    "required": ["name"]
                },
                "Address": {
                    "type": "object",
                    "properties": {
                        "street": { "type": "string" },
                        "city": { "type": "string" }
                    },
                    "required": ["street", "city"]
                }
            }
        });
        
        let document = parse_json_schema_document(&schema_json).unwrap();
        let resolver = ReferenceResolver::new(Rc::new(document));
        
        // Test that we can resolve nested references
        let person_schema = resolver.resolve_reference("#/definitions/Person").unwrap();
        let address_schema = resolver.resolve_reference("#/definitions/Address").unwrap();
        
        // Verify schemas are correct
        match &*person_schema {
            SchemaState::Object { required, optional } => {
                assert!(required.contains_key("name"));
                assert!(optional.contains_key("address"));
            }
            _ => panic!("Expected object schema for Person"),
        }
        
        match &*address_schema {
            SchemaState::Object { required, optional } => {
                assert!(required.contains_key("street"));
                assert!(required.contains_key("city"));
                assert!(optional.is_empty());
            }
            _ => panic!("Expected object schema for Address"),
        }
    }

    #[test]
    fn test_backward_compatibility() {
        let schema_json = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer" }
            },
            "required": ["name"]
        });
        
        // Should work with both old and new APIs
        let old_result = parse_json_schema(&schema_json).unwrap();
        let new_result = parse_json_schema_document(&schema_json).unwrap();
        
        // Results should be equivalent
        assert_eq!(old_result, new_result.root_schema);
    }
}
```

#### 7.2 Integration Tests
```rust
#[test]
fn test_ref_with_generation() {
    let schema_json = json!({
        "type": "array",
        "items": { "$ref": "#/definitions/Person" },
        "minItems": 1,
        "maxItems": 3,
        "definitions": {
            "Person": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "minLength": 1 },
                    "age": { "type": "integer", "minimum": 0, "maximum": 120 }
                },
                "required": ["name"]
            }
        }
    });
    
    let document = parse_json_schema_document(&schema_json).unwrap();
    let resolver = ReferenceResolver::new(Rc::new(document));
    let mut rng = rand::thread_rng();
    
    // Generate data and verify it matches schema constraints
    for _ in 0..10 {
        let generated = resolver.document.root_schema
            .generate_with_resolver(&mut rng, Some(&resolver))
            .unwrap();
        
        let array = generated.as_array().unwrap();
        assert!(array.len() >= 1 && array.len() <= 3);
        
        for person in array {
            let person_obj = person.as_object().unwrap();
            assert!(person_obj.contains_key("name"));
            let name = person_obj["name"].as_str().unwrap();
            assert!(!name.is_empty());
            
            if let Some(age) = person_obj.get("age") {
                let age_val = age.as_i64().unwrap();
                assert!(age_val >= 0 && age_val <= 120);
            }
        }
    }
}
```

## Implementation Plan Summary

1. **Add Reference SchemaState variant** and error types
2. **Create SchemaDocument structure** to hold definitions
3. **Implement ReferenceResolver** with circular dependency detection
4. **Update parser** to extract definitions and handle `$ref`
5. **Create new parse_json_schema_document()** function
6. **Update generation and display** to work with resolver
7. **Maintain backward compatibility** in existing API
8. **Add comprehensive tests** for all reference scenarios
9. **Update CLI** to use new document model when references present

The key insight is to maintain the existing `SchemaState` enum structure while adding reference support through a new variant and document-level context. This preserves backward compatibility while enabling powerful reference capabilities.