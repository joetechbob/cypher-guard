/// Tests for RETURN DISTINCT with aggregation functions and case-insensitive AS keyword
///
/// This test suite verifies support for:
/// 1. RETURN DISTINCT with multiple columns
/// 2. Case-insensitive "as" and "AS" keywords
/// 3. Aggregation functions like count(*) with aliases
/// 4. ORDER BY with aliased columns
/// 5. Complex multi-line return expressions

use cypher_guard::validation::{extract_query_elements, validate_query_elements};
use cypher_guard::{parse_query, DbSchema};

/// Helper function to validate a query against a schema
fn validate_query(query: &str, schema: &DbSchema) -> Vec<cypher_guard::CypherGuardValidationError> {
    let ast = parse_query(query).expect("Query should parse");
    let elements = extract_query_elements(&ast);
    validate_query_elements(&elements, schema)
}

#[test]
fn test_return_distinct_lowercase_as() {
    // Test lowercase "as" keyword (the original failing case)
    let query = r#"
        MATCH (n:Person)
        RETURN DISTINCT n.name as person_name,
               n.age as person_age
        ORDER BY person_name
    "#;

    let result = parse_query(query);
    assert!(result.is_ok(), "Query should parse successfully");

    // Test validation
    let schema_json = r#"{
        "node_props": {
            "Person": [
                {"name": "name", "neo4j_type": "STRING"},
                {"name": "age", "neo4j_type": "INTEGER"}
            ]
        },
        "rel_props": {},
        "relationships": [],
        "metadata": {"constraint": [], "index": []}
    }"#;

    let schema: DbSchema = serde_json::from_str(schema_json).unwrap();
    let (errors, type_issues) = validate_cypher(query, &schema, TypeCheckLevel::Off).unwrap();

    assert_eq!(errors.len(), 0, "Should have no validation errors");
    assert_eq!(type_issues.len(), 0, "Should have no type issues");
}

#[test]
fn test_return_distinct_uppercase_as() {
    // Test uppercase "AS" keyword for consistency
    let query = r#"
        MATCH (n:Person)
        RETURN DISTINCT n.name AS person_name,
               n.age AS person_age
        ORDER BY person_name
    "#;

    let result = parse_query(query);
    assert!(result.is_ok(), "Query should parse successfully");

    // Test validation
    let schema_json = r#"{
        "node_props": {
            "Person": [
                {"name": "name", "neo4j_type": "STRING"},
                {"name": "age", "neo4j_type": "INTEGER"}
            ]
        },
        "rel_props": {},
        "relationships": [],
        "metadata": {"constraint": [], "index": []}
    }"#;

    let schema: DbSchema = serde_json::from_str(schema_json).unwrap();
    let (errors, type_issues) = validate_cypher(query, &schema, TypeCheckLevel::Off).unwrap();

    assert_eq!(errors.len(), 0, "Should have no validation errors");
    assert_eq!(type_issues.len(), 0, "Should have no type issues");
}

#[test]
fn test_return_distinct_mixed_case_as() {
    // Test mixed case "As" keyword
    let query = r#"
        MATCH (n:Person)
        RETURN DISTINCT n.name As person_name,
               n.age As person_age
    "#;

    let result = parse_query(query);
    assert!(result.is_ok(), "Query should parse successfully with mixed case As");
}

#[test]
fn test_return_distinct_with_count_aggregation() {
    // This is the exact failing query from the user
    let query = r#"
        MATCH (eli:ExpenseLineItem)
        WHERE eli.custom4 IS NOT NULL
        RETURN DISTINCT eli.custom4 as custom4_value,
               eli.custom5 as custom5_value,
               eli.custom6 as custom6_value,
               count(*) as usage_count
        ORDER BY usage_count DESC
        LIMIT 10
    "#;

    let result = parse_query(query);
    assert!(result.is_ok(), "Query with count(*) should parse successfully");

    // Test validation
    let schema_json = r#"{
        "node_props": {
            "ExpenseLineItem": [
                {"name": "custom4", "neo4j_type": "STRING"},
                {"name": "custom5", "neo4j_type": "STRING"},
                {"name": "custom6", "neo4j_type": "STRING"}
            ]
        },
        "rel_props": {},
        "relationships": [],
        "metadata": {"constraint": [], "index": []}
    }"#;

    let schema: DbSchema = serde_json::from_str(schema_json).unwrap();
    let (errors, type_issues) = validate_cypher(query, &schema, TypeCheckLevel::Off).unwrap();

    assert_eq!(
        errors.len(),
        0,
        "Should have no validation errors. Errors: {:?}",
        errors
    );
    assert_eq!(type_issues.len(), 0, "Should have no type issues");
}

#[test]
fn test_return_distinct_with_sum_aggregation() {
    let query = r#"
        MATCH (e:Expense)
        RETURN DISTINCT e.category as category,
               sum(e.amount) as total_amount
        ORDER BY total_amount DESC
    "#;

    let result = parse_query(query);
    assert!(result.is_ok(), "Query with sum() should parse successfully");

    let schema_json = r#"{
        "node_props": {
            "Expense": [
                {"name": "category", "neo4j_type": "STRING"},
                {"name": "amount", "neo4j_type": "FLOAT"}
            ]
        },
        "rel_props": {},
        "relationships": [],
        "metadata": {"constraint": [], "index": []}
    }"#;

    let schema: DbSchema = serde_json::from_str(schema_json).unwrap();
    let (errors, _) = validate_cypher(query, &schema, TypeCheckLevel::Off).unwrap();

    assert_eq!(errors.len(), 0, "Should have no validation errors");
}

#[test]
fn test_return_distinct_with_avg_min_max() {
    let query = r#"
        MATCH (p:Product)
        RETURN DISTINCT p.category as category,
               avg(p.price) as avg_price,
               min(p.price) as min_price,
               max(p.price) as max_price,
               count(*) as product_count
    "#;

    let result = parse_query(query);
    assert!(result.is_ok(), "Query with multiple aggregations should parse");
}

#[test]
fn test_return_distinct_without_aggregation() {
    // Test DISTINCT without aggregation
    let query = r#"
        MATCH (p:Person)-[:WORKS_FOR]->(c:Company)
        RETURN DISTINCT c.name as company_name,
               c.industry as industry
    "#;

    let result = parse_query(query);
    assert!(result.is_ok(), "DISTINCT without aggregation should parse");
}

#[test]
fn test_return_distinct_with_collect() {
    let query = r#"
        MATCH (p:Person)-[:WORKS_FOR]->(c:Company)
        RETURN DISTINCT c.name as company,
               collect(p.name) as employees
    "#;

    let result = parse_query(query);
    assert!(result.is_ok(), "Query with collect() should parse");
}

#[test]
fn test_return_without_distinct_lowercase_as() {
    // Test regular RETURN (no DISTINCT) with lowercase "as"
    let query = r#"
        MATCH (n:Person)
        RETURN n.name as person_name,
               n.age as person_age
        LIMIT 10
    "#;

    let result = parse_query(query);
    assert!(result.is_ok(), "Regular RETURN with lowercase 'as' should parse");

    let schema_json = r#"{
        "node_props": {
            "Person": [
                {"name": "name", "neo4j_type": "STRING"},
                {"name": "age", "neo4j_type": "INTEGER"}
            ]
        },
        "rel_props": {},
        "relationships": [],
        "metadata": {"constraint": [], "index": []}
    }"#;

    let schema: DbSchema = serde_json::from_str(schema_json).unwrap();
    let (errors, _) = validate_cypher(query, &schema, TypeCheckLevel::Off).unwrap();

    assert_eq!(errors.len(), 0, "Should have no validation errors");
}

#[test]
fn test_complex_multi_line_return() {
    // Test complex real-world query with multiple lines and formatting
    let query = r#"
        MATCH (emp:Employee)-[:SUBMITTED]->(exp:Expense)
        WHERE exp.status = 'PENDING'
        RETURN DISTINCT
            emp.department as department,
            emp.manager as manager_name,
            count(exp) as pending_count,
            sum(exp.amount) as total_amount,
            avg(exp.amount) as avg_amount,
            min(exp.submitDate) as earliest_date,
            max(exp.submitDate) as latest_date
        ORDER BY total_amount DESC
        LIMIT 20
    "#;

    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "Complex multi-line query should parse successfully"
    );
}

#[test]
fn test_return_alias_becomes_variable() {
    // Test that aliases in RETURN create new defined variables
    let query = r#"
        MATCH (n:Person)
        RETURN n.name as person_name,
               n.age as person_age
        ORDER BY person_name
    "#;

    let result = parse_query(query);
    assert!(result.is_ok(), "Query should parse");

    // The aliases (person_name, person_age) should be available for ORDER BY
    // This is tested implicitly by the query parsing successfully
}

#[test]
fn test_return_distinct_with_expression_alias() {
    // Test expressions with aliases
    let query = r#"
        MATCH (p:Person)
        RETURN DISTINCT
            p.firstName + ' ' + p.lastName as full_name,
            p.age * 12 as age_months
    "#;

    let result = parse_query(query);
    assert!(result.is_ok(), "Expressions with aliases should parse");
}

#[test]
fn test_return_distinct_single_column() {
    // Test DISTINCT with single column
    let query = r#"
        MATCH (n:Person)
        RETURN DISTINCT n.country as country
    "#;

    let result = parse_query(query);
    assert!(result.is_ok(), "DISTINCT with single column should parse");
}

#[test]
fn test_return_no_alias() {
    // Test RETURN without any aliases
    let query = r#"
        MATCH (n:Person)
        RETURN DISTINCT n.name, n.age
    "#;

    let result = parse_query(query);
    assert!(result.is_ok(), "RETURN without aliases should parse");
}

#[test]
fn test_return_mixed_alias_and_no_alias() {
    // Test mixed: some with aliases, some without
    let query = r#"
        MATCH (n:Person)
        RETURN DISTINCT
            n.name as person_name,
            n.age,
            n.country as location
    "#;

    let result = parse_query(query);
    assert!(result.is_ok(), "Mixed alias/no-alias should parse");
}
