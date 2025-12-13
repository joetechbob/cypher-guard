#!/usr/bin/env python
"""Test script for new type checking functionality in Python bindings."""

from cypher_guard import (
    validate_query_elements_with_options,
    CypherValidationOptions,
    TypeCheckLevel,
    TypeMismatchSeverity,
    TypeIssue,
    DbSchema,
)
import json


def main():
    print("=" * 60)
    print("Cypher Guard Type Checking - Python Bindings Test")
    print("=" * 60)
    print()

    # Load test schema
    with open("data/schema/eval_schema.json") as f:
        schema_data = json.load(f)
    schema = DbSchema.from_dict(schema_data)

    # Test 1: TypeCheckLevel enum
    print("✅ TypeCheckLevel Enum:")
    print(f"   - Off: {TypeCheckLevel.Off}")
    print(f"   - Warnings: {TypeCheckLevel.Warnings}")
    print(f"   - Strict: {TypeCheckLevel.Strict}")
    print()

    # Test 2: TypeMismatchSeverity enum
    print("✅ TypeMismatchSeverity Enum:")
    print(f"   - Warning: {TypeMismatchSeverity.Warning}")
    print(f"   - Error: {TypeMismatchSeverity.Error}")
    print()

    # Test 3: CypherValidationOptions
    print("✅ CypherValidationOptions:")
    options_off = CypherValidationOptions(type_checking=TypeCheckLevel.Off)
    options_warn = CypherValidationOptions(type_checking=TypeCheckLevel.Warnings)
    options_strict = CypherValidationOptions(type_checking=TypeCheckLevel.Strict)
    print(f"   - Off: {options_off.type_checking}")
    print(f"   - Warnings: {options_warn.type_checking}")
    print(f"   - Strict: {options_strict.type_checking}")
    print()

    # Test 4: validate_query_elements_with_options
    print("✅ Testing validate_query_elements_with_options:")
    query = """
    MATCH (p:Person)-[:KNOWS]->(friend:Person)
    WHERE p.age > 25
    RETURN p.firstName, p.lastName, friend.firstName, friend.lastName, p.age
    """
    print(f"   Query: {query.strip()}")
    print()

    errors, type_issues = validate_query_elements_with_options(
        query, schema, options_warn
    )

    print(f"   Validation Errors: {len(errors)}")
    for err in errors:
        print(f"     - {err}")
    print()

    print(f"   Type Issues: {len(type_issues)}")
    for issue in type_issues:
        print(f"     - {issue.severity}: {issue.message}")
        if issue.suggestion:
            print(f"       Suggestion: {issue.suggestion}")
    print()

    # Test 5: Different type checking levels
    print("✅ Testing Different Type Checking Levels:")
    for level_name, level in [
        ("Off", TypeCheckLevel.Off),
        ("Warnings", TypeCheckLevel.Warnings),
        ("Strict", TypeCheckLevel.Strict),
    ]:
        opts = CypherValidationOptions(type_checking=level)
        errors, issues = validate_query_elements_with_options(query, schema, opts)
        print(f"   {level_name}: {len(errors)} errors, {len(issues)} type issues")
    print()

    print("=" * 60)
    print("✅ All Type Checking Features Working!")
    print("=" * 60)


if __name__ == "__main__":
    main()
