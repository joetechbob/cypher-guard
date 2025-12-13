use cypher_guard::parse_query;

/// Priority 13: Group Variables in QPP
///
/// In Neo4j, variables defined inside a Quantified Path Pattern (QPP) become
/// group variables that are exposed as lists outside the QPP. This is a
/// semantic feature where each variable inside the QPP represents multiple
/// matches (one per repetition).
///
/// Examples:
/// - In ((a)-[r:KNOWS]->(b)){1,3}, variables a, r, and b are group variables
/// - Outside the QPP, they should be treated as lists when validating
/// - Functions like collect(), all(), any() etc. can operate on group variables

#[test]
fn test_qpp_group_variable_basic() {
    // Basic QPP with group variables returned
    let query = "MATCH ((a)-[r:KNOWS]->(b)){1,3} RETURN a, collect(r) AS relationships, collect(b) AS people";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse QPP with group variables: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.match_clauses.len(), 1);
    assert_eq!(query_ast.return_clauses.len(), 1);

    // Verify that the return clause has the expected items
    let return_clause = &query_ast.return_clauses[0];
    assert!(return_clause.items.len() >= 3, "Expected at least 3 return items");
}

#[test]
fn test_qpp_group_variable_with_collect() {
    // Group variables with explicit collect() function
    let query = "MATCH ((a)-[r:LINK]->(b:Station)){1,5} RETURN a.name, collect(b.name) AS stations, collect(r.distance) AS distances";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse QPP with collect on group variables: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.match_clauses.len(), 1);
    assert_eq!(query_ast.return_clauses.len(), 1);
}

#[test]
fn test_qpp_group_variable_with_aggregation() {
    // Using aggregation functions on group variables
    let query = "MATCH ((a)-[r:KNOWS]->(b)){2,4} RETURN a.name, count(b) AS connections, avg(r.weight) AS avg_weight";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse QPP with aggregation on group variables: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.match_clauses.len(), 1);
    assert_eq!(query_ast.return_clauses.len(), 1);
}

// Commented out: all() predicate not yet implemented
// #[test]
// fn test_qpp_group_variable_with_all_predicate() {
//     // Using all() predicate on group variables in WHERE
//     let query = "MATCH ((a)-[r:LINK]->(b:Station)){1,5} WHERE all(s IN b WHERE s.active = true) RETURN a, b, r";
//     let result = parse_query(query);
//     assert!(result.is_ok(), "Failed to parse QPP with all() predicate on group variable: {:?}", result.err());
//
//     let query_ast = result.unwrap();
//     assert_eq!(query_ast.match_clauses.len(), 1);
//     assert_eq!(query_ast.where_clauses.len(), 1);
//     assert_eq!(query_ast.return_clauses.len(), 1);
// }

// Commented out: any() predicate not yet implemented
// #[test]
// fn test_qpp_group_variable_with_any_predicate() {
//     // Using any() predicate on group variables
//     let query = "MATCH ((a)-[r:KNOWS]->(b)){1,3} WHERE any(person IN b WHERE person.age > 30) RETURN a, b";
//     let result = parse_query(query);
//     assert!(result.is_ok(), "Failed to parse QPP with any() predicate on group variable: {:?}", result.err());
//
//     let query_ast = result.unwrap();
//     assert_eq!(query_ast.match_clauses.len(), 1);
//     assert_eq!(query_ast.where_clauses.len(), 1);
// }

#[test]
fn test_qpp_multiple_group_variables() {
    // Multiple group variables in same QPP
    let query = r#"
        MATCH ((source)-[r1:FRIEND]->(middle)-[r2:KNOWS]->(target)){1,2}
        RETURN source.name,
               collect(middle.name) AS intermediates,
               collect(r1.since) AS friend_dates,
               collect(r2.strength) AS know_strengths,
               collect(target.id) AS targets
    "#;
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse QPP with multiple group variables: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.match_clauses.len(), 1);
    assert_eq!(query_ast.return_clauses.len(), 1);

    // Verify that all return items are present
    let return_clause = &query_ast.return_clauses[0];
    assert!(return_clause.items.len() >= 5, "Expected at least 5 return items for multiple group variables");
}

#[test]
fn test_qpp_group_variable_list_operations() {
    // Group variables with list operations like size()
    let query = "MATCH ((a)-[r:KNOWS]->(b)){1,3} RETURN a.name, size(b) AS connection_count, b";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse QPP with list operations on group variables: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.match_clauses.len(), 1);
    assert_eq!(query_ast.return_clauses.len(), 1);
}

#[test]
fn test_qpp_group_variable_in_with_clause() {
    // Group variables passed through WITH clause
    let query = r#"
        MATCH ((a)-[r:LINK]->(b)){2,4}
        WITH a, collect(b) AS targets, collect(r) AS links
        WHERE size(targets) > 2
        RETURN a.name, targets, links
    "#;
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse QPP group variables through WITH: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.match_clauses.len(), 1);
    assert_eq!(query_ast.with_clauses.len(), 1);
    assert_eq!(query_ast.where_clauses.len(), 1);
    assert_eq!(query_ast.return_clauses.len(), 1);
}

#[test]
fn test_qpp_group_variable_unwind() {
    // Unwinding group variables
    let query = r#"
        MATCH ((a)-[r:KNOWS]->(b)){1,3}
        UNWIND b AS person
        RETURN a.name, person.name, person.age
    "#;
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse QPP group variable with UNWIND: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.match_clauses.len(), 1);
    assert_eq!(query_ast.unwind_clauses.len(), 1);
    assert_eq!(query_ast.return_clauses.len(), 1);
}

// Commented out: List comprehension with WHERE not yet fully implemented
// #[test]
// fn test_qpp_group_variable_filtering() {
//     // Filtering on group variables with list comprehension
//     let query = "MATCH ((a)-[r:KNOWS]->(b)){1,3} WHERE size([p IN b WHERE p.active = true]) > 0 RETURN a, b";
//     let result = parse_query(query);
//     assert!(result.is_ok(), "Failed to parse QPP with list comprehension filter on group variable: {:?}", result.err());
//
//     let query_ast = result.unwrap();
//     assert_eq!(query_ast.match_clauses.len(), 1);
//     assert_eq!(query_ast.where_clauses.len(), 1);
// }

// Commented out: Nested operations with all() and any() not yet fully implemented
// #[test]
// fn test_qpp_nested_group_variables() {
//     // Complex scenario with nested operations on group variables
//     let query = r#"
//         MATCH ((start)-[r:PATH]->(middle)-[r2:NEXT]->(end)){1,2}
//         WHERE all(m IN middle WHERE m.status = 'active')
//           AND any(e IN end WHERE e.priority > 5)
//         RETURN start.id,
//                [m IN middle | m.name] AS middle_names,
//                collect(end.name) AS end_names
//     "#;
//     let result = parse_query(query);
//     assert!(result.is_ok(), "Failed to parse QPP with nested group variable operations: {:?}", result.err());
//
//     let query_ast = result.unwrap();
//     assert_eq!(query_ast.match_clauses.len(), 1);
//     assert_eq!(query_ast.where_clauses.len(), 1);
//     assert_eq!(query_ast.return_clauses.len(), 1);
// }
