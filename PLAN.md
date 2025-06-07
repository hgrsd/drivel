# JSON Schema Parser Implementation Plan

## Goal
Add functionality to parse JSON schemas into SchemaState enums, enabling `drivel` to generate synthetic data from input JSON schemas instead of just inferring from example data.

Target usage: `cat schema.json | drivel produce -n 3`

## Current Understanding
- drivel infers schemas from JSON data → `SchemaState` → generates synthetic data
- Core types: `SchemaState`, `StringType`, `NumberType` in `src/schema.rs`
- Inference logic in `src/infer.rs`, data generation in `src/produce.rs`
- CLI in `src/main.rs` with `describe` and `produce` commands

## Implementation Plan

### Phase 1: Core Parser Foundation
1. **Create `src/parse_schema.rs` module**
   - Add to `src/lib.rs` exports
   - Create `parse_json_schema(json: serde_json::Value) -> Result<SchemaState, ParseError>`
   - Define custom error type for parsing failures

2. **Write tests first** (following TDD approach)
   - Test parsing basic types: string, number, integer, boolean, null
   - Test error cases: invalid schemas, unsupported features
   - Use test-driven development to guide implementation

3. **Implement basic type parsing**
   - Handle primitive JSON Schema types
   - Map to corresponding `SchemaState` variants

### Phase 2: Complex Schema Support
4. **Object schema support**
   - Parse `properties`, `required`, `additionalProperties`
   - Map to `SchemaState::Object` with required/optional fields
   - Handle nested object schemas

5. **Array schema support**
   - Parse `items`, `minItems`, `maxItems`
   - Map to `SchemaState::Array`
   - Support for nested array schemas

### Phase 3: JSON Schema Features
6. **String format support**
   - Map JSON Schema `format` to `StringType` variants
   - Support: email, uuid, date-time, date, hostname, uri
   - Handle `minLength`, `maxLength`

7. **Number/integer constraints**
   - Parse `minimum`, `maximum` for numbers
   - Map to `NumberType::Integer` and `NumberType::Float`

8. **Enum support**
   - Parse `enum` keyword
   - Map to `StringType::Enum` variants

9. **Nullable types**
   - Handle `type: ["string", "null"]` patterns
   - Map to `SchemaState::Nullable`

### Phase 4: CLI Integration
10. **Modify CLI to detect JSON Schema input**
    - Check for `$schema` field to detect JSON Schema
    - Alternative: Add explicit `--from-schema` flag
    - Maintain backward compatibility

11. **Update main.rs flow**
    - Branch logic: if JSON Schema detected → parse schema, else → infer schema
    - Keep existing functionality intact

### Phase 5: Testing & Documentation
12. **Integration tests**
    - End-to-end tests: JSON Schema → synthetic data
    - Test with various schema complexities
    - Verify generated data matches schema constraints

13. **Documentation updates**
    - Update README with new functionality
    - Add examples showing schema → data generation
    - Document supported JSON Schema features

## Technical Decisions

### JSON Schema Detection Strategy
**Option 1**: Auto-detect based on `$schema` field presence
**Option 2**: Add explicit CLI flag `--from-schema`

**Recommendation**: Start with auto-detection for simplicity, add flag if needed

### Error Handling
- Return `Result<SchemaState, ParseError>` for graceful error handling
- Provide clear error messages for unsupported features
- Fall back to reasonable defaults where possible

### Unsupported Features Strategy
- Document clearly what JSON Schema features are not supported
- Fail gracefully with helpful error messages
- Consider future extensibility

## Commit Strategy
Following incremental development:
1. Add parser module with basic tests
2. Implement primitive type parsing
3. Add object schema support
4. Add array schema support  
5. Add format and constraint support
6. Integrate with CLI
7. Add integration tests
8. Update documentation

Each commit should be functional and tested.

## Questions for User
1. ~~Preference for JSON Schema detection: auto-detect vs explicit flag?~~ → **DECIDED: Auto-detect based on `$schema` field**
2. ~~How to handle unsupported JSON Schema features? (error vs ignore vs default)~~ → **DECIDED: Ignore unsupported features, warn to stderr**
3. ~~Should we validate input JSON Schema before parsing?~~ → **DECIDED: Yes, validate first**
4. Any specific JSON Schema draft version to target? (default: draft 2020-12)

## User Decisions Made
- **Unsupported features**: Ignore and warn to stderr (don't break unix pipes)
- **Validation**: Validate JSON Schema first before parsing
- **TDD approach**: Focus on interface first, write failing test, then implement
- **Method signature**: Approved interface design

## Approved Interface Design
```rust
pub fn parse_json_schema(schema_json: &serde_json::Value) -> Result<SchemaState, ParseSchemaError>

#[derive(Debug, thiserror::Error)]
pub enum ParseSchemaError {
    #[error("Invalid JSON Schema: {0}")]
    InvalidSchema(String),
    #[error("Unsupported JSON Schema feature: {0}")]
    UnsupportedFeature(String),
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
}
```

## Current Status
- [x] Research existing codebase
- [x] Create implementation plan
- [x] Set up todo tracking
- [x] Gathered user feedback and decisions
- [x] Design parser interface/method signature
- [x] Got approval for interface design
- [x] Create parse_schema.rs module with stub
- [x] Write first failing test for basic string parsing
- [x] Implement basic string type parsing (TDD cycle 1 complete)
- [x] Commit: "Add JSON schema parser foundation with basic string type support"

## Implementation Progress
**Phase 1: Core Parser Foundation** ✅ COMPLETED
1. ✅ Created `src/parse_schema.rs` module with exports in `lib.rs`
2. ✅ Implemented `parse_json_schema()` function with validation
3. ✅ Added comprehensive error handling with `ParseSchemaError`
4. ✅ Basic string type parsing: `{"type": "string"}` → `SchemaState::String`
5. ✅ String format support: email, uuid, date, date-time, uri, hostname
6. ✅ String length constraints: minLength, maxLength parsing
7. ✅ Graceful handling of unsupported formats with stderr warnings
8. ✅ Code refactoring for maintainability and single responsibility
9. ✅ Comprehensive test coverage (8 test cases covering all string features)
10. ✅ TDD workflow established and validated
11. ✅ Commit: "Add comprehensive string schema parsing with format and constraint support"
12. ✅ Number/integer type parsing implementation with constraints
13. ✅ Support for min/max constraints for both number and integer types
14. ✅ Warning system for unsupported constraints (exclusiveMinimum, exclusiveMaximum, multipleOf)
15. ✅ Comprehensive test coverage for number parsing (6 test cases)
16. ✅ All 83 tests passing
17. ✅ Boolean and null type parsing implementation
18. ✅ Added tests for boolean and null types
19. ✅ All 85 tests passing
20. ✅ Commit: "Add boolean and null type parsing support"
21. ✅ String enum support implementation
22. ✅ Add parse_string_enum function with comprehensive validation
23. ✅ All 87 tests passing (2 new enum tests added)
24. ✅ Commit: "Add string enum support to JSON schema parser"

**Phase 2: Additional Type Support** ✅ COMPLETED
- [x] Add number/integer type parsing with min/max constraints
- [x] Handle exclusive bounds and multipleOf with warnings
- [x] Add boolean type support
- [x] Add null type support
- [x] Add enum support for string types

**Phase 3: Complex Schema Support** ✅ COMPLETED
- [x] Object schema parsing with properties and required fields
- [x] Nested object schema support (recursive parsing)
- [x] Array schema parsing with items schema
- [x] Array constraints (minItems/maxItems) support
- [x] Nested array schema support
- [x] Array of objects support
- [x] Warning system for unsupported object/array features
- [x] Comprehensive test coverage (5 new tests: basic object, nested object, basic array, array without constraints, nested array, array of objects)
- [x] All 93 tests passing
- [x] Refactoring: Extract helper functions for cleaner code organization
  - `parse_required_field_names()` for object required field parsing
  - `parse_object_properties()` for property categorization
  - `parse_optional_usize_field()` for common numeric constraint parsing
  - Simplified array and string constraint parsing
- [x] All 93 tests still passing after refactoring
- [x] Commit: "Add object and array schema parsing with full nesting support"

**Current Priority:** Begin Phase 4 - JSON Schema Features (nullable types, etc.)

**Future Phases:**
- Phase 4: JSON Schema Features (nullable types, etc.)
- Phase 5: CLI Integration and Testing