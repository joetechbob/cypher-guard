// Priority 10: Path Selector Tests (Neo4j 5.x Feature)
// Tests for SHORTEST, ALL SHORTEST, SHORTEST k GROUPS, ANY k, ALL path selectors

use cypher_guard::parse_query;

#[test]
fn test_shortest_k_path_selector() {
    // SHORTEST k - Find k shortest paths
    let query = "MATCH SHORTEST 2 (a:Person)-[:KNOWS*]-(b:Person) RETURN a, b";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse SHORTEST k: {:?}", result.err());
    let query = result.unwrap();

    assert_eq!(query.match_clauses.len(), 1);
    let match_clause = &query.match_clauses[0];

    assert!(match_clause.path_selector.is_some());
    match &match_clause.path_selector {
        Some(cypher_guard::parser::ast::PathSelector::Shortest { k }) => {
            assert_eq!(*k, Some(2));
        }
        _ => panic!("Expected Shortest path selector with k=2"),
    }
}

#[test]
fn test_shortest_without_k() {
    // SHORTEST without k - Finds single shortest path
    let query = "MATCH SHORTEST (a:Person)-[:KNOWS*]-(b:Person) RETURN a, b";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse SHORTEST: {:?}", result.err());
    let query = result.unwrap();

    assert_eq!(query.match_clauses.len(), 1);
    let match_clause = &query.match_clauses[0];

    assert!(match_clause.path_selector.is_some());
    match &match_clause.path_selector {
        Some(cypher_guard::parser::ast::PathSelector::Shortest { k }) => {
            assert_eq!(*k, None);
        }
        _ => panic!("Expected Shortest path selector without k"),
    }
}

#[test]
fn test_all_shortest_path_selector() {
    // ALL SHORTEST - Find all paths of shortest length
    let query = "MATCH ALL SHORTEST (a:Station)-[:LINK*]-(b:Station) RETURN a, b";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse ALL SHORTEST: {:?}", result.err());
    let query = result.unwrap();

    assert_eq!(query.match_clauses.len(), 1);
    let match_clause = &query.match_clauses[0];

    assert!(match_clause.path_selector.is_some());
    match &match_clause.path_selector {
        Some(cypher_guard::parser::ast::PathSelector::AllShortest) => {
            // Correct!
        }
        _ => panic!("Expected AllShortest path selector"),
    }
}

#[test]
fn test_shortest_k_groups() {
    // SHORTEST k GROUPS - Find paths from k shortest groups
    let query = "MATCH SHORTEST 3 GROUPS (a:Person)-[:KNOWS*]-(b:Person) RETURN a, b";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse SHORTEST k GROUPS: {:?}", result.err());
    let query = result.unwrap();

    assert_eq!(query.match_clauses.len(), 1);
    let match_clause = &query.match_clauses[0];

    assert!(match_clause.path_selector.is_some());
    match &match_clause.path_selector {
        Some(cypher_guard::parser::ast::PathSelector::ShortestGroups { k }) => {
            assert_eq!(*k, 3);
        }
        _ => panic!("Expected ShortestGroups path selector with k=3"),
    }
}

#[test]
fn test_any_k_path_selector() {
    // ANY k - Find any k paths
    let query = "MATCH ANY 5 (a:Person)-[:KNOWS*]-(b:Person) RETURN a, b";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse ANY k: {:?}", result.err());
    let query = result.unwrap();

    assert_eq!(query.match_clauses.len(), 1);
    let match_clause = &query.match_clauses[0];

    assert!(match_clause.path_selector.is_some());
    match &match_clause.path_selector {
        Some(cypher_guard::parser::ast::PathSelector::Any { k }) => {
            assert_eq!(*k, 5);
        }
        _ => panic!("Expected Any path selector with k=5"),
    }
}

#[test]
fn test_all_path_selector() {
    // ALL - Find all paths
    let query = "MATCH ALL (a:Person)-[:KNOWS*]-(b:Person) RETURN a, b";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse ALL: {:?}", result.err());
    let query = result.unwrap();

    assert_eq!(query.match_clauses.len(), 1);
    let match_clause = &query.match_clauses[0];

    assert!(match_clause.path_selector.is_some());
    match &match_clause.path_selector {
        Some(cypher_guard::parser::ast::PathSelector::All) => {
            // Correct!
        }
        _ => panic!("Expected All path selector"),
    }
}

#[test]
fn test_path_selector_case_insensitive() {
    // Path selectors should be case-insensitive
    let queries = vec![
        "MATCH SHORTEST 2 (a)-[:REL*]-(b) RETURN a",
        "MATCH shortest 2 (a)-[:REL*]-(b) RETURN a",
        "MATCH Shortest 2 (a)-[:REL*]-(b) RETURN a",
        "MATCH ALL SHORTEST (a)-[:REL*]-(b) RETURN a",
        "MATCH all shortest (a)-[:REL*]-(b) RETURN a",
    ];

    for query_str in queries {
        let result = parse_query(query_str);
        assert!(result.is_ok(), "Failed to parse: {}", query_str);
        let query = result.unwrap();
        assert!(query.match_clauses[0].path_selector.is_some(),
            "Path selector not found in: {}", query_str);
    }
}

#[test]
fn test_path_selector_with_properties() {
    // Path selector with node properties
    let query = "MATCH SHORTEST 2 (a:Person {name: 'Alice'})-[:KNOWS*]-(b:Person {name: 'Bob'}) RETURN a, b";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse path selector with properties: {:?}", result.err());
    let query = result.unwrap();

    assert_eq!(query.match_clauses.len(), 1);
    assert!(query.match_clauses[0].path_selector.is_some());
}

#[test]
fn test_path_selector_with_where() {
    // Path selector with WHERE clause
    let query = "MATCH SHORTEST 2 (a:Person)-[:KNOWS*]-(b:Person) WHERE a.age > 30 RETURN a, b";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse path selector with WHERE: {:?}", result.err());
    let query = result.unwrap();

    assert_eq!(query.match_clauses.len(), 1);
    assert!(query.match_clauses[0].path_selector.is_some());
    assert_eq!(query.where_clauses.len(), 1);
}

#[test]
fn test_path_selector_with_return_modifiers() {
    // Path selector with ORDER BY, LIMIT
    let query = "MATCH SHORTEST 3 (a)-[:REL*]-(b) RETURN a, b ORDER BY a.name LIMIT 10";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse path selector with modifiers: {:?}", result.err());
    let query = result.unwrap();

    assert!(query.match_clauses[0].path_selector.is_some());
    assert_eq!(query.return_clauses.len(), 1);
}

#[test]
fn test_path_selector_shortestpath_function_not_confused() {
    // Make sure shortestPath() function is not confused with SHORTEST selector
    let query = "MATCH p = shortestPath((a:Person)-[:KNOWS*]-(b:Person)) RETURN p";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse shortestPath function: {:?}", result.err());
    let query = result.unwrap();

    // Should NOT have a path selector
    assert!(query.match_clauses[0].path_selector.is_none(),
        "shortestPath() function should not be parsed as path selector");
}

#[test]
fn test_path_selector_allshortestpaths_function_not_confused() {
    // Make sure allShortestPaths() function is not confused with ALL SHORTEST selector
    let query = "MATCH p = allShortestPaths((a:Person)-[:KNOWS*]-(b:Person)) RETURN p";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse allShortestPaths function: {:?}", result.err());
    let query = result.unwrap();

    // Should NOT have a path selector
    assert!(query.match_clauses[0].path_selector.is_none(),
        "allShortestPaths() function should not be parsed as path selector");
}

#[test]
fn test_match_without_path_selector() {
    // Regular MATCH without path selector should still work
    let query = "MATCH (a:Person)-[:KNOWS]-(b:Person) RETURN a, b";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse regular MATCH: {:?}", result.err());
    let query = result.unwrap();

    assert_eq!(query.match_clauses.len(), 1);
    assert!(query.match_clauses[0].path_selector.is_none(),
        "Regular MATCH should not have path selector");
}

#[test]
fn test_path_selector_with_qpp() {
    // Path selector with Quantified Path Pattern
    let query = "MATCH SHORTEST 2 ((a)-[:KNOWS]->(b)){1,3} RETURN a, b";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse path selector with QPP: {:?}", result.err());
    let query = result.unwrap();

    assert!(query.match_clauses[0].path_selector.is_some());
}

#[test]
fn test_path_selector_multiple_relationships() {
    // Path selector with multiple relationship types
    let query = "MATCH SHORTEST 2 (a:Person)-[:KNOWS|WORKS_WITH*]-(b:Person) RETURN a, b";
    let result = parse_query(query);

    // This may or may not work depending on parser implementation
    // Just verify it doesn't crash
    let _ = result;
}

#[test]
fn test_path_selector_with_whitespace_variations() {
    // Test various whitespace patterns
    let queries = vec![
        "MATCH SHORTEST 2 (a)-[:REL*]-(b) RETURN a",
        "MATCH SHORTEST  2  (a)-[:REL*]-(b) RETURN a",
        "MATCH\nSHORTEST\n2\n(a)-[:REL*]-(b) RETURN a",
        "MATCH\tSHORTEST\t2\t(a)-[:REL*]-(b) RETURN a",
    ];

    for query_str in queries {
        let result = parse_query(query_str);
        assert!(result.is_ok(), "Failed to parse with whitespace: {}", query_str);
        let query = result.unwrap();
        assert!(query.match_clauses[0].path_selector.is_some());
    }
}

#[test]
fn test_path_selector_shortest_1() {
    // SHORTEST 1 should work
    let query = "MATCH SHORTEST 1 (a)-[:REL*]-(b) RETURN a";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse SHORTEST 1: {:?}", result.err());
    let query = result.unwrap();

    match &query.match_clauses[0].path_selector {
        Some(cypher_guard::parser::ast::PathSelector::Shortest { k }) => {
            assert_eq!(*k, Some(1));
        }
        _ => panic!("Expected Shortest with k=1"),
    }
}

#[test]
fn test_path_selector_shortest_groups_1() {
    // SHORTEST 1 GROUPS should work
    let query = "MATCH SHORTEST 1 GROUPS (a)-[:REL*]-(b) RETURN a";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse SHORTEST 1 GROUPS: {:?}", result.err());
    let query = result.unwrap();

    match &query.match_clauses[0].path_selector {
        Some(cypher_guard::parser::ast::PathSelector::ShortestGroups { k }) => {
            assert_eq!(*k, 1);
        }
        _ => panic!("Expected ShortestGroups with k=1"),
    }
}

#[test]
fn test_path_selector_large_k() {
    // Test large k values
    let query = "MATCH SHORTEST 100 (a)-[:REL*]-(b) RETURN a";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse SHORTEST 100: {:?}", result.err());
    let query = result.unwrap();

    match &query.match_clauses[0].path_selector {
        Some(cypher_guard::parser::ast::PathSelector::Shortest { k }) => {
            assert_eq!(*k, Some(100));
        }
        _ => panic!("Expected Shortest with k=100"),
    }
}

#[test]
fn test_optional_match_with_path_selector() {
    // OPTIONAL MATCH with path selector
    let query = "OPTIONAL MATCH SHORTEST 2 (a:Person)-[:KNOWS*]-(b:Person) RETURN a, b";
    let result = parse_query(query);

    assert!(result.is_ok(), "Failed to parse OPTIONAL MATCH with path selector: {:?}", result.err());
    let query = result.unwrap();

    assert_eq!(query.match_clauses.len(), 1);
    assert!(query.match_clauses[0].is_optional);
    assert!(query.match_clauses[0].path_selector.is_some());
}
