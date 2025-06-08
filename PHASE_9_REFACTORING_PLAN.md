# Phase 9: Detailed Refactoring Plan

## Overview
This document outlines specific refactoring transformations to improve code quality and adherence to functional programming principles while maintaining all existing functionality and test coverage.

## Refactoring Strategy

### Principles
1. **One change per commit** - Small, focused transformations
2. **Preserve all public APIs** - No breaking changes
3. **Maintain test coverage** - All 130 tests must continue passing
4. **Functional over imperative** - Where it improves clarity
5. **Simple solutions first** - Avoid clever optimizations

### Execution Order
Refactor in dependency order (leaf modules first, main module last) to minimize cascading changes.

## Detailed Refactoring Plan

### 1. Replace Custom Utility Functions (Low Risk)
**File**: `src/infer.rs`
**Issue**: Custom `min()` and `max()` functions reinvent standard library
**Transformation**: Replace with `std::cmp::{min, max}`

**Before**:
```rust
fn min<T: PartialOrd>(left: T, right: T) -> T {
    if left < right { left } else { right }
}
```

**After**:
```rust
use std::cmp::{min, max};
// Remove custom functions, use std::cmp directly
```

**Risk**: Very low - simple find/replace
**Tests**: All existing tests should pass unchanged

### 2. Functional String Formatting (Medium Risk)
**File**: `src/schema.rs`
**Issue**: `to_string_pretty_inner()` uses imperative mutable state
**Transformation**: Use functional string building with iterators

**Before**:
```rust
fn to_string_pretty_inner(schema: &SchemaState, indent: usize) -> String {
    let mut result = String::new();
    // ... imperative building
    result
}
```

**After**:
```rust
fn to_string_pretty_inner(schema: &SchemaState, indent: usize) -> String {
    match schema {
        SchemaState::Object { required, optional } => {
            let indent_str = "  ".repeat(indent);
            let fields = required.iter().chain(optional.iter())
                .map(|(k, v)| format!("{}{}: {}", indent_str, k, to_string_pretty_inner(v, indent + 1)))
                .collect::<Vec<_>>()
                .join("\n");
            format!("object {{\n{}\n{}}}", fields, "  ".repeat(indent.saturating_sub(1)))
        }
        // ... other cases
    }
}
```

**Risk**: Medium - changes output format logic
**Tests**: String formatting tests need careful validation

### 3. Simplify Main Function (High Risk)
**File**: `src/main.rs`
**Issue**: Large imperative function with mixed concerns
**Transformation**: Extract pure functions for each concern

**Before**:
```rust
fn main() {
    let args = Args::parse();
    let input = match std::io::read_to_string(std::io::stdin()) {
        // ... large imperative block
    };
    // ... more imperative logic
}
```

**After**:
```rust
fn main() {
    let args = Args::parse();
    let result = run_drivel(&args)
        .map_err(|e| {
            eprintln!("{}", e);
            std::process::exit(1);
        });
}

fn run_drivel(args: &Args) -> Result<(), DrivelError> {
    let input = read_input()?;
    let schema = parse_schema(&input, args)?;
    write_output(&schema, args)
}

fn read_input() -> Result<String, DrivelError> { /* ... */ }
fn parse_schema(input: &str, args: &Args) -> Result<SchemaState, DrivelError> { /* ... */ }
fn write_output(schema: &SchemaState, args: &Args) -> Result<(), DrivelError> { /* ... */ }
```

**Risk**: High - changes CLI behavior structure
**Tests**: Manual CLI testing required

### 4. Functional Merge Logic (High Risk)
**File**: `src/infer.rs`
**Issue**: Large `merge()` function with deeply nested patterns
**Transformation**: Extract pure merge functions for each type

**Before**:
```rust
fn merge(initial: SchemaState, new: SchemaState) -> SchemaState {
    match (initial, new) {
        // ... 100+ lines of nested patterns
    }
}
```

**After**:
```rust
fn merge(initial: SchemaState, new: SchemaState) -> SchemaState {
    use SchemaState::*;
    match (initial, new) {
        (Initial, s) | (s, Initial) | (Indefinite, s) | (s, Indefinite) => s,
        (String(a), String(b)) => String(merge_string_types(a, b)),
        (Number(a), Number(b)) => Number(merge_number_types(a, b)),
        (Array { .. }, Array { .. }) => merge_array_schemas(initial, new),
        // ... simplified patterns
    }
}

fn merge_string_types(a: StringType, b: StringType) -> StringType { /* ... */ }
fn merge_number_types(a: NumberType, b: NumberType) -> NumberType { /* ... */ }
fn merge_array_schemas(a: SchemaState, b: SchemaState) -> SchemaState { /* ... */ }
```

**Risk**: High - core inference logic
**Tests**: All inference tests must pass

### 5. Functional Validation Patterns (Medium Risk)
**File**: `src/parse_schema.rs`
**Issue**: Repetitive validation patterns
**Transformation**: Create functional validation combinators

**Before**:
```rust
fn parse_string_type(schema_obj: &Map<String, Value>) -> Result<SchemaState, ParseSchemaError> {
    let format = schema_obj.get("format");
    let min_length = schema_obj.get("minLength");
    // ... repetitive validation
}
```

**After**:
```rust
fn parse_string_type(schema_obj: &Map<String, Value>) -> Result<SchemaState, ParseSchemaError> {
    let format = get_string_field(schema_obj, "format")?;
    let min_length = get_usize_field(schema_obj, "minLength")?;
    // ... using validation combinators
}

fn get_string_field(obj: &Map<String, Value>, key: &str) -> Result<Option<String>, ParseSchemaError> { /* ... */ }
fn get_usize_field(obj: &Map<String, Value>, key: &str) -> Result<Option<usize>, ParseSchemaError> { /* ... */ }
```

**Risk**: Medium - changes validation logic
**Tests**: All parsing tests must pass

### 6. Functional Data Generation (Low Risk)
**File**: `src/produce.rs`
**Issue**: Some imperative string building
**Transformation**: Use functional string generation

**Before**:
```rust
StringType::Unknown { chars_seen, .. } => {
    let mut s = String::with_capacity(take_n);
    for _ in 0..take_n {
        let idx = thread_rng().gen_range(0..chars_seen.len());
        s.push(chars_seen[idx]);
    }
    s
}
```

**After**:
```rust
StringType::Unknown { chars_seen, .. } => {
    (0..take_n)
        .map(|_| {
            let idx = thread_rng().gen_range(0..chars_seen.len());
            chars_seen[idx]
        })
        .collect()
}
```

**Risk**: Low - simple transformation
**Tests**: Data generation tests should pass unchanged

## Implementation Steps

### Phase 1: Low Risk Changes (1-2 commits) ✅ COMPLETED
1. ✅ Replace custom `min/max` functions with std library - DONE (commit 387e857)
2. ✅ Functional data generation improvements - DONE (commit 9424b10)
3. ✅ Minor test organization improvements - DONE (commit a242918)

### Phase 2: Medium Risk Changes (2-3 commits)
1. Functional string formatting in `schema.rs`
2. Validation pattern improvements in `parse_schema.rs`
3. Extract common constraint parsing (only where truly shared)

### Phase 3: High Risk Changes (3-4 commits)
1. Refactor `merge()` function with type-specific extractors
2. Simplify `main.rs` with pure function extraction
3. Clean up any remaining imperative patterns

### Phase 4: Validation and Polish (1 commit)
1. Run full test suite
2. Manual CLI testing
3. Performance validation
4. Documentation updates if needed

## Testing Strategy

### Automated Testing
- Run `cargo test` after each commit
- All 130 existing tests must pass
- No new test failures acceptable

### Manual Testing
- Test CLI functionality after `main.rs` changes
- Verify output format after string formatting changes
- Test edge cases for inference logic changes

### Rollback Plan
- Each commit is atomic and revertible
- If any test fails, revert the specific commit
- Re-evaluate approach for failed transformations

## Success Criteria

### Functional Code Metrics
- Reduce imperative patterns by ~60%
- Maintain or improve readability
- Preserve all existing functionality
- No performance degradation

### Quality Metrics
- All 130 tests pass
- No new compiler warnings
- Maintain API compatibility
- Improve code organization

## Risk Mitigation

1. **Small commits** - Easy to revert individual changes
2. **Test-driven** - Validate after each change
3. **Incremental** - Don't change multiple modules simultaneously
4. **Conservative** - When in doubt, prefer simpler transformation

## Estimated Timeline

- **Phase 1**: 1-2 hours (low risk, simple changes)
- **Phase 2**: 3-4 hours (medium risk, more complex)
- **Phase 3**: 4-6 hours (high risk, core logic)
- **Phase 4**: 1-2 hours (validation and polish)
- **Total**: 9-14 hours of focused work

## Questions for User Approval

1. **Approach**: Does this incremental, test-driven approach align with your preferences?
2. **Priority**: Should we focus on the highest-impact changes first, or lowest-risk?
3. **Scope**: Are there any specific areas you'd like to exclude from refactoring?
4. **Timeline**: Any constraints on when this should be completed?

Once approved, we'll begin with Phase 1 (low risk changes) and proceed incrementally.