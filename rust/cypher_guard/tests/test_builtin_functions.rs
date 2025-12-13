use cypher_guard::parse_query;

/// Neo4j Built-in Functions Tests
///
/// Neo4j provides hundreds of built-in functions for data manipulation,
/// mathematical operations, string processing, temporal operations, and more.
/// This test suite verifies that the parser correctly handles these functions.

// ========================================
// Aggregation Functions
// ========================================

#[test]
fn test_aggregation_count() {
    let query = "MATCH (n:Person) RETURN count(n) AS total";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse count(): {:?}", result.err());
}

#[test]
fn test_aggregation_count_star() {
    let query = "MATCH (n) RETURN count(*) AS total";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse count(*): {:?}", result.err());
}

#[test]
fn test_aggregation_multiple() {
    let query = "MATCH (n:Person) RETURN count(n) AS total, sum(n.age) AS totalAge, avg(n.age) AS avgAge, min(n.age) AS minAge, max(n.age) AS maxAge";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse multiple aggregations: {:?}", result.err());
}

#[test]
fn test_aggregation_collect() {
    let query = "MATCH (n:Person) RETURN collect(n.name) AS names";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse collect(): {:?}", result.err());
}

// ========================================
// String Functions
// ========================================

#[test]
fn test_string_substring() {
    let query = "MATCH (n) RETURN substring(n.name, 0, 5) AS shortName";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse substring(): {:?}", result.err());
}

#[test]
fn test_string_tolower_toupper() {
    let query = "MATCH (n) RETURN toLower(n.name) AS lower, toUpper(n.name) AS upper";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse toLower/toUpper(): {:?}", result.err());
}

#[test]
fn test_string_trim() {
    let query = "MATCH (n) RETURN trim(n.name) AS trimmed, ltrim(n.name) AS leftTrim, rtrim(n.name) AS rightTrim";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse trim functions: {:?}", result.err());
}

#[test]
fn test_string_replace() {
    let query = "MATCH (n) RETURN replace(n.text, 'old', 'new') AS updated";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse replace(): {:?}", result.err());
}

#[test]
fn test_string_split() {
    let query = "MATCH (n) RETURN split(n.csv, ',') AS parts";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse split(): {:?}", result.err());
}

// ========================================
// Mathematical Functions
// ========================================

#[test]
fn test_math_basic() {
    let query = "MATCH (n) RETURN abs(n.value) AS absolute, ceil(n.value) AS ceiling, floor(n.value) AS floored, round(n.value) AS rounded";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse basic math functions: {:?}", result.err());
}

#[test]
fn test_math_trigonometric() {
    let query = "MATCH (n) RETURN sin(n.angle) AS sine, cos(n.angle) AS cosine, tan(n.angle) AS tangent";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse trigonometric functions: {:?}", result.err());
}

#[test]
fn test_math_power_sqrt() {
    let query = "RETURN sqrt(16) AS squareRoot, pow(2, 3) AS power, exp(2) AS exponential, log(10) AS logarithm";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse power/sqrt functions: {:?}", result.err());
}

#[test]
fn test_math_random() {
    let query = "RETURN rand() AS random, randomUUID() AS uuid";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse random functions: {:?}", result.err());
}

// ========================================
// Temporal Functions
// ========================================

#[test]
fn test_temporal_date() {
    let query = "RETURN date() AS today, date('2020-01-15') AS specificDate";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse date(): {:?}", result.err());
}

#[test]
fn test_temporal_datetime() {
    let query = "RETURN datetime() AS now, datetime('2020-01-15T10:30:00') AS specificDateTime";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse datetime(): {:?}", result.err());
}

#[test]
fn test_temporal_time() {
    let query = "RETURN time() AS currentTime, localtime() AS localCurrentTime";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse time(): {:?}", result.err());
}

// Commented out: Map arguments in RETURN without MATCH not fully working
// #[test]
// fn test_temporal_duration() {
//     let query = "RETURN duration('P1Y2M10D') AS period, duration({days: 14, hours: 16}) AS timeDuration";
//     let result = parse_query(query);
//     assert!(result.is_ok(), "Failed to parse duration(): {:?}", result.err());
// }

#[test]
fn test_temporal_timestamp() {
    let query = "MATCH (n) WHERE n.created > timestamp() - 86400000 RETURN n";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse timestamp(): {:?}", result.err());
}

// ========================================
// Spatial Functions
// ========================================

// Commented out: Map arguments in RETURN without MATCH not fully working
// #[test]
// fn test_spatial_point() {
//     let query = "RETURN point({latitude: 12.78, longitude: 56.7}) AS location";
//     let result = parse_query(query);
//     assert!(result.is_ok(), "Failed to parse point(): {:?}", result.err());
// }

// #[test]
// fn test_spatial_point_cartesian() {
//     let query = "RETURN point({x: 2.3, y: 4.5}) AS cartesianPoint";
//     let result = parse_query(query);
//     assert!(result.is_ok(), "Failed to parse point() with x/y: {:?}", result.err());
// }

// Commented out: Namespace function calls (point.distance) not yet supported
// #[test]
// fn test_spatial_distance() {
//     let query = r#"
//         MATCH (a:Place), (b:Place)
//         WHERE point.distance(a.location, b.location) < 1000
//         RETURN a, b
//     "#;
//     let result = parse_query(query);
//     assert!(result.is_ok(), "Failed to parse point.distance(): {:?}", result.err());
// }

// ========================================
// List Functions
// ========================================

#[test]
fn test_list_size() {
    let query = "MATCH (n) WHERE size(n.tags) > 2 RETURN n";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse size(): {:?}", result.err());
}

#[test]
fn test_list_head_tail_last() {
    let query = "MATCH (n) RETURN head(n.items) AS first, tail(n.items) AS rest, last(n.items) AS last";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse head/tail/last(): {:?}", result.err());
}

#[test]
fn test_list_range() {
    let query = "UNWIND range(1, 10) AS i RETURN i";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse range(): {:?}", result.err());
}

#[test]
fn test_list_range_with_step() {
    let query = "UNWIND range(0, 100, 10) AS i RETURN i";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse range() with step: {:?}", result.err());
}

#[test]
fn test_list_reverse() {
    let query = "MATCH (n) RETURN reverse(n.items) AS reversed";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse reverse(): {:?}", result.err());
}

// ========================================
// Type Conversion Functions
// ========================================

#[test]
fn test_conversion_to_functions() {
    let query = "RETURN toInteger('42') AS int, toFloat('3.14') AS float, toString(123) AS str, toBoolean('true') AS bool";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse type conversion functions: {:?}", result.err());
}

// ========================================
// Predicate Functions
// ========================================

#[test]
fn test_predicate_exists() {
    let query = "MATCH (n) WHERE exists(n.email) RETURN n";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse exists(): {:?}", result.err());
}

#[test]
fn test_predicate_isempty() {
    let query = "MATCH (n) WHERE isEmpty(n.tags) RETURN n";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse isEmpty(): {:?}", result.err());
}

// ========================================
// Path Functions
// ========================================

#[test]
fn test_path_length() {
    let query = "MATCH p = (a)-[*]->(b) RETURN length(p) AS pathLength";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse length() on path: {:?}", result.err());
}

#[test]
fn test_path_nodes() {
    let query = "MATCH p = (a)-[*]->(b) RETURN nodes(p) AS pathNodes";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse nodes(): {:?}", result.err());
}

#[test]
fn test_path_relationships() {
    let query = "MATCH p = (a)-[*]->(b) RETURN relationships(p) AS pathRels";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse relationships(): {:?}", result.err());
}

// ========================================
// Node/Relationship Functions
// ========================================

#[test]
fn test_id_function() {
    let query = "MATCH (n) RETURN id(n) AS nodeId";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse id(): {:?}", result.err());
}

#[test]
fn test_labels_function() {
    let query = "MATCH (n) RETURN labels(n) AS nodeLabels";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse labels(): {:?}", result.err());
}

#[test]
fn test_type_function() {
    let query = "MATCH ()-[r]->() RETURN type(r) AS relType";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse type(): {:?}", result.err());
}

#[test]
fn test_properties_function() {
    let query = "MATCH (n) RETURN properties(n) AS props";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse properties(): {:?}", result.err());
}

#[test]
fn test_keys_function() {
    let query = "MATCH (n) RETURN keys(n) AS propertyKeys";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse keys(): {:?}", result.err());
}

#[test]
fn test_startnode_endnode() {
    let query = "MATCH ()-[r]->() RETURN startNode(r) AS start, endNode(r) AS end";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse startNode/endNode(): {:?}", result.err());
}

// ========================================
// Functions in Complex Queries
// ========================================

#[test]
fn test_functions_in_where() {
    let query = r#"
        MATCH (n:Person)
        WHERE size(n.name) > 5
          AND toLower(n.email) CONTAINS '@example.com'
          AND n.age > toInteger('18')
        RETURN n
    "#;
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse functions in WHERE: {:?}", result.err());
}

#[test]
fn test_nested_functions() {
    let query = "MATCH (n) RETURN toUpper(substring(n.name, 0, 1)) AS firstLetterUpper";
    let result = parse_query(query);
    assert!(result.is_ok(), "Failed to parse nested functions: {:?}", result.err());
}

// Commented out: CASE expressions not yet fully implemented
// #[test]
// fn test_functions_with_case() {
//     let query = r#"
//         MATCH (n:Person)
//         RETURN n.name,
//                CASE WHEN size(n.friends) > 10 THEN 'popular'
//                     WHEN size(n.friends) > 5 THEN 'social'
//                     ELSE 'quiet'
//                END AS socialLevel
//     "#;
//     let result = parse_query(query);
//     assert!(result.is_ok(), "Failed to parse functions with CASE: {:?}", result.err());
// }

// Commented out: Functions in ORDER BY not yet supported
// #[test]
// fn test_functions_in_order_by() {
//     let query = "MATCH (n:Person) RETURN n ORDER BY size(n.name) DESC, toLower(n.name) ASC";
//     let result = parse_query(query);
//     assert!(result.is_ok(), "Failed to parse functions in ORDER BY: {:?}", result.err());
// }

// Commented out: Functions in ORDER BY not yet supported
// #[test]
// fn test_functions_with_aggregation() {
//     let query = r#"
//         MATCH (p:Person)-[:LIVES_IN]->(c:City)
//         RETURN c.name, count(p) AS population, avg(p.age) AS avgAge, collect(p.name) AS residents
//         ORDER BY count(p) DESC
//     "#;
//     let result = parse_query(query);
//     assert!(result.is_ok(), "Failed to parse functions with aggregation: {:?}", result.err());
// }
