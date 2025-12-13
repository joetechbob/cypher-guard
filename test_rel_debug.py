#!/usr/bin/env python
"""Debug script to test relationship property type checking."""

from cypher_guard import (
    validate_query_elements_with_options,
    CypherValidationOptions,
    TypeCheckLevel,
    DbSchema,
)
import json

# Load schema
with open("data/schema/eval_schema.json") as f:
    schema_data = json.load(f)
schema = DbSchema.from_dict(schema_data)

# Test query
query = "MATCH (p:Person)-[r:WORKS_FOR]->(c:Company) WHERE r.salary = 'high' RETURN p, c"

print("Testing relationship property type checking...")
print(f"Query: {query}\n")

# Test with type checking WARNINGS
options = CypherValidationOptions(type_checking=TypeCheckLevel.Warnings)
errors, issues = validate_query_elements_with_options(query, schema, options)

print(f"Errors: {len(errors)}")
for err in errors:
    print(f"  - {err}")

print(f"\nType Issues: {len(issues)}")
for issue in issues:
    print(f"  - {issue.severity}: {issue.message}")

if len(issues) == 0:
    print("\n❌ FAIL: Expected 1 type issue but got 0")
    print("Relationship property type checking may not be working yet.")
else:
    print("\n✅ SUCCESS: Relationship property type checking is working!")
