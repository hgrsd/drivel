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
1. ~~Preference for JSON Schema detection: auto-detect vs explicit flag?~~ → **DECIDED: Explicit `--from-schema` flag (UPDATED)**
2. ~~How to handle unsupported JSON Schema features? (error vs ignore vs default)~~ → **DECIDED: Ignore unsupported features, warn to stderr**
3. ~~Should we validate input JSON Schema before parsing?~~ → **DECIDED: Yes, validate first**
4. ~~Any specific JSON Schema draft version to target? (default: draft 2020-12)~~ → **DECIDED: Any valid JSON Schema**
5. ~~Testing approach for CLI integration?~~ → **DECIDED: Command-line testing instead of integration tests**

## User Decisions Made
- **Schema detection**: Use explicit `--from-schema` flag instead of auto-detection
- **Unsupported features**: Ignore and warn to stderr (don't break unix pipes)
- **Validation**: Validate JSON Schema first before parsing
- **TDD approach**: Focus on interface first, write failing test, then implement
- **Method signature**: Approved interface design
- **Testing approach**: Manual command-line testing instead of automated integration tests

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

**Phase 4: JSON Schema Features** ✅ COMPLETED
- [x] Nullable type support implementation
  - [x] Array notation: `{"type": ["string", "null"]}`
  - [x] anyOf pattern: `{"anyOf": [{"type": "string"}, {"type": "null"}]}`
  - [x] oneOf pattern: `{"oneOf": [{"type": "string"}, {"type": "null"}]}`
  - [x] Support for constraints on non-null types
  - [x] Order-independent parsing (null can be first or second)
  - [x] Comprehensive test coverage (10 nullable test cases)
  - [x] All 102 tests passing
- [x] Commit: "Add comprehensive nullable type support to JSON schema parser"

**Phase 5: CLI Integration and Testing** ✅ COMPLETED
- [x] Add `--from-schema` global flag to indicate input is JSON Schema
- [x] Update CLI argument parsing to handle this flag  
- [x] Maintain backward compatibility (default behavior unchanged)
- [x] Add conditional branching in main.rs:
  - [x] If `--from-schema` flag → use `parse_json_schema()`
  - [x] Else → use existing `infer_schema()` workflow
- [x] Handle schema parsing errors gracefully with user-friendly messages
- [x] Ignore enum inference options when `--from-schema` is used (not applicable)
- [x] **BUG FIX**: Fix array generation to respect minItems/maxItems constraints
  - [x] Arrays now properly respect schema constraints when no `-n` specified
  - [x] `-n` flag still overrides constraints when explicitly provided
  - [x] Tested with nested arrays and complex object hierarchies
- [x] Manual command-line testing completed:
  - [x] Test `describe` command with JSON Schema input
  - [x] Test `produce` command with JSON Schema input  
  - [x] Test various schema complexities (basic types, objects, arrays, nullable)
  - [x] Test nested arrays and nested objects
  - [x] Test error cases (invalid schemas, unsupported features)
  - [x] Verify generated data matches schema constraints
  - [x] Ensure backward compatibility (existing workflows still work)
- [x] All 102 tests passing
- [x] Commit: "Add CLI integration with --from-schema flag and fix array minItems bug"

**Working Usage:**
```bash
# New functionality
cat schema.json | drivel --from-schema produce -n 3
cat schema.json | drivel --from-schema describe

# Existing functionality (unchanged)  
cat data.json | drivel produce -n 3
cat data.json | drivel describe
```

**Phase 6: Bug Bash** 📋 PLANNED
- [ ] Comprehensive testing of edge cases and boundary conditions
- [ ] Test with real-world JSON Schema examples
- [ ] Stress test with large and complex schemas
- [ ] Validate constraint handling across all types
- [ ] Test error handling and user experience
- [ ] Performance testing with large schemas
- [ ] Identify and fix any remaining issues

**Phase 7: Test Suite Improvements** ✅ COMPLETED
- [x] Analyze test suite structure and organization  
- [x] Identify code duplication patterns and extraction opportunities
- [x] Review test coverage for edge cases and missing scenarios
- [x] Evaluate assertion quality and error message clarity
- [x] Create specific improvement recommendations
- [x] Implement structural organization (test modules)
- [x] Extract test utilities and reduce duplication
- [x] Improve assertion specificity and error messages
- [x] Verify all tests pass after improvements

**Phase 8: Simplifications** ✅ COMPLETED
**Goal**: Remove speculative min/max constraints from JSON schema outputs while keeping enum inference

**Analysis**: Currently the `ToJsonSchema` implementations for `NumberType` variants output `minimum` and `maximum` fields which are too speculative for JSON schema output. These constraints are useful internally for data generation but shouldn't appear in schema output.

**Implementation Steps**:
1. ✅ **Write failing tests first** (TDD approach)
   - ✅ Test that integer schemas don't include minimum/maximum in JSON output
   - ✅ Test that float schemas don't include minimum/maximum in JSON output
   - ✅ Test that string length constraints are also removed from Unknown string types
   - ✅ Test that enum inference is preserved (should still work)

2. ✅ **Update NumberType ToJsonSchema implementation**
   - ✅ Remove `minimum` and `maximum` fields from Integer variant output
   - ✅ Remove `minimum` and `maximum` fields from Float variant output
   - ✅ Keep simple `{"type": "integer"}` and `{"type": "number"}` outputs

3. ✅ **Update StringType ToJsonSchema implementation** 
   - ✅ Remove `minLength` and `maxLength` from Unknown variant output
   - ✅ Keep all other string type outputs unchanged (formats, enums, etc.)

4. ✅ **Update array ToJsonSchema implementation**
   - ✅ Remove `minItems` and `maxItems` from array schema output
   - ✅ Keep `items` schema and `type: "array"`

5. ✅ **Verify enum inference still works**
   - ✅ Ensure StringType::Enum variants still output enum arrays
   - ✅ Test end-to-end enum inference and output

6. ✅ **Update documentation and examples**
   - ✅ Update any examples that show min/max in JSON schema output
   - ✅ Update comments that reference these constraint outputs

**Results**:
- ✅ All 124 unit tests pass + 6 new Phase 8 tests
- ✅ All 11 doctests pass after updating examples
- ✅ JSON schema output now excludes speculative constraints
- ✅ Enum inference still works correctly
- ✅ Data generation still uses constraints internally for realistic output
- ✅ Backward compatibility maintained for all existing functionality

**Rationale**: 
- Min/max constraints are useful internally for realistic data generation
- They're too speculative for JSON schema output (not reliable type information)
- Enum inference provides more reliable semantic information
- Internal constraint tracking remains for data generation quality

** Phase 9: Wholesale code quality assessment and refactoring plan
- [ ] Look at the user's code preference and assess to what extent the code in this project adheres to it. Then draw up an initial analysis.
- [ ] Based on this initial analysis, come up with a refactoring plan.  There should be good test coverage in the project, so you can safely refactor the code as long as the public method signatures remain the same.
- [ ] Apply the refactoring after the user has given his approval.


**Current Status:** Phase 8 complete - JSON schema output simplified, constraints removed but enum inference preserved. Phase 9 code quality assessment and refactoring plan completed. Currently working on Phase 2 of refactoring plan (Medium Risk Changes).
