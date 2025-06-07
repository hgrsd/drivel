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
**Phase 1: Core Parser Foundation** ✅ STARTED
1. ✅ Created `src/parse_schema.rs` module with exports in `lib.rs`
2. ✅ Implemented `parse_json_schema()` function with validation
3. ✅ Added comprehensive error handling with `ParseSchemaError`
4. ✅ Basic string type parsing: `{"type": "string"}` → `SchemaState::String`
5. ✅ Test coverage for basic string schema parsing
6. ✅ TDD workflow established and validated

**Next Steps:**
- Add support for string format parsing (email, uuid, date, etc.)
- Add string length constraints (minLength, maxLength)
- Add tests and implementation for number/integer types
- Add boolean and null type support