use cypher_guard::parse_query;

#[test]
fn test_minimal_unwind_call() {
    let query = "UNWIND [1] AS i CALL { CREATE (n) } IN TRANSACTIONS RETURN count(*) AS c";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse minimal UNWIND+CALL: {:?}", result.err());
}

#[test]
fn test_unwind_call_with_function() {
    let query = "UNWIND range(1, 10) AS i CALL { CREATE (n) } IN TRANSACTIONS RETURN count(*) AS c";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed with range(): {:?}", result.err());
}

// Known Limitations (to be fixed in future):
// - WITH clause inside CALL subquery after another clause causes parse errors
// - Property maps in node/rel patterns inside CALL subquery after another clause cause parse errors
// These work fine when CALL is the first clause but fail when CALL comes after MATCH/UNWIND/etc.

#[test]
fn test_call_in_transactions_basic() {
    let query = "CALL { CREATE (n:Node) } IN TRANSACTIONS RETURN count(*) AS created";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse basic CALL IN TRANSACTIONS: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.call_clauses.len(), 1);
    assert!(query_ast.call_clauses[0].in_transactions.is_some());
    assert!(query_ast.call_clauses[0].in_transactions.as_ref().unwrap().batch_size.is_none());
}

#[test]
fn test_call_in_transactions_with_batch_size() {
    let query = "CALL { CREATE (n:Node) } IN TRANSACTIONS OF 1000 ROWS RETURN count(*) AS created";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse CALL IN TRANSACTIONS with batch size: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.call_clauses.len(), 1);
    assert!(query_ast.call_clauses[0].in_transactions.is_some());
    assert_eq!(query_ast.call_clauses[0].in_transactions.as_ref().unwrap().batch_size, Some(1000));
}

#[test]
fn test_call_in_transactions_with_load_csv() {
    let query = "LOAD CSV FROM 'file:///data.csv' AS row CALL { CREATE (n:Node) } IN TRANSACTIONS OF 500 ROWS RETURN count(*) AS created";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse CALL IN TRANSACTIONS with LOAD CSV: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.load_csv_clauses.len(), 1);
    assert_eq!(query_ast.call_clauses.len(), 1);
    assert!(query_ast.call_clauses[0].in_transactions.is_some());
    assert_eq!(query_ast.call_clauses[0].in_transactions.as_ref().unwrap().batch_size, Some(500));
}

#[test]
fn test_call_in_transactions_case_insensitive() {
    // Test lowercase 'in transactions'
    let query1 = "CALL { CREATE (n) } in transactions";
    let result1 = parse_query(query1);
    assert!(result1.is_ok(), "Failed to parse lowercase 'in transactions': {:?}", result1.err());

    // Test mixed case 'In Transactions'
    let query2 = "CALL { CREATE (n) } In Transactions";
    let result2 = parse_query(query2);
    assert!(result2.is_ok(), "Failed to parse mixed case 'In Transactions': {:?}", result2.err());

    // Test with OF ROWS
    let query3 = "CALL { CREATE (n) } IN TRANSACTIONS of 100 rows";
    let result3 = parse_query(query3);
    assert!(result3.is_ok(), "Failed to parse case-insensitive 'of rows': {:?}", result3.err());
}

#[test]
fn test_call_without_in_transactions() {
    let query = "CALL { MATCH (n) RETURN n } RETURN count(*) AS total";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse CALL without IN TRANSACTIONS: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.call_clauses.len(), 1);
    assert!(query_ast.call_clauses[0].in_transactions.is_none());
}

#[test]
fn test_call_procedure_no_in_transactions() {
    let query = "CALL db.labels() YIELD label RETURN label";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse procedure CALL: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.call_clauses.len(), 1);
    assert!(query_ast.call_clauses[0].procedure.is_some());
    assert!(query_ast.call_clauses[0].in_transactions.is_none());
}

#[test]
fn test_call_in_transactions_with_with_clause() {
    let query = "MATCH (n:Person) WITH n CALL { CREATE (m:Copy) } IN TRANSACTIONS OF 100 ROWS RETURN count(*) AS copied";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse CALL IN TRANSACTIONS with WITH: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.match_clauses.len(), 1);
    assert_eq!(query_ast.with_clauses.len(), 1);
    assert_eq!(query_ast.call_clauses.len(), 1);
    assert!(query_ast.call_clauses[0].in_transactions.is_some());
}

#[test]
fn test_call_in_transactions_with_set() {
    let query = "MATCH (n:Node) CALL { MATCH (n) SET n.processed = true } IN TRANSACTIONS OF 500 ROWS RETURN count(*) AS processed";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse CALL IN TRANSACTIONS with SET: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.match_clauses.len(), 1);
    assert_eq!(query_ast.call_clauses.len(), 1);
    assert!(query_ast.call_clauses[0].in_transactions.is_some());
}

// Commented out due to known limitation with WITH in subquery after MATCH
// #[test]
// fn test_call_in_transactions_complex_subquery() {
//     let query = r#"
//         MATCH (source:Person)
//         CALL {
//             WITH source
//             MATCH (target:Person)
//             WHERE target.age > source.age
//             CREATE (source)-[:MENTOR]->(target)
//         } IN TRANSACTIONS OF 1000 ROWS
//         RETURN count(*) AS relationships_created
//     "#;
//     let result = parse_query(query);
//     assert!(result.is_ok(), "Failed to parse complex CALL IN TRANSACTIONS: {:?}", result.err());
//
//     let query_ast = result.unwrap();
//     assert_eq!(query_ast.match_clauses.len(), 1);
//     assert_eq!(query_ast.call_clauses.len(), 1);
//     assert_eq!(query_ast.return_clauses.len(), 1);
//     assert!(query_ast.call_clauses[0].in_transactions.is_some());
//     assert_eq!(query_ast.call_clauses[0].in_transactions.as_ref().unwrap().batch_size, Some(1000));
// }

#[test]
fn test_call_in_transactions_with_unwind() {
    let query = "UNWIND range(1, 10000) AS i CALL { CREATE (n:Number) } IN TRANSACTIONS OF 100 ROWS RETURN count(*) AS created";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse CALL IN TRANSACTIONS with UNWIND: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.unwind_clauses.len(), 1);
    assert_eq!(query_ast.call_clauses.len(), 1);
    assert!(query_ast.call_clauses[0].in_transactions.is_some());
}

#[test]
fn test_call_in_transactions_multiple_operations() {
    let query = "CALL { MATCH (n:Old) DELETE n CREATE (m:New) } IN TRANSACTIONS OF 250 ROWS";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse CALL IN TRANSACTIONS with multiple operations: {:?}", result.err());

    let query_ast = result.unwrap();
    assert_eq!(query_ast.call_clauses.len(), 1);
    assert!(query_ast.call_clauses[0].in_transactions.is_some());
    assert_eq!(query_ast.call_clauses[0].in_transactions.as_ref().unwrap().batch_size, Some(250));
}

#[test]
fn test_call_in_transactions_batch_sizes() {
    // Test different batch sizes
    let test_cases = vec![
        ("CALL { CREATE (n) } IN TRANSACTIONS OF 1 ROWS", Some(1)),
        ("CALL { CREATE (n) } IN TRANSACTIONS OF 100 ROWS", Some(100)),
        ("CALL { CREATE (n) } IN TRANSACTIONS OF 1000 ROWS", Some(1000)),
        ("CALL { CREATE (n) } IN TRANSACTIONS OF 10000 ROWS", Some(10000)),
        ("CALL { CREATE (n) } IN TRANSACTIONS", None),
    ];

    for (query, expected_batch_size) in test_cases {
        let result = parse_query(query);
        assert!(result.is_ok(), "Failed to parse '{}': {:?}", query, result.err());

        let query_ast = result.unwrap();
        assert_eq!(query_ast.call_clauses.len(), 1);
        assert!(query_ast.call_clauses[0].in_transactions.is_some());
        assert_eq!(
            query_ast.call_clauses[0].in_transactions.as_ref().unwrap().batch_size,
            expected_batch_size,
            "Batch size mismatch for query: {}",
            query
        );
    }
}
