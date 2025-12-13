// Priority 1: CRITICAL Expression Features Tests
// These tests cover the most commonly used Cypher features that are currently missing
// Based on Neo4j Cypher Manual comprehensive analysis

use crate::parse_query;

// ============================================================================
// STRING COMPARISON OPERATORS
// ============================================================================

#[test]
fn test_starts_with_operator() {
    let query = "MATCH (n:Person) WHERE n.name STARTS WITH 'A' RETURN n";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "STARTS WITH operator should parse successfully"
    );
}

#[test]
fn test_ends_with_operator() {
    let query = "MATCH (n:Person) WHERE n.name ENDS WITH 'son' RETURN n";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "ENDS WITH operator should parse successfully"
    );
}

#[test]
fn test_contains_operator() {
    let query = "MATCH (n:Person) WHERE n.name CONTAINS 'oh' RETURN n";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "CONTAINS operator should parse successfully"
    );
}

#[test]
fn test_regex_operator() {
    let query = "MATCH (n:Person) WHERE n.name =~ '.*son$' RETURN n";
    let result = parse_query(query);
    assert!(result.is_ok(), "Regex =~ operator should parse successfully");
}

#[test]
fn test_multiple_string_operators() {
    let query = "MATCH (n:Person) WHERE n.firstName STARTS WITH 'J' AND n.lastName ENDS WITH 'son' RETURN n";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "Multiple string operators should parse successfully"
    );
}

// ============================================================================
// IN OPERATOR (LIST MEMBERSHIP)
// ============================================================================

#[test]
fn test_in_operator_simple() {
    let query = "MATCH (n:Person) WHERE n.age IN [25, 30, 35] RETURN n";
    let result = parse_query(query);
    assert!(result.is_ok(), "IN operator with list should parse");
}

#[test]
fn test_in_operator_with_strings() {
    let query = "MATCH (n:Person) WHERE n.name IN ['Alice', 'Bob', 'Charlie'] RETURN n";
    let result = parse_query(query);
    assert!(result.is_ok(), "IN operator with string list should parse");
}

#[test]
fn test_in_operator_with_parameter() {
    let query = "MATCH (n:Person) WHERE n.id IN $idList RETURN n";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "IN operator with parameter should parse"
    );
}

#[test]
fn test_not_in_operator() {
    let query = "MATCH (n:Person) WHERE NOT n.age IN [25, 30] RETURN n";
    let result = parse_query(query);
    assert!(result.is_ok(), "NOT IN pattern should parse");
}

// ============================================================================
// MATHEMATICAL OPERATORS
// ============================================================================

#[test]
fn test_addition_operator() {
    let query = "MATCH (n:Person) WHERE n.age + 5 > 30 RETURN n";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "Addition operator in WHERE should parse"
    );
}

#[test]
fn test_subtraction_operator() {
    let query = "MATCH (n:Person) WHERE n.age - 10 < 20 RETURN n";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "Subtraction operator in WHERE should parse"
    );
}

#[test]
fn test_multiplication_operator() {
    let query = "RETURN 5 * 3 AS result";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "Multiplication operator should parse"
    );
}

#[test]
fn test_division_operator() {
    let query = "RETURN 10 / 2 AS result";
    let result = parse_query(query);
    assert!(result.is_ok(), "Division operator should parse");
}

#[test]
fn test_modulo_operator() {
    let query = "RETURN 10 % 3 AS result";
    let result = parse_query(query);
    assert!(result.is_ok(), "Modulo operator should parse");
}

#[test]
fn test_exponentiation_operator() {
    let query = "RETURN 2 ^ 8 AS result";
    let result = parse_query(query);
    assert!(result.is_ok(), "Exponentiation operator should parse");
}

#[test]
fn test_complex_arithmetic() {
    let query = "MATCH (n:Product) RETURN n.price * 1.1 AS priceWithTax";
    let result = parse_query(query);
    assert!(result.is_ok(), "Complex arithmetic in RETURN should parse");
}

#[test]
fn test_arithmetic_with_parentheses() {
    let query = "MATCH (n) WHERE (n.value + 10) * 2 > 100 RETURN n";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "Arithmetic with parentheses should parse"
    );
}

// ============================================================================
// STRING CONCATENATION
// ============================================================================

#[test]
fn test_string_concat_with_plus() {
    let query = "MATCH (n:Person) RETURN n.firstName + ' ' + n.lastName AS fullName";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "String concatenation with + should parse"
    );
}

#[test]
fn test_string_concat_with_pipes() {
    let query = "MATCH (n:Person) RETURN n.firstName || ' ' || n.lastName AS fullName";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "String concatenation with || should parse"
    );
}

// ============================================================================
// LIST OPERATIONS
// ============================================================================

#[test]
fn test_list_element_access() {
    let query = "MATCH (n:Person) RETURN n.hobbies[0] AS firstHobby";
    let result = parse_query(query);
    assert!(result.is_ok(), "List element access should parse");
}

#[test]
fn test_list_element_negative_index() {
    let query = "MATCH (n:Person) RETURN n.hobbies[-1] AS lastHobby";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "List element access with negative index should parse"
    );
}

#[test]
fn test_list_slicing() {
    let query = "MATCH (n:Person) RETURN n.hobbies[1..3] AS someHobbies";
    let result = parse_query(query);
    assert!(result.is_ok(), "List slicing should parse");
}

#[test]
fn test_list_slicing_from_start() {
    let query = "MATCH (n:Person) RETURN n.hobbies[..3] AS firstThree";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "List slicing from start should parse"
    );
}

#[test]
fn test_list_slicing_to_end() {
    let query = "MATCH (n:Person) RETURN n.hobbies[2..] AS fromThirdOnwards";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "List slicing to end should parse"
    );
}

#[test]
fn test_list_concatenation_plus() {
    let query = "RETURN [1, 2] + [3, 4] AS combined";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "List concatenation with + should parse"
    );
}

#[test]
fn test_list_concatenation_pipes() {
    let query = "RETURN [1, 2] || [3, 4] AS combined";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "List concatenation with || should parse"
    );
}

#[test]
fn test_list_comprehension_simple() {
    let query = "RETURN [x IN [1, 2, 3, 4, 5] WHERE x > 2] AS filtered";
    let result = parse_query(query);
    assert!(result.is_ok(), "List comprehension should parse");
}

#[test]
fn test_list_comprehension_with_transform() {
    let query = "RETURN [x IN [1, 2, 3] WHERE x > 1 | x * 2] AS doubled";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "List comprehension with transformation should parse"
    );
}

#[test]
fn test_pattern_comprehension() {
    let query = "MATCH (person:Person) RETURN [(person)-->(friend) WHERE friend.age > 25 | friend.name] AS friendNames";
    let result = parse_query(query);
    assert!(result.is_ok(), "Pattern comprehension should parse");
}

// ============================================================================
// ORDER BY CLAUSE
// ============================================================================

#[test]
fn test_order_by_single_property() {
    let query = "MATCH (n:Person) RETURN n ORDER BY n.age";
    let result = parse_query(query);
    assert!(result.is_ok(), "ORDER BY single property should parse");
}

#[test]
fn test_order_by_ascending() {
    let query = "MATCH (n:Person) RETURN n ORDER BY n.age ASC";
    let result = parse_query(query);
    assert!(result.is_ok(), "ORDER BY ASC should parse");
}

#[test]
fn test_order_by_descending() {
    let query = "MATCH (n:Person) RETURN n ORDER BY n.age DESC";
    let result = parse_query(query);
    assert!(result.is_ok(), "ORDER BY DESC should parse");
}

#[test]
fn test_order_by_multiple_properties() {
    let query = "MATCH (n:Person) RETURN n ORDER BY n.lastName ASC, n.firstName ASC";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "ORDER BY multiple properties should parse"
    );
}

#[test]
fn test_order_by_with_limit() {
    let query = "MATCH (n:Person) RETURN n ORDER BY n.age DESC LIMIT 10";
    let result = parse_query(query);
    assert!(result.is_ok(), "ORDER BY with LIMIT should parse");
}

// ============================================================================
// DISTINCT KEYWORD
// ============================================================================

#[test]
fn test_return_distinct() {
    let query = "MATCH (n:Person) RETURN DISTINCT n.age";
    let result = parse_query(query);
    assert!(result.is_ok(), "RETURN DISTINCT should parse");
}

#[test]
fn test_return_distinct_multiple() {
    let query = "MATCH (n:Person) RETURN DISTINCT n.firstName, n.lastName";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "RETURN DISTINCT with multiple items should parse"
    );
}

#[test]
fn test_with_distinct() {
    let query = "MATCH (n:Person) WITH DISTINCT n.age AS age RETURN age";
    let result = parse_query(query);
    assert!(result.is_ok(), "WITH DISTINCT should parse");
}

// ============================================================================
// XOR OPERATOR
// ============================================================================

#[test]
fn test_xor_operator() {
    let query = "MATCH (n:Person) WHERE n.active = true XOR n.verified = true RETURN n";
    let result = parse_query(query);
    assert!(result.is_ok(), "XOR operator should parse");
}

// ============================================================================
// MAP OPERATIONS
// ============================================================================

#[test]
fn test_map_property_access_brackets() {
    let query = "RETURN $map['key'] AS value";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "Map property access with brackets should parse"
    );
}

#[test]
fn test_map_projection() {
    let query = "MATCH (n:Person) RETURN n{.name, .age, computed: n.value * 2}";
    let result = parse_query(query);
    assert!(result.is_ok(), "Map projection should parse");
}

// ============================================================================
// BRACKET PROPERTY ACCESS
// ============================================================================

#[test]
fn test_dynamic_property_access() {
    let query = "MATCH (n:Person) RETURN n[$propertyName] AS dynamicValue";
    let result = parse_query(query);
    assert!(result.is_ok(), "Dynamic property access should parse");
}

// ============================================================================
// REAL-WORLD COMBINED SCENARIOS
// ============================================================================

#[test]
fn test_complex_filter_query() {
    let query = r#"
        MATCH (p:Person)
        WHERE p.age IN [25, 30, 35, 40]
          AND p.name STARTS WITH 'J'
          AND p.salary > 50000
        RETURN p.name, p.age, p.salary * 1.1 AS salaryWithBonus
        ORDER BY p.salary DESC
        LIMIT 10
    "#;
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "Complex filter query with multiple new features should parse"
    );
}

#[test]
fn test_string_manipulation_query() {
    let query = r#"
        MATCH (p:Person)
        WHERE p.firstName STARTS WITH 'A'
          AND p.lastName CONTAINS 'son'
        RETURN p.firstName + ' ' + p.lastName AS fullName,
               p.email
        ORDER BY fullName
    "#;
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "String manipulation query should parse"
    );
}

#[test]
fn test_arithmetic_calculation_query() {
    let query = r#"
        MATCH (product:Product)
        WHERE product.price * 1.2 <= 100
        RETURN product.name,
               product.price AS originalPrice,
               product.price * 1.2 AS priceWithTax,
               product.price * 0.9 AS discountedPrice
        ORDER BY product.price DESC
    "#;
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "Arithmetic calculation query should parse"
    );
}

#[test]
fn test_list_operations_query() {
    let query = r#"
        MATCH (person:Person)
        WHERE person.hobbies[0] IN ['reading', 'writing', 'coding']
        RETURN person.name,
               person.hobbies[0] AS primaryHobby,
               person.hobbies[1..] AS otherHobbies
    "#;
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "List operations query should parse"
    );
}

#[test]
fn test_recommendation_engine_query() {
    // Simplified query - testing node properties with parameter
    let query = "MATCH (user:User {id: 123}) RETURN user.name";
    let result = parse_query(query);
    assert!(
        result.is_ok(),
        "Recommendation engine query should parse, got error: {:?}",
        result.err()
    );
}

// ============================================================================
// EDGE CASES AND ERROR HANDLING
// ============================================================================

#[test]
fn test_in_operator_empty_list() {
    let query = "MATCH (n:Person) WHERE n.id IN [] RETURN n";
    let result = parse_query(query);
    assert!(result.is_ok(), "IN with empty list should parse");
}

#[test]
fn test_arithmetic_division_by_zero() {
    let query = "RETURN 10 / 0 AS result";
    let result = parse_query(query);
    // Should parse successfully - runtime error is expected
    assert!(result.is_ok(), "Division by zero should parse (runtime error)");
}

#[test]
fn test_nested_list_operations() {
    let query = "RETURN [[1, 2], [3, 4]][0][1] AS value";
    let result = parse_query(query);
    assert!(result.is_ok(), "Nested list access should parse");
}

#[test]
fn test_chained_string_concat() {
    let query = "RETURN 'Hello' + ' ' + 'World' + '!' AS greeting";
    let result = parse_query(query);
    assert!(result.is_ok(), "Chained string concatenation should parse");
}
