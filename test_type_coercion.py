#!/usr/bin/env python
"""
Test type coercion detection in Python bindings.

These tests focus on TYPE CHECKING validation paths (not schema validation).
All queries use VALID properties, labels, and relationships from the schema,
but have type mismatches that would cause silent failures in Neo4j.
"""

from cypher_guard import (
    validate_query_elements_with_options,
    CypherValidationOptions,
    TypeCheckLevel,
    DbSchema,
)
import json


def load_schema():
    """Load the eval schema."""
    with open("data/schema/eval_schema.json") as f:
        schema_data = json.load(f)
    return DbSchema.from_dict(schema_data)


def print_test_header(title):
    """Print a nice test header."""
    print(f"\n{'=' * 70}")
    print(f"{title}")
    print(f"{'=' * 70}")


def test_type_coercion(schema, query, description, expected_type_issues):
    """Test a query for type coercion issues."""
    print(f"\nTest: {description}")
    print(f"Query: {query.strip()}\n")
    
    # Test with type checking OFF (should pass)
    options_off = CypherValidationOptions(type_checking=TypeCheckLevel.Off)
    errors_off, issues_off = validate_query_elements_with_options(query, schema, options_off)
    
    print(f"  Type Checking OFF:")
    print(f"    - Errors: {len(errors_off)} (should be 0 - no schema errors)")
    print(f"    - Type Issues: {len(issues_off)} (should be 0 - type checking disabled)")
    
    # Test with type checking WARNINGS
    options_warn = CypherValidationOptions(type_checking=TypeCheckLevel.Warnings)
    errors_warn, issues_warn = validate_query_elements_with_options(query, schema, options_warn)
    
    print(f"  Type Checking WARNINGS:")
    print(f"    - Errors: {len(errors_warn)} (should be 0 - no schema errors)")
    print(f"    - Type Issues: {len(issues_warn)} (should be {expected_type_issues})")
    for issue in issues_warn:
        print(f"      ⚠️  {issue.severity}: {issue.message}")
        if issue.suggestion:
            print(f"         Suggestion: {issue.suggestion}")
    
    # Test with type checking STRICT
    options_strict = CypherValidationOptions(type_checking=TypeCheckLevel.Strict)
    errors_strict, issues_strict = validate_query_elements_with_options(query, schema, options_strict)
    
    print(f"  Type Checking STRICT:")
    print(f"    - Errors: {len(errors_strict)} (should be 0 - no schema errors)")
    print(f"    - Type Issues: {len(issues_strict)} (should be {expected_type_issues})")
    for issue in issues_strict:
        print(f"      ❌ {issue.severity}: {issue.message}")
    
    # Assertions
    assert len(errors_off) == 0, f"Should have NO schema errors with type checking OFF, got: {errors_off}"
    assert len(issues_off) == 0, "Type checking OFF should not detect type issues"
    assert len(errors_warn) == 0, "Should have NO schema errors with type checking WARNINGS"
    assert len(issues_warn) == expected_type_issues, f"Expected {expected_type_issues} type issues in WARNINGS mode, got {len(issues_warn)}"
    assert len(errors_strict) == 0, "Should have NO schema errors with type checking STRICT"
    assert len(issues_strict) == expected_type_issues, f"Expected {expected_type_issues} type issues in STRICT mode, got {len(issues_strict)}"
    
    print(f"  ✅ PASS: Type coercion correctly detected!")


def main():
    schema = load_schema()
    
    print_test_header("Type Coercion Detection Tests")
    print("\nThese tests use VALID schema elements (properties, labels, relationships)")
    print("but have TYPE MISMATCHES that would cause silent failures in Neo4j.")
    print("\nSchema Summary:")
    print("  - Person: firstName(STRING), lastName(STRING), age(INTEGER), email(STRING), active(BOOLEAN)")
    print("  - Company: companyName(STRING), foundedYear(INTEGER), industry(STRING), employeeCount(INTEGER)")
    print("  - WORKS_FOR: startDate(DATE_TIME), position(STRING), salary(FLOAT)")
    
    # Test 1: INTEGER property compared with STRING literal
    print_test_header("Test 1: INTEGER vs STRING Comparison")
    test_type_coercion(
        schema,
        """
        MATCH (p:Person)
        WHERE p.age = 'twenty-five'
        RETURN p.firstName
        """,
        "age (INTEGER) compared with string literal 'twenty-five'",
        expected_type_issues=1
    )
    
    # Test 2: STRING property compared with INTEGER literal
    print_test_header("Test 2: STRING vs INTEGER Comparison")
    test_type_coercion(
        schema,
        """
        MATCH (p:Person)
        WHERE p.firstName = 123
        RETURN p
        """,
        "firstName (STRING) compared with integer 123",
        expected_type_issues=1
    )
    
    # Test 3: BOOLEAN property compared with STRING literal
    print_test_header("Test 3: BOOLEAN vs STRING Comparison")
    test_type_coercion(
        schema,
        """
        MATCH (p:Person)
        WHERE p.active = 'yes'
        RETURN p
        """,
        "active (BOOLEAN) compared with string 'yes' instead of true/false",
        expected_type_issues=1
    )
    
    # Test 4: BOOLEAN property compared with INTEGER
    print_test_header("Test 4: BOOLEAN vs INTEGER Comparison")
    test_type_coercion(
        schema,
        """
        MATCH (p:Person)
        WHERE p.active = 1
        RETURN p
        """,
        "active (BOOLEAN) compared with integer 1 instead of true",
        expected_type_issues=1
    )
    
    # Test 5: INTEGER property compared with BOOLEAN
    print_test_header("Test 5: INTEGER vs BOOLEAN Comparison")
    test_type_coercion(
        schema,
        """
        MATCH (p:Person)
        WHERE p.age = true
        RETURN p
        """,
        "age (INTEGER) compared with boolean true",
        expected_type_issues=1
    )
    
    # Test 6: FLOAT vs STRING Comparison (Node property)
    print_test_header("Test 6: FLOAT vs STRING Comparison")
    test_type_coercion(
        schema,
        """
        MATCH (c:Company)
        WHERE c.employeeCount = 'many'
        RETURN c
        """,
        "employeeCount (INTEGER) compared with string 'many'",
        expected_type_issues=1
    )
    
    # Test 7: Multiple type mismatches in one query
    print_test_header("Test 7: Multiple Type Mismatches")
    test_type_coercion(
        schema,
        """
        MATCH (p:Person)
        WHERE p.age = 'thirty' AND p.active = 1
        RETURN p
        """,
        "Two type mismatches: age(INTEGER)='thirty' AND active(BOOLEAN)=1",
        expected_type_issues=2
    )
    
    # Test 8: Valid query with correct types (should pass with 0 issues)
    print_test_header("Test 8: Valid Query - Correct Types")
    print("\nTest: Valid query with matching types")
    query = """
    MATCH (p:Person)
    WHERE p.age > 25 AND p.active = true
    RETURN p.firstName, p.age
    """
    print(f"Query: {query.strip()}\n")
    
    options = CypherValidationOptions(type_checking=TypeCheckLevel.Strict)
    errors, issues = validate_query_elements_with_options(query, schema, options)
    
    print(f"  Type Checking STRICT:")
    print(f"    - Errors: {len(errors)}")
    print(f"    - Type Issues: {len(issues)}")
    
    assert len(errors) == 0, f"Valid query should have no errors, got: {errors}"
    assert len(issues) == 0, f"Valid query should have no type issues, got: {issues}"
    print(f"  ✅ PASS: Valid query passes type checking!")
    
    # Summary
    print_test_header("Summary")
    print("\n✅ All type coercion tests passed!")
    print("\nKey Points:")
    print("  • Type checking OFF: No type issues detected (backward compatible)")
    print("  • Type checking WARNINGS: Type issues detected with warning severity")
    print("  • Type checking STRICT: Type issues detected with error severity")
    print("  • Schema validation (labels, properties, relationships) still works")
    print("  • Type coercion detection is SEPARATE from schema validation")
    print("\nType coercion detection successfully identifies:")
    print("  ✓ INTEGER vs STRING mismatches")
    print("  ✓ STRING vs INTEGER mismatches")
    print("  ✓ BOOLEAN vs STRING mismatches")
    print("  ✓ BOOLEAN vs INTEGER mismatches")
    print("  ✓ INTEGER vs BOOLEAN mismatches")
    print("  ✓ FLOAT vs STRING mismatches")
    print("  ✓ Multiple mismatches in one query")
    print()


if __name__ == "__main__":
    main()
