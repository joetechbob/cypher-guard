# Cypher-Guard Unit Test Coverage Completion Plan

**Created**: 2025-12-13
**Status**: Ready for Implementation
**Current Test Status**: 376 passing, 24 failing (94% pass rate)
**Target**: 500+ tests with 100% pass rate

---

## Executive Summary

This plan addresses two major objectives:
1. **Fix 24 failing tests** - Critical parser bugs blocking core functionality
2. **Implement Priority 1 missing features** - High-usage Cypher features identified in coverage analysis

The plan is organized into phases that can be executed sequentially, with each phase building on the previous one.

---

## Current Test Analysis

### Test Distribution
- `test_comprehensive_queries.rs` - 38 tests covering core patterns
- `validation_typecheck_tests.rs` - Type checking validation tests
- `test_agent_queries.rs` - Real-world agent query tests
- `test_user_query.rs` - User-submitted query tests
- `test_priority1_features.rs` - 19 failing tests for new features
- Unit tests in `clauses.rs` - ~150 tests for individual parsers

### Critical Failures (24 tests)

#### Category 1: WHERE Clause Comparison Operators (3 tests)
**Files**: `rust/cypher_guard/src/parser/clauses.rs`
- `test_where_clause_less_than_equal` - `<=` operator parsing
- `test_where_clause_greater_than_equal` - `>=` operator parsing
- Root cause: Operator precedence or tokenization issue

#### Category 2: Function Calls in WHERE (2 tests)
**Files**: `rust/cypher_guard/src/test_comprehensive_queries.rs`, `rust/cypher_guard/src/test_user_query.rs`
- `test_function_in_where` - `WHERE length(name) > 5`
- `test_exact_user_query_string_date_comparison` - `WHERE ps.valid_from <= date('2025-04-08')`
- Root cause: Function call support incomplete in WHERE expressions
- Error message: "function calls not supported in WHERE!"

#### Category 3: MERGE Clause (1 test)
**Files**: `rust/cypher_guard/src/test_comprehensive_queries.rs`
- `test_merge_with_on_create_on_match` - MERGE with conditional SET clauses
- Root cause: ON CREATE SET / ON MATCH SET parsing incomplete

#### Category 4: Invalid Identifier Test (1 test)
**Files**: `rust/cypher_guard/src/parser/clauses.rs`
- `test_return_item_invalid_identifier` - Should reject invalid identifiers
- Root cause: Parser too permissive

#### Category 5: Priority 1 Features (17 tests)
**Files**: `rust/cypher_guard/src/test_priority1_features.rs`
- Arithmetic operations with parentheses
- List operations (access, slicing, concatenation, comprehension)
- Map operations (bracket access, projection)
- Pattern comprehension
- Dynamic property access
- Root cause: Features not yet implemented

---

## Phase 1: Fix Critical Parser Bugs (Priority: CRITICAL)

**Estimated Time**: 4-6 hours
**Goal**: Get all existing functionality tests passing (376 → 357 passing)

### Task 1.1: Fix WHERE Comparison Operators (2 hours)

**Problem**: `<=` and `>=` operators not parsing correctly

**Location**: `rust/cypher_guard/src/parser/expressions.rs`

**Steps**:
1. Review comparison operator parser
2. Ensure proper operator precedence: `<=` and `>=` before `<` and `>`
3. Add tokenization tests
4. Fix parser to handle multi-character operators

**Tests to Fix**:
- `parser::clauses::tests::test_where_clause_less_than_equal`
- `parser::clauses::tests::test_where_clause_greater_than_equal`

**Verification**:
```bash
cargo test test_where_clause_less_than_equal
cargo test test_where_clause_greater_than_equal
```

### Task 1.2: Enable Function Calls in WHERE (2 hours)

**Problem**: Function calls like `date('2025-04-08')` and `length(name)` fail in WHERE clauses

**Location**: `rust/cypher_guard/src/parser/expressions.rs`

**Current Status**: Functions parse in RETURN but fail in WHERE

**Steps**:
1. Review WHERE expression parser
2. Enable function call parsing in WHERE context
3. Add test cases for common functions in WHERE:
   - `date()`, `datetime()`, `timestamp()`
   - `length()`, `size()`
   - `count()`, `sum()`

**Tests to Fix**:
- `test_comprehensive_queries::test_function_in_where`
- `test_user_query::test_exact_user_query_string_date_comparison`

**Verification**:
```bash
cargo test test_function_in_where
cargo test test_exact_user_query_string_date_comparison
```

### Task 1.3: Fix MERGE ON CREATE/ON MATCH (1-2 hours)

**Problem**: MERGE with ON CREATE SET / ON MATCH SET not fully supported

**Location**: `rust/cypher_guard/src/parser/clauses.rs`

**Steps**:
1. Review MERGE clause parser
2. Enhance to support ON CREATE SET and ON MATCH SET
3. Add AST nodes for conditional SET clauses
4. Update validation logic

**Test to Fix**:
- `test_comprehensive_queries::test_merge_with_on_create_on_match`

**Verification**:
```bash
cargo test test_merge_with_on_create_on_match
```

### Task 1.4: Fix Invalid Identifier Test (30 minutes)

**Problem**: Parser should reject invalid identifiers but doesn't

**Location**: `rust/cypher_guard/src/parser/expressions.rs`

**Steps**:
1. Review identifier parsing rules
2. Add validation for invalid characters
3. Ensure error handling works correctly

**Test to Fix**:
- `parser::clauses::tests::test_return_item_invalid_identifier`

**Verification**:
```bash
cargo test test_return_item_invalid_identifier
```

**Phase 1 Deliverables**:
- ✅ All comparison operators working (`<=`, `>=`, `<`, `>`, `=`, `<>`)
- ✅ Function calls working in WHERE clauses
- ✅ MERGE with ON CREATE/ON MATCH fully supported
- ✅ Proper identifier validation
- ✅ 7 critical tests passing
- ✅ Test count: 383/400 passing (95.75%)

---

## Phase 2: Implement Priority 1 Expression Features (Priority: HIGH)

**Estimated Time**: 8-12 hours
**Goal**: Add most commonly used Cypher expression features

### Task 2.1: String Comparison Operators (2 hours)

**Features**:
- `STARTS WITH` - `WHERE n.name STARTS WITH 'A'`
- `ENDS WITH` - `WHERE n.name ENDS WITH 'son'`
- `CONTAINS` - `WHERE n.name CONTAINS 'oh'`
- `IN` operator - `WHERE n.prop IN [1, 2, 3]`

**Location**: `rust/cypher_guard/src/parser/expressions.rs`

**Implementation**:
1. Add string operator tokens to lexer
2. Create string comparison expression AST nodes
3. Add parser for string operators
4. Integrate with WHERE clause parser

**New Tests Required** (create `test_string_operators.rs`):
```rust
#[test]
fn test_starts_with() {
    let query = "MATCH (n:Person) WHERE n.name STARTS WITH 'J' RETURN n";
    // ...
}

#[test]
fn test_ends_with() {
    let query = "MATCH (n:Person) WHERE n.name ENDS WITH 'son' RETURN n";
    // ...
}

#[test]
fn test_contains() {
    let query = "MATCH (n:Person) WHERE n.name CONTAINS 'oh' RETURN n";
    // ...
}

#[test]
fn test_in_operator_with_list() {
    let query = "MATCH (n:Person) WHERE n.age IN [25, 30, 35] RETURN n";
    // ...
}

#[test]
fn test_in_operator_with_property() {
    let query = "MATCH (n) WHERE 'email' IN keys(n) RETURN n";
    // ...
}
```

**Test Count**: +10 tests

### Task 2.2: Arithmetic Operators (3 hours)

**Features**:
- Addition: `+` for numbers
- Subtraction: `-`
- Multiplication: `*`
- Division: `/`
- Modulo: `%`
- Exponentiation: `^`

**Location**: `rust/cypher_guard/src/parser/expressions.rs`

**Implementation**:
1. Create arithmetic expression AST node
2. Implement operator precedence:
   - Level 1: `^` (exponentiation)
   - Level 2: `*`, `/`, `%`
   - Level 3: `+`, `-`
3. Add parentheses support for precedence override
4. Handle both WHERE and RETURN contexts

**Tests to Fix**:
- `test_priority1_features::test_arithmetic_with_parentheses`

**New Tests Required** (enhance `test_priority1_features.rs`):
```rust
#[test]
fn test_arithmetic_addition() {
    let query = "MATCH (n) WHERE n.age + 5 > 30 RETURN n";
    // ...
}

#[test]
fn test_arithmetic_multiplication() {
    let query = "MATCH (p:Product) RETURN p.price * 1.1 AS newPrice";
    // ...
}

#[test]
fn test_arithmetic_exponentiation() {
    let query = "RETURN 2 ^ 3 AS result";
    // ...
}

#[test]
fn test_arithmetic_complex() {
    let query = "RETURN (2 + 3) * 4 - 5 / 2 AS result";
    // ...
}
```

**Test Count**: +15 tests (1 fixed, 14 new)

### Task 2.3: String Concatenation (1 hour)

**Features**:
- Concatenation with `+`: `n.firstName + ' ' + n.lastName`
- Concatenation with `||`: `n.firstName || ' ' || n.lastName`

**Location**: `rust/cypher_guard/src/parser/expressions.rs`

**Implementation**:
1. Extend arithmetic operator parser to handle string context
2. Add type-aware operator handling (+ can be arithmetic or string)
3. Support `||` as alternative string concatenation operator

**New Tests Required**:
```rust
#[test]
fn test_string_concatenation_plus() {
    let query = "RETURN n.firstName + ' ' + n.lastName AS fullName";
    // ...
}

#[test]
fn test_string_concatenation_pipes() {
    let query = "RETURN n.firstName || ' ' || n.lastName AS fullName";
    // ...
}
```

**Test Count**: +8 tests

### Task 2.4: List Operations (3-4 hours)

**Features**:
- Element access: `list[0]`, `list[-1]`
- Slicing: `list[1..3]`, `list[1..]`, `list[..3]`
- Concatenation: `list1 + list2`, `list1 || list2`
- List comprehension: `[x IN list WHERE x > 5 | x * 2]`

**Location**: `rust/cypher_guard/src/parser/expressions.rs`

**Implementation**:
1. Add list index expression AST node
2. Add list slice expression AST node
3. Implement list comprehension parser
4. Support negative indexing

**Tests to Fix** (7 tests):
- `test_priority1_features::test_list_element_access`
- `test_priority1_features::test_list_element_negative_index`
- `test_priority1_features::test_list_slicing`
- `test_priority1_features::test_list_slicing_from_start`
- `test_priority1_features::test_list_slicing_to_end`
- `test_priority1_features::test_list_concatenation_plus`
- `test_priority1_features::test_list_concatenation_pipes`
- `test_priority1_features::test_list_comprehension_simple`
- `test_priority1_features::test_list_comprehension_with_transform`
- `test_priority1_features::test_nested_list_operations`
- `test_priority1_features::test_list_operations_query`

**Test Count**: +11 tests (11 fixed)

### Task 2.5: Map Operations (2-3 hours)

**Features**:
- Bracket access: `map['key']`
- Projection: `n{.name, .age, computed: n.prop * 2}`
- Dynamic property access: `n[variableKey]`

**Location**: `rust/cypher_guard/src/parser/expressions.rs`

**Implementation**:
1. Add map access expression AST node
2. Add map projection expression AST node
3. Support both static and dynamic property access
4. Handle nested map operations

**Tests to Fix** (3 tests):
- `test_priority1_features::test_map_property_access_brackets`
- `test_priority1_features::test_map_projection`
- `test_priority1_features::test_dynamic_property_access`

**Test Count**: +8 tests (3 fixed, 5 new)

### Task 2.6: Pattern Comprehension (2 hours)

**Features**:
- Pattern comprehension: `[(n)-->(m) WHERE m.age > 25 | m.name]`
- Complex patterns in list context

**Location**: `rust/cypher_guard/src/parser/expressions.rs`

**Implementation**:
1. Add pattern comprehension AST node
2. Parse pattern inside list brackets
3. Support WHERE filtering
4. Support projection expressions

**Tests to Fix** (2 tests):
- `test_priority1_features::test_pattern_comprehension`
- `test_priority1_features::test_recommendation_engine_query`

**Test Count**: +5 tests (2 fixed, 3 new)

**Phase 2 Deliverables**:
- ✅ All string comparison operators working
- ✅ All arithmetic operators with proper precedence
- ✅ String concatenation support
- ✅ Complete list operation support
- ✅ Complete map operation support
- ✅ Pattern comprehension support
- ✅ 17 Priority 1 feature tests passing
- ✅ ~57 new tests added
- ✅ Test count: 440+ passing (expected)

---

## Phase 3: Implement Type Checking System (Priority: MEDIUM)

**Estimated Time**: 5-7 hours
**Goal**: Add opt-in type checking as specified in IMPLEMENTATION_ROADMAP.md

### Task 3.1: Create Type System Module (1 hour)

**File**: `rust/cypher_guard/src/types.rs` (NEW)

**Implementation**: Follow IMPLEMENTATION_ROADMAP.md Step 1
- `TypeCheckLevel` enum (off, warnings, strict)
- `Neo4jType` enum for property types
- `TypeMismatchSeverity` enum
- `TypeIssue` struct
- `parse_neo4j_type()` function
- `check_type_compatibility()` function (blocklist approach)

**Tests**: Include inline unit tests in `types.rs`
- Test string vs date mismatch detection
- Test integer vs float allowance
- Test unknown type handling

**Test Count**: +8 tests

### Task 3.2: Extend Validation Module (2 hours)

**File**: `rust/cypher_guard/src/validation.rs` (MODIFY)

**Implementation**: Follow IMPLEMENTATION_ROADMAP.md Step 2
- Add `ValidationOptions` struct
- Modify `validate_query_elements()` signature
- Add `check_property_comparison_types()` function
- Return both errors and type issues

**Test Count**: Existing validation tests should still pass

### Task 3.3: Update Python Bindings (1.5 hours)

**File**: `rust/python_bindings/src/lib.rs` (MODIFY)

**Implementation**: Follow IMPLEMENTATION_ROADMAP.md Step 3
- Add `type_checking` parameter to `validate_cypher()`
- Parse type checking level from string
- Return type warnings and errors separately
- Handle strict mode validation

**Test Count**: Python binding tests update automatically

### Task 3.4: Create Type Checking Test Suite (2.5 hours)

**File**: `rust/python_bindings/tests/unit/test_type_checking.py` (NEW)

**Implementation**: Follow IMPLEMENTATION_ROADMAP.md Step 4
- Test default off mode (backward compatibility)
- Test warnings mode (reports but doesn't block)
- Test strict mode (blocks on type errors)
- Test allowed type combinations
- Test suggestions for common mismatches

**Test Count**: +15 Python tests

### Task 3.5: Integration Testing (1 hour)

**Steps**:
1. Rebuild with `maturin develop --release`
2. Run all Python tests
3. Run all Rust tests
4. Verify no regressions
5. Test with real-world queries

**Test Count**: All existing tests must pass

**Phase 3 Deliverables**:
- ✅ Type checking system implemented
- ✅ Opt-in API (backward compatible)
- ✅ Blocklist-based type compatibility
- ✅ Python bindings updated
- ✅ +23 new tests added
- ✅ Test count: 463+ passing (expected)

---

## Phase 4: Enhance Test Coverage for Existing Features (Priority: MEDIUM)

**Estimated Time**: 4-6 hours
**Goal**: Add edge case tests for already-implemented features

### Task 4.1: Edge Case Tests for WHERE Clauses (1.5 hours)

**File**: `rust/cypher_guard/src/test_where_edge_cases.rs` (NEW)

**Tests**:
- Deeply nested AND/OR conditions
- Mixed parentheses
- NULL handling in various contexts
- Multiple NOT operators
- Complex function nesting

**Test Count**: +15 tests

### Task 4.2: Pattern Matching Edge Cases (1.5 hours)

**File**: `rust/cypher_guard/src/test_pattern_edge_cases.rs` (NEW)

**Tests**:
- Variable-length path edge cases (`*0..`, `*..0`)
- Multiple relationship types in same pattern
- Circular patterns
- Self-referencing relationships
- Mixed optional and required patterns

**Test Count**: +12 tests

### Task 4.3: MERGE and SET Edge Cases (1 hour)

**File**: Enhance `test_comprehensive_queries.rs`

**Tests**:
- Multiple properties in SET
- SET with function results
- MERGE with complex patterns
- Nested MERGE operations

**Test Count**: +8 tests

### Task 4.4: RETURN Clause Variations (1 hour)

**File**: Enhance `test_comprehensive_queries.rs`

**Tests**:
- RETURN with complex expressions
- Multiple RETURN aliases
- RETURN with calculations
- LIMIT/SKIP edge cases (0, negative, etc.)

**Test Count**: +10 tests

**Phase 4 Deliverables**:
- ✅ Comprehensive edge case coverage
- ✅ +45 new tests added
- ✅ Test count: 508+ passing (expected)

---

## Phase 5: Additional Priority Features (Priority: LOW)

**Estimated Time**: 6-10 hours
**Goal**: Implement remaining high-value features from coverage analysis

### Task 5.1: Aggregation Functions (2 hours)

**Features**:
- `sum()`, `avg()`, `min()`, `max()`
- `collect()`
- Proper aggregation validation

**Test Count**: +12 tests

### Task 5.2: ORDER BY Clause (2 hours)

**Features**:
- `ORDER BY` with ASC/DESC
- Multiple sort keys
- ORDER BY with expressions

**Test Count**: +10 tests

### Task 5.3: DISTINCT Support (1.5 hours)

**Features**:
- `RETURN DISTINCT`
- `WITH DISTINCT`

**Test Count**: +8 tests

### Task 5.4: DELETE and REMOVE Clauses (2.5 hours)

**Features**:
- `DELETE` nodes/relationships
- `DETACH DELETE`
- `REMOVE` properties and labels

**Test Count**: +12 tests

**Phase 5 Deliverables**:
- ✅ Most common remaining features implemented
- ✅ +42 new tests added
- ✅ Test count: 550+ passing (expected)

---

## Testing Strategy

### Test Organization

**Existing Test Files**:
- `parser/clauses.rs` - Unit tests for individual parsers (~150 tests)
- `test_comprehensive_queries.rs` - Integration tests (38 tests)
- `validation_typecheck_tests.rs` - Type validation tests
- `test_agent_queries.rs` - Real-world usage tests
- `test_user_query.rs` - User-submitted queries
- `test_priority1_features.rs` - Priority 1 feature tests (19 tests)

**New Test Files to Create**:
- `test_string_operators.rs` - String comparison operators
- `test_where_edge_cases.rs` - WHERE clause edge cases
- `test_pattern_edge_cases.rs` - Pattern matching edge cases
- `types.rs` - Type system unit tests
- `test_type_checking.py` - Python type checking tests

### Test Naming Convention

```rust
// Unit tests (in parser modules)
#[test]
fn test_<clause>_<feature>_<scenario>() { }

// Integration tests
#[test]
fn test_<feature>_<use_case>() { }

// Edge case tests
#[test]
fn test_<feature>_edge_<specific_edge_case>() { }
```

### Running Tests

```bash
# Run all Rust tests
cargo test --lib

# Run specific test file
cargo test --lib test_comprehensive_queries

# Run specific test
cargo test --lib test_where_clause_less_than_equal

# Run Python tests
cd rust/python_bindings
pytest tests/ -v

# Run with coverage
cargo tarpaulin --out Html --output-dir coverage
```

---

## Success Metrics

### Phase 1 Success Criteria
- ✅ 0 failing tests for existing functionality
- ✅ All comparison operators working
- ✅ Function calls in WHERE working
- ✅ MERGE fully functional
- ✅ Test count: 383/400 (95.75% pass rate)

### Phase 2 Success Criteria
- ✅ All Priority 1 expression features implemented
- ✅ 17 Priority 1 feature tests passing
- ✅ String operators fully functional
- ✅ Arithmetic expressions working
- ✅ List operations complete
- ✅ Map operations complete
- ✅ Test count: 440+ passing

### Phase 3 Success Criteria
- ✅ Type checking system operational
- ✅ Backward compatible (off by default)
- ✅ All three modes working (off, warnings, strict)
- ✅ Python integration complete
- ✅ Test count: 463+ passing

### Phase 4 Success Criteria
- ✅ Comprehensive edge case coverage
- ✅ No known parser bugs
- ✅ Test count: 508+ passing

### Phase 5 Success Criteria
- ✅ Most common Cypher features supported
- ✅ >95% of real-world queries parseable
- ✅ Test count: 550+ passing

### Final Success Criteria
- ✅ **500+ tests passing** (Target achieved!)
- ✅ **100% pass rate** (No failing tests)
- ✅ **Parse speed <0.05s** for full test suite
- ✅ **Agent query success >95%** for real-world patterns
- ✅ All Priority 1 features from CYPHER_COVERAGE_ANALYSIS.md implemented

---

## Timeline Summary

| Phase | Focus | Time | Tests Added/Fixed | Cumulative |
|-------|-------|------|-------------------|------------|
| 1 | Fix Critical Bugs | 4-6h | +7 | 383 |
| 2 | Priority 1 Features | 8-12h | +57 | 440+ |
| 3 | Type Checking | 5-7h | +23 | 463+ |
| 4 | Edge Cases | 4-6h | +45 | 508+ |
| 5 | Additional Features | 6-10h | +42 | 550+ |
| **Total** | | **27-41h** | **+174** | **550+** |

**Recommended Approach**:
- Execute Phase 1 immediately (critical bugs)
- Execute Phase 2 for high-value features
- Execute Phase 3 for type safety
- Phases 4-5 can be done incrementally

---

## Risk Mitigation

### Risk 1: Breaking Changes
**Mitigation**: Run full test suite after each task. Commit frequently.

### Risk 2: Parser Performance Degradation
**Mitigation**: Monitor test execution time. Target: <0.05s for 550 tests.

### Risk 3: Backward Compatibility
**Mitigation**: Type checking is opt-in. All new features should be backward compatible.

### Risk 4: Incomplete AST Representation
**Mitigation**: Design AST nodes carefully. Consider future extensibility.

---

## Appendix: Quick Reference

### Current Failing Tests (24)
1. `test_where_clause_less_than_equal` - WHERE with `<=`
2. `test_where_clause_greater_than_equal` - WHERE with `>=`
3. `test_function_in_where` - Function calls in WHERE
4. `test_exact_user_query_string_date_comparison` - date() in WHERE
5. `test_merge_with_on_create_on_match` - MERGE conditional SET
6. `test_return_item_invalid_identifier` - Identifier validation
7-11. List operation tests (5 tests)
12-14. List comprehension tests (3 tests)
15-17. Map operation tests (3 tests)
18-19. Pattern comprehension tests (2 tests)
20-24. Arithmetic tests (5 tests)

### Test Commands Cheatsheet
```bash
# Quick test run
cargo test --lib --quiet

# Verbose test run with output
cargo test --lib -- --nocapture

# Run single test
cargo test test_name

# Run tests matching pattern
cargo test test_where

# Python tests
pytest rust/python_bindings/tests/ -v

# Full coverage report
cargo tarpaulin --out Html
```

---

**Next Steps**: Begin with Phase 1, Task 1.1 - Fix WHERE comparison operators.
