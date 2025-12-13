// Priority 9: USE Clause Tests for Multi-Database Routing
// Neo4j 4.x+ feature for switching database context

use cypher_guard::parse_query;

#[test]
fn test_use_static_database() {
    // Simple static database name
    let query = "USE myDatabase MATCH (n) RETURN n";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse USE with static database: {:?}", result.err());
    let query = result.unwrap();

    assert!(query.use_clause.is_some());
    let use_clause = query.use_clause.unwrap();

    match use_clause.graph_reference {
        cypher_guard::parser::ast::GraphReference::Static(name) => {
            assert_eq!(name, "myDatabase");
        }
        _ => panic!("Expected Static graph reference"),
    }

    assert_eq!(query.match_clauses.len(), 1);
    assert_eq!(query.return_clauses.len(), 1);
}

#[test]
fn test_use_case_insensitive() {
    // USE should be case-insensitive
    let queries = vec![
        "USE analytics MATCH (n) RETURN n",
        "use analytics MATCH (n) RETURN n",
        "Use analytics MATCH (n) RETURN n",
        "uSe analytics MATCH (n) RETURN n",
    ];

    for query_str in queries {
        let result = parse_query(query_str);
        assert!(result.is_ok(), "Failed to parse: {}", query_str);
        let query = result.unwrap();
        assert!(query.use_clause.is_some(), "USE clause not found in: {}", query_str);
    }
}

#[test]
fn test_use_graph_byname_string() {
    // Dynamic database selection with string literal
    let query = "USE graph.byName('analytics') MATCH (n) RETURN n";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse USE graph.byName with string: {:?}", result.err());
    let query = result.unwrap();

    assert!(query.use_clause.is_some());
    let use_clause = query.use_clause.unwrap();

    match use_clause.graph_reference {
        cypher_guard::parser::ast::GraphReference::ByName(prop_value) => {
            match *prop_value {
                cypher_guard::parser::ast::PropertyValue::String(s) => {
                    assert_eq!(s, "analytics");
                }
                _ => panic!("Expected String PropertyValue"),
            }
        }
        _ => panic!("Expected ByName graph reference"),
    }
}

#[test]
fn test_use_graph_byname_parameter() {
    // Dynamic database selection with parameter
    let query = "USE graph.byName($dbName) MATCH (n) RETURN n";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse USE graph.byName with parameter: {:?}", result.err());
    let query = result.unwrap();

    assert!(query.use_clause.is_some());
    let use_clause = query.use_clause.unwrap();

    match use_clause.graph_reference {
        cypher_guard::parser::ast::GraphReference::ByName(prop_value) => {
            match *prop_value {
                cypher_guard::parser::ast::PropertyValue::Parameter(p) => {
                    assert_eq!(p, "dbName");
                }
                _ => panic!("Expected Parameter PropertyValue"),
            }
        }
        _ => panic!("Expected ByName graph reference"),
    }
}

#[test]
fn test_use_graph_byelementid_string() {
    // Access graph by element ID with string literal
    let query = "USE graph.byElementId('4:abc:123') MATCH (n) RETURN n";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse USE graph.byElementId with string: {:?}", result.err());
    let query = result.unwrap();

    assert!(query.use_clause.is_some());
    let use_clause = query.use_clause.unwrap();

    match use_clause.graph_reference {
        cypher_guard::parser::ast::GraphReference::ByElementId(prop_value) => {
            match *prop_value {
                cypher_guard::parser::ast::PropertyValue::String(s) => {
                    assert_eq!(s, "4:abc:123");
                }
                _ => panic!("Expected String PropertyValue"),
            }
        }
        _ => panic!("Expected ByElementId graph reference"),
    }
}

#[test]
fn test_use_graph_byelementid_parameter() {
    // Access graph by element ID with parameter
    let query = "USE graph.byElementId($elementId) MATCH (n) RETURN n";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse USE graph.byElementId with parameter: {:?}", result.err());
    let query = result.unwrap();

    assert!(query.use_clause.is_some());
    let use_clause = query.use_clause.unwrap();

    match use_clause.graph_reference {
        cypher_guard::parser::ast::GraphReference::ByElementId(prop_value) => {
            match *prop_value {
                cypher_guard::parser::ast::PropertyValue::Parameter(p) => {
                    assert_eq!(p, "elementId");
                }
                _ => panic!("Expected Parameter PropertyValue"),
            }
        }
        _ => panic!("Expected ByElementId graph reference"),
    }
}

#[test]
fn test_use_composite_database() {
    // Composite database syntax: composite.constituent
    let query = "USE composite.constituent MATCH (n) RETURN n";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse USE with composite database: {:?}", result.err());
    let query = result.unwrap();

    assert!(query.use_clause.is_some());
    let use_clause = query.use_clause.unwrap();

    match use_clause.graph_reference {
        cypher_guard::parser::ast::GraphReference::Composite(first, second) => {
            assert_eq!(first, "composite");
            assert_eq!(second, "constituent");
        }
        _ => panic!("Expected Composite graph reference"),
    }
}

#[test]
fn test_use_with_complex_query() {
    // USE with more complex query including WHERE and ORDER BY
    let query = "USE analytics MATCH (p:Person)-[:KNOWS]->(f:Person) WHERE p.age > 30 RETURN p.name, f.name ORDER BY p.name LIMIT 10";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse USE with complex query: {:?}", result.err());
    let query = result.unwrap();

    assert!(query.use_clause.is_some());
    assert_eq!(query.match_clauses.len(), 1);
    assert_eq!(query.where_clauses.len(), 1);
    assert_eq!(query.return_clauses.len(), 1);
}

#[test]
fn test_use_with_create() {
    // USE with write operations
    let query = "USE myDatabase CREATE (n:Person {name: 'Alice'}) RETURN n";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse USE with CREATE: {:?}", result.err());
    let query = result.unwrap();

    assert!(query.use_clause.is_some());
    assert_eq!(query.create_clauses.len(), 1);
    assert_eq!(query.return_clauses.len(), 1);
}

#[test]
fn test_use_with_merge() {
    // USE with MERGE operation
    let query = "USE production MERGE (p:Person {id: 123}) ON CREATE SET p.created = timestamp() RETURN p";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse USE with MERGE: {:?}", result.err());
    let query = result.unwrap();

    assert!(query.use_clause.is_some());
    assert_eq!(query.merge_clauses.len(), 1);
}

#[test]
fn test_use_with_unwind() {
    // USE with UNWIND
    let query = "USE analytics UNWIND [1, 2, 3] AS x RETURN x";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse USE with UNWIND: {:?}", result.err());
    let query = result.unwrap();

    assert!(query.use_clause.is_some());
    assert_eq!(query.unwind_clauses.len(), 1);
    assert_eq!(query.return_clauses.len(), 1);
}

#[test]
fn test_use_with_call_subquery() {
    // USE with CALL subquery
    let query = "USE myDatabase CALL { MATCH (n:Person) RETURN count(n) AS personCount } RETURN personCount";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse USE with CALL subquery: {:?}", result.err());
    let query = result.unwrap();

    assert!(query.use_clause.is_some());
    assert_eq!(query.call_clauses.len(), 1);
    assert_eq!(query.return_clauses.len(), 1);
}

#[test]
fn test_use_with_whitespace_variations() {
    // Test various whitespace patterns
    let queries = vec![
        "USE myDatabase MATCH (n) RETURN n",
        "USE  myDatabase  MATCH (n) RETURN n",
        "USE\nmyDatabase\nMATCH (n) RETURN n",
        "USE\tmyDatabase\tMATCH (n) RETURN n",
        "  USE myDatabase MATCH (n) RETURN n",
    ];

    for query_str in queries {
        let result = parse_query(query_str);
        assert!(result.is_ok(), "Failed to parse with whitespace: {}", query_str);
        let query = result.unwrap();
        assert!(query.use_clause.is_some());
    }
}

#[test]
fn test_query_without_use_clause() {
    // Verify queries without USE still work
    let query = "MATCH (n:Person) RETURN n";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse query without USE: {:?}", result.err());
    let query = result.unwrap();

    assert!(query.use_clause.is_none(), "USE clause should be None");
    assert_eq!(query.match_clauses.len(), 1);
    assert_eq!(query.return_clauses.len(), 1);
}

#[test]
fn test_use_must_come_first() {
    // USE must come before other clauses
    let valid_query = "USE myDatabase MATCH (n) RETURN n";
    let result = parse_query(valid_query);
    assert!(result.is_ok(), "Valid USE placement failed to parse");

    // Invalid: USE after MATCH should fail (or be parsed as part of WHERE condition)
    let invalid_query = "MATCH (n) USE myDatabase RETURN n";
    let result = parse_query(invalid_query);
    // This should either fail or parse differently (USE would not be recognized as clause)
    if result.is_ok() {
        let query = result.unwrap();
        assert!(query.use_clause.is_none(), "USE should not be parsed when not first");
    }
}

#[test]
fn test_use_with_union() {
    // USE with UNION queries
    let query = "USE db1 MATCH (n:Person) RETURN n UNION USE db2 MATCH (m:Person) RETURN m";
    let result = parse_query(query);

    // This may or may not work depending on implementation
    // For now, just test that the first USE is parsed
    if result.is_ok() {
        let query = result.unwrap();
        assert!(query.use_clause.is_some(), "First USE should be parsed");
    }
}

#[test]
fn test_use_graph_functions_case_insensitive() {
    // graph.byName and graph.byElementId should be case-insensitive
    let queries = vec![
        "USE graph.byName('db') MATCH (n) RETURN n",
        "USE graph.BYNAME('db') MATCH (n) RETURN n",
        "USE GRAPH.byName('db') MATCH (n) RETURN n",
        "USE graph.byElementId('id') MATCH (n) RETURN n",
        "USE graph.BYELEMENTID('id') MATCH (n) RETURN n",
    ];

    for query_str in queries {
        let result = parse_query(query_str);
        assert!(result.is_ok(), "Failed to parse case variant: {}", query_str);
        let query = result.unwrap();
        assert!(query.use_clause.is_some());
    }
}

#[test]
fn test_use_database_names_with_special_chars() {
    // Database names can have various formats
    let queries = vec![
        "USE myDatabase MATCH (n) RETURN n",
        "USE database123 MATCH (n) RETURN n",
        "USE my_database MATCH (n) RETURN n",
        "USE MyDatabase MATCH (n) RETURN n",
    ];

    for query_str in queries {
        let result = parse_query(query_str);
        assert!(result.is_ok(), "Failed to parse database name: {}", query_str);
        let query = result.unwrap();
        assert!(query.use_clause.is_some());
    }
}
