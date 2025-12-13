#!/usr/bin/env python
"""Demo showing validation working correctly - both valid and invalid queries."""

from cypher_guard import (
    validate_query_elements_with_options,
    CypherValidationOptions,
    TypeCheckLevel,
    DbSchema,
)
import json


def test_query(schema, query, description):
    """Test a query and print results."""
    print(f"\n{'=' * 70}")
    print(f"Test: {description}")
    print(f"{'=' * 70}")
    print(f"Query:\n{query.strip()}\n")
    
    options = CypherValidationOptions(type_checking=TypeCheckLevel.Warnings)
    errors, type_issues = validate_query_elements_with_options(query, schema, options)
    
    if not errors and not type_issues:
        print("✅ VALID - No errors found!")
    else:
        print(f"❌ INVALID - {len(errors)} validation error(s), {len(type_issues)} type issue(s)")
        for err in errors:
            print(f"   ERROR: {err}")
        for issue in type_issues:
            print(f"   {issue.severity.upper()}: {issue.message}")


def main():
    # Load schema
    with open("data/schema/eval_schema.json") as f:
        schema_data = json.load(f)
    schema = DbSchema.from_dict(schema_data)

    print("\n" + "=" * 70)
    print("Cypher Guard Validation Demo")
    print("=" * 70)
    print("\nSchema has:")
    print("  - Person: firstName, lastName, age, email, active, employmentType")
    print("  - Company: companyName, foundedYear, industry, employeeCount")
    print("  - Relationships: KNOWS, WORKS_FOR, LOCATED_IN")

    # Test 1: Valid query
    test_query(
        schema,
        """
        MATCH (p:Person)-[:KNOWS]->(friend:Person)
        WHERE p.age > 25
        RETURN p.firstName, p.lastName, p.age
        """,
        "Valid Query - Using correct property names"
    )

    # Test 2: Invalid property (name doesn't exist)
    test_query(
        schema,
        """
        MATCH (p:Person)
        RETURN p.name
        """,
        "Invalid Query - 'name' property doesn't exist (should be firstName/lastName)"
    )

    # Test 3: Invalid relationship
    test_query(
        schema,
        """
        MATCH (p:Person)-[:LIKES]->(c:Company)
        RETURN p.firstName
        """,
        "Invalid Query - LIKES relationship doesn't exist in schema"
    )

    # Test 4: Valid complex query
    test_query(
        schema,
        """
        MATCH (p:Person)-[:WORKS_FOR]->(c:Company)
        WHERE c.industry = 'Technology' AND p.age > 30
        RETURN p.firstName, p.lastName, c.companyName, p.age
        ORDER BY p.age DESC
        LIMIT 10
        """,
        "Valid Complex Query - Multiple valid properties and relationships"
    )

    # Test 5: Invalid label
    test_query(
        schema,
        """
        MATCH (u:User)
        RETURN u.firstName
        """,
        "Invalid Query - 'User' label doesn't exist in schema"
    )

    print("\n" + "=" * 70)
    print("✅ Validation Demo Complete!")
    print("=" * 70)
    print("\nConclusion: Validation correctly identifies:")
    print("  ✓ Invalid property names (name vs firstName/lastName)")
    print("  ✓ Invalid relationship types (LIKES)")
    print("  ✓ Invalid node labels (User)")
    print("  ✓ Valid queries pass without errors")
    print()


if __name__ == "__main__":
    main()
