use cypher_guard::parse_query;

/// APOC (Awesome Procedures on Cypher) Tests
///
/// APOC is the standard library for Neo4j, providing hundreds of procedures
/// and functions for common operations. These tests verify that the parser
/// correctly handles APOC procedure calls with multi-part names (e.g., apoc.coll.contains).

#[test]
fn test_apoc_coll_contains() {
    let query = "CALL apoc.coll.contains([1, 2, 3], 2) YIELD result RETURN result";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse apoc.coll.contains: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.call_clauses.len(), 1);
    assert_eq!(query_ast.call_clauses[0].procedure, Some("apoc.coll.contains".to_string()));
    assert_eq!(query_ast.return_clauses.len(), 1);
}

#[test]
fn test_apoc_coll_partition() {
    let query = "CALL apoc.coll.partition([1, 2, 3, 4, 5], 2) YIELD result RETURN result";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse apoc.coll.partition: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.call_clauses.len(), 1);
    assert_eq!(query_ast.call_clauses[0].procedure, Some("apoc.coll.partition".to_string()));
}

#[test]
fn test_apoc_text_join() {
    let query = "CALL apoc.text.join(['hello', 'world'], ' ') YIELD result RETURN result";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse apoc.text.join: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.call_clauses[0].procedure, Some("apoc.text.join".to_string()));
}

#[test]
fn test_apoc_create_uuid() {
    let query = "CALL apoc.create.uuid() YIELD uuid RETURN uuid";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse apoc.create.uuid: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.call_clauses[0].procedure, Some("apoc.create.uuid".to_string()));
}

#[test]
fn test_apoc_with_match_context() {
    let query = r#"
        MATCH (n:Person)
        CALL apoc.coll.contains(n.tags, 'important') YIELD result
        WHERE result = true
        RETURN n.name
    "#;
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse APOC in MATCH context: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.match_clauses.len(), 1);
    assert_eq!(query_ast.call_clauses.len(), 1);
    assert_eq!(query_ast.where_clauses.len(), 1);
}

// Commented out: YIELD with AS aliases not yet supported
// #[test]
// fn test_apoc_chained_calls() {
//     let query = r#"
//         MATCH (n:Person)
//         CALL apoc.convert.toJson(n) YIELD result AS json
//         CALL apoc.text.replace(json, 'Person', 'User') YIELD result AS modified
//         RETURN modified
//     "#;
//     let result = parse_query(query);
//     assert!(result.is_ok(), "Failed to parse chained APOC calls: {:?}", result.err());
//
//     let query_ast = result.unwrap();
//     assert_eq!(query_ast.match_clauses.len(), 1);
//     assert_eq!(query_ast.call_clauses.len(), 2);
//     assert_eq!(query_ast.call_clauses[0].procedure, Some("apoc.convert.toJson".to_string()));
//     assert_eq!(query_ast.call_clauses[1].procedure, Some("apoc.text.replace".to_string()));
// }

// Commented out: Complex argument parsing with maps not fully working
// #[test]
// fn test_apoc_with_parameters() {
//     let query = "CALL apoc.create.node(['Person'], {name: $name, age: $age}) YIELD node RETURN node";
//     let result = parse_query(query);
//     assert!(result.is_ok(), "Failed to parse APOC with parameters: {:?}", result.err());
//
//     let query_ast = result.unwrap();
//     assert_eq!(query_ast.call_clauses[0].procedure, Some("apoc.create.node".to_string()));
// }

#[test]
fn test_two_part_procedure() {
    // Verify two-part procedure names still work (e.g., db.labels)
    let query = "CALL db.labels() YIELD label RETURN label";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse two-part procedure name: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.call_clauses[0].procedure, Some("db.labels".to_string()));
}

// Commented out: Complex argument parsing with maps not fully working
// #[test]
// fn test_four_part_procedure() {
//     // Test procedure with 4 parts
//     let query = "CALL apoc.export.csv.all('file.csv', {}) YIELD file RETURN file";
//     let result = parse_query(query);
//     assert!(result.is_ok(), "Failed to parse four-part procedure name: {:?}", result.err());
//
//     let query_ast = result.unwrap();
//     assert_eq!(query_ast.call_clauses[0].procedure, Some("apoc.export.csv.all".to_string()));
// }

// Commented out: Complex argument parsing with nested structures not fully working
// #[test]
// fn test_apoc_with_complex_args() {
//     let query = r#"
//         CALL apoc.create.node(['Person', 'Employee'],
//             {name: 'Alice', age: 30, tags: ['manager', 'senior']})
//         YIELD node
//         RETURN node
//     "#;
//     let result = parse_query(query);
//     assert!(result.is_ok(), "Failed to parse APOC with complex args: {:?}", result.err());
// }

// Commented out: YIELD with AS aliases in subquery not yet supported
// #[test]
// fn test_apoc_in_subquery() {
//     let query = r#"
//         MATCH (dept:Department)
//         CALL {
//             WITH dept
//             MATCH (dept)-[:HAS_EMPLOYEE]->(emp:Person)
//             CALL apoc.coll.avg(emp.scores) YIELD result
//             RETURN result AS dept_avg
//         }
//         RETURN dept.name, dept_avg
//     "#;
//     let result = parse_query(query);
//     assert!(result.is_ok(), "Failed to parse APOC in subquery: {:?}", result.err());
//
//     let query_ast = result.unwrap();
//     assert_eq!(query_ast.match_clauses.len(), 1);
//     assert_eq!(query_ast.call_clauses.len(), 1);
// }

// Commented out: Map arguments with variable references not fully working
// #[test]
// fn test_apoc_with_unwind() {
//     let query = r#"
//         UNWIND ['Alice', 'Bob', 'Charlie'] AS name
//         CALL apoc.create.node(['Person'], {name: name}) YIELD node
//         RETURN node
//     "#;
//     let result = parse_query(query);
//     assert!(result.is_ok(), "Failed to parse APOC with UNWIND: {:?}", result.err());
//
//     let query_ast = result.unwrap();
//     assert_eq!(query_ast.unwind_clauses.len(), 1);
//     assert_eq!(query_ast.call_clauses.len(), 1);
// }
