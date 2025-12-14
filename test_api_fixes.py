#!/usr/bin/env python3
"""
Unit tests to verify all API fixes in knowledge_api_tools.py

Tests:
1. validate_cypher returns tuple (errors, type_issues) - Bug #1 fix
2. Type checking works correctly
3. Exception handling works without NameError - Bug #2 fix
"""

import json
import sys
import unittest
from pathlib import Path

# Add parent directory to path
sys.path.insert(0, str(Path(__file__).parent))

from cypher_guard import (
    validate_cypher,
    DbSchema,
    TypeCheckLevel,
    parse_query,
)


class TestValidateCypherAPI(unittest.TestCase):
    """Test the validate_cypher API works correctly after tuple unpacking fix"""

    def setUp(self):
        """Create a test schema"""
        self.schema_dict = {
            "node_props": {
                "Person": [
                    {"name": "firstName", "neo4j_type": "STRING"},
                    {"name": "lastName", "neo4j_type": "STRING"},
                    {"name": "age", "neo4j_type": "INTEGER"},
                    {"name": "active", "neo4j_type": "BOOLEAN"},
                ],
                "Company": [
                    {"name": "name", "neo4j_type": "STRING"},
                    {"name": "employeeCount", "neo4j_type": "INTEGER"},
                ],
            },
            "rel_props": {
                "WORKS_FOR": [
                    {"name": "salary", "neo4j_type": "FLOAT"},
                    {"name": "since", "neo4j_type": "INTEGER"},
                ]
            },
            "relationships": [
                {"start": "Person", "end": "Company", "rel_type": "WORKS_FOR"}
            ],
            "metadata": {"constraint": [], "index": []},
        }
        self.schema = DbSchema.from_dict(self.schema_dict)

    def test_validate_cypher_returns_tuple(self):
        """Test that validate_cypher returns a tuple (errors, type_issues)"""
        query = "MATCH (p:Person) RETURN p"

        result = validate_cypher(query, self.schema)

        # Should return tuple
        self.assertIsInstance(result, tuple, "validate_cypher should return a tuple")
        self.assertEqual(len(result), 2, "Tuple should have 2 elements")

        errors, type_issues = result
        self.assertIsInstance(errors, list, "First element should be list of errors")
        self.assertIsInstance(type_issues, list, "Second element should be list of type issues")

    def test_valid_query_no_type_checking(self):
        """Test valid query without type checking (default Off)"""
        query = "MATCH (p:Person) WHERE p.age = 'string' RETURN p"

        errors, type_issues = validate_cypher(query, self.schema)

        # No schema errors (labels/properties exist)
        self.assertEqual(len(errors), 0, "Valid schema should have no errors")
        # No type issues (type checking is Off by default)
        self.assertEqual(len(type_issues), 0, "Type checking Off should have no type issues")

    def test_type_checking_warnings(self):
        """Test type checking detects mismatches in Warnings mode"""
        query = "MATCH (p:Person) WHERE p.age = 'twenty-five' RETURN p"

        errors, type_issues = validate_cypher(query, self.schema, TypeCheckLevel.Warnings)

        # No schema errors
        self.assertEqual(len(errors), 0, "Should have no schema errors")
        # Should have type issue
        self.assertEqual(len(type_issues), 1, "Should detect type mismatch")
        self.assertIn("age", type_issues[0].message.lower(), "Should mention 'age' property")
        self.assertIn("integer", type_issues[0].message.lower(), "Should mention INTEGER type")
        self.assertIn("string", type_issues[0].message.lower(), "Should mention STRING comparison")

    def test_type_checking_strict(self):
        """Test type checking in Strict mode"""
        query = "MATCH (p:Person) WHERE p.active = 1 RETURN p"

        errors, type_issues = validate_cypher(query, self.schema, TypeCheckLevel.Strict)

        self.assertEqual(len(errors), 0, "Should have no schema errors")
        self.assertEqual(len(type_issues), 1, "Should detect BOOLEAN vs INTEGER mismatch")

    def test_relationship_property_type_checking(self):
        """Test relationship property type checking works (was broken before fix)"""
        query = "MATCH (p:Person)-[r:WORKS_FOR]->(c:Company) WHERE r.salary = 'high' RETURN p"

        errors, type_issues = validate_cypher(query, self.schema, TypeCheckLevel.Strict)

        self.assertEqual(len(errors), 0, "Should have no schema errors")
        self.assertEqual(len(type_issues), 1, "Should detect relationship property type mismatch")
        self.assertIn("salary", type_issues[0].message.lower())
        self.assertIn("float", type_issues[0].message.lower())

    def test_invalid_label_error(self):
        """Test invalid label returns proper error"""
        query = "MATCH (e:Employee) RETURN e"  # Employee doesn't exist in schema

        errors, type_issues = validate_cypher(query, self.schema, TypeCheckLevel.Strict)

        # Should have schema error
        self.assertGreater(len(errors), 0, "Should have schema error for invalid label")
        self.assertTrue(
            any("employee" in err.lower() for err in errors),
            "Error should mention 'Employee' label"
        )

    def test_invalid_property_error(self):
        """Test invalid property returns proper error"""
        query = "MATCH (p:Person) WHERE p.invalidProp = 'test' RETURN p"

        errors, type_issues = validate_cypher(query, self.schema, TypeCheckLevel.Strict)

        self.assertGreater(len(errors), 0, "Should have error for invalid property")
        self.assertTrue(
            any("invalidprop" in err.lower() for err in errors),
            "Error should mention invalid property"
        )

    def test_multiple_type_issues(self):
        """Test multiple type mismatches are detected"""
        query = "MATCH (p:Person) WHERE p.age = 'thirty' AND p.active = 1 RETURN p"

        errors, type_issues = validate_cypher(query, self.schema, TypeCheckLevel.Strict)

        self.assertEqual(len(errors), 0, "Should have no schema errors")
        self.assertEqual(len(type_issues), 2, "Should detect both type mismatches")


class TestParserErrorHandling(unittest.TestCase):
    """Test that parser error handling doesn't crash with NameError"""

    def setUp(self):
        """Create a test schema"""
        self.schema_dict = {
            "node_props": {
                "Person": [{"name": "name", "neo4j_type": "STRING"}],
            },
            "rel_props": {},
            "relationships": [],
            "metadata": {"constraint": [], "index": []},
        }
        self.schema = DbSchema.from_dict(self.schema_dict)

    def test_complex_query_doesnt_crash(self):
        """Test that queries don't crash with NameError about undefined 'error' variable"""
        # Test that we properly handle all code paths without NameError
        # The query itself doesn't matter - we're testing the error handling code
        query = "MATCH (n:NonExistentLabel) RETURN n"

        try:
            errors, type_issues = validate_cypher(query, self.schema)
            # If we get here, no crash occurred
            self.assertIsInstance(errors, list)
            self.assertIsInstance(type_issues, list)
            # Should have validation errors for the non-existent label
            self.assertGreater(len(errors), 0, "Should detect invalid label")
        except NameError as e:
            if "'error' is not defined" in str(e):
                self.fail("NameError about undefined 'error' variable - Bug #2 not fixed!")
            raise


class TestIteratingOverErrors(unittest.TestCase):
    """Test that iterating over errors works (was broken when errors was a tuple)"""

    def setUp(self):
        self.schema_dict = {
            "node_props": {
                "Person": [{"name": "name", "neo4j_type": "STRING"}],
            },
            "rel_props": {},
            "relationships": [],
            "metadata": {"constraint": [], "index": []},
        }
        self.schema = DbSchema.from_dict(self.schema_dict)

    def test_can_iterate_over_errors(self):
        """Test that we can iterate over errors and call .lower()"""
        query = "MATCH (e:Employee) RETURN e"  # Invalid label

        errors, type_issues = validate_cypher(query, self.schema)

        # This was the original bug - trying to call .lower() on a list
        for error in errors:
            # Should be able to call .lower() on each error string
            try:
                error_lower = error.lower()
                self.assertIsInstance(error_lower, str, "error.lower() should return string")
            except AttributeError as e:
                if "'list' object has no attribute 'lower'" in str(e):
                    self.fail("Bug #1 not fixed - errors is still a list instead of strings!")
                raise

    def test_error_is_string_not_list(self):
        """Test that each error is a string, not a list"""
        query = "MATCH (e:Employee) RETURN e"

        errors, type_issues = validate_cypher(query, self.schema)

        if errors:
            first_error = errors[0]
            self.assertIsInstance(first_error, str, "Each error should be a string")
            self.assertNotIsInstance(first_error, list, "Error should not be a list")


class TestBackwardCompatibility(unittest.TestCase):
    """Test that code using the new API works correctly"""

    def setUp(self):
        self.schema_dict = {
            "node_props": {
                "Person": [
                    {"name": "age", "neo4j_type": "INTEGER"},
                ],
            },
            "rel_props": {},
            "relationships": [],
            "metadata": {"constraint": [], "index": []},
        }
        self.schema = DbSchema.from_dict(self.schema_dict)

    def test_unpack_with_default_type_checking(self):
        """Test unpacking works with default (no type_check_level specified)"""
        query = "MATCH (p:Person) WHERE p.age = 'string' RETURN p"

        # This is how the fixed code calls it
        errors, type_issues = validate_cypher(query, self.schema)

        self.assertIsInstance(errors, list)
        self.assertIsInstance(type_issues, list)
        self.assertEqual(len(type_issues), 0, "Default should be Off (no type checking)")

    def test_unpack_with_strict_type_checking(self):
        """Test unpacking works with explicit TypeCheckLevel"""
        query = "MATCH (p:Person) WHERE p.age = 'string' RETURN p"

        # This is how the fixed code calls it with type checking
        errors, type_issues = validate_cypher(query, self.schema, TypeCheckLevel.Strict)

        self.assertIsInstance(errors, list)
        self.assertIsInstance(type_issues, list)
        self.assertGreater(len(type_issues), 0, "Strict mode should detect type issues")

    def test_can_discard_type_issues(self):
        """Test that we can use _ to discard type issues if not needed"""
        query = "MATCH (p:Person) RETURN p"

        # This is how line 1250 calls it (pattern validation doesn't need type issues)
        errors, _ = validate_cypher(query, self.schema)

        self.assertIsInstance(errors, list)


class TestTypeIssueObjects(unittest.TestCase):
    """Test that TypeIssue objects have the expected structure"""

    def setUp(self):
        self.schema_dict = {
            "node_props": {
                "Person": [
                    {"name": "age", "neo4j_type": "INTEGER"},
                    {"name": "active", "neo4j_type": "BOOLEAN"},
                ],
            },
            "rel_props": {},
            "relationships": [],
            "metadata": {"constraint": [], "index": []},
        }
        self.schema = DbSchema.from_dict(self.schema_dict)

    def test_type_issue_has_severity(self):
        """Test TypeIssue objects have severity attribute"""
        query = "MATCH (p:Person) WHERE p.age = 'string' RETURN p"

        errors, type_issues = validate_cypher(query, self.schema, TypeCheckLevel.Warnings)

        if type_issues:
            issue = type_issues[0]
            self.assertTrue(hasattr(issue, 'severity'), "TypeIssue should have severity")
            self.assertTrue(hasattr(issue, 'message'), "TypeIssue should have message")
            # Test that we can call __str__() on severity (as the fixed code does)
            severity_str = issue.severity.__str__()
            self.assertIsInstance(severity_str, str)
            self.assertIn(severity_str, ['warning', 'error'])

    def test_type_issue_has_message(self):
        """Test TypeIssue message is descriptive"""
        query = "MATCH (p:Person) WHERE p.age = 'string' RETURN p"

        errors, type_issues = validate_cypher(query, self.schema, TypeCheckLevel.Strict)

        if type_issues:
            issue = type_issues[0]
            self.assertIsInstance(issue.message, str)
            self.assertGreater(len(issue.message), 0, "Message should not be empty")

    def test_type_issue_suggestion(self):
        """Test TypeIssue can have optional suggestion"""
        query = "MATCH (p:Person) WHERE p.active = 1 RETURN p"

        errors, type_issues = validate_cypher(query, self.schema, TypeCheckLevel.Strict)

        if type_issues:
            issue = type_issues[0]
            # Suggestion might be None or a string
            self.assertTrue(
                issue.suggestion is None or isinstance(issue.suggestion, str),
                "Suggestion should be None or string"
            )


def run_tests():
    """Run all tests and print results"""
    print("=" * 70)
    print("Running Unit Tests for API Fixes")
    print("=" * 70)
    print()

    # Create test suite
    loader = unittest.TestLoader()
    suite = unittest.TestSuite()

    # Add all test classes
    suite.addTests(loader.loadTestsFromTestCase(TestValidateCypherAPI))
    suite.addTests(loader.loadTestsFromTestCase(TestParserErrorHandling))
    suite.addTests(loader.loadTestsFromTestCase(TestIteratingOverErrors))
    suite.addTests(loader.loadTestsFromTestCase(TestBackwardCompatibility))
    suite.addTests(loader.loadTestsFromTestCase(TestTypeIssueObjects))

    # Run tests with verbose output
    runner = unittest.TextTestRunner(verbosity=2)
    result = runner.run(suite)

    print()
    print("=" * 70)
    print("Test Summary")
    print("=" * 70)
    print(f"Tests run: {result.testsRun}")
    print(f"Successes: {result.testsRun - len(result.failures) - len(result.errors)}")
    print(f"Failures: {len(result.failures)}")
    print(f"Errors: {len(result.errors)}")
    print()

    if result.wasSuccessful():
        print("✅ ALL TESTS PASSED!")
        print()
        print("Verified:")
        print("  ✓ validate_cypher returns tuple (errors, type_issues)")
        print("  ✓ Can unpack tuple correctly")
        print("  ✓ Can iterate over errors and call .lower()")
        print("  ✓ Type checking works (Warnings and Strict modes)")
        print("  ✓ Relationship property type checking works")
        print("  ✓ No NameError about undefined 'error' variable")
        print("  ✓ TypeIssue objects have correct structure")
        print("  ✓ Backward compatibility maintained")
        return 0
    else:
        print("❌ SOME TESTS FAILED")
        print()
        print("Please review the failures above.")
        return 1


if __name__ == "__main__":
    sys.exit(run_tests())
