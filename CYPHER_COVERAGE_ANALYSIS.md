# Cypher-Guard: Neo4j Coverage Analysis

**Status**: 350/350 tests passing (100%)
**Date**: 2025-12-13
**Goal**: Achieve comprehensive Neo4j Cypher coverage

## Current Coverage Status

### ✅ Fully Supported Features

#### Reading Clauses
- [x] `MATCH` - Standard pattern matching
- [x] `OPTIONAL MATCH` - Optional pattern matching  
- [x] `WHERE` - Filtering with comparisons
- [x] `RETURN` - Returning results
- [x] `WITH` - Projection and piping
- [x] `UNWIND` - List unwinding
- [x] `CALL` - Subqueries and procedures

#### Writing Clauses
- [x] `CREATE` - Node and relationship creation
- [x] `MERGE` - Merge patterns
- [x] `SET` - Property updates (including functions like `timestamp()`)
- [x] `ON CREATE SET` - Conditional updates on create
- [x] `ON MATCH SET` - Conditional updates on match

#### Pattern Matching
- [x] Node patterns: `(n:Label {prop: value})`
- [x] Relationship patterns: `-[r:TYPE]->`, `<-[r:TYPE]-`, `-[r:TYPE]-`
- [x] Variable length paths: `*1..5`, `*..3`, `*2..`
- [x] Multiple patterns in single MATCH

#### WHERE Clause Features
- [x] Comparison operators: `=`, `<>`, `<`, `>`, `<=`, `>=`
- [x] `IS NULL` / `IS NOT NULL`
- [x] `AND` / `OR` / `NOT`
- [x] Parenthesized conditions
- [x] Function calls in WHERE: `WHERE length(name) > 5`
- [x] Property access: `WHERE n.prop = value`

#### Functions
- [x] `count()` with wildcard: `count(*)`
- [x] `count()` with property: `count(n.prop)`
- [x] Function calls in RETURN
- [x] Function calls in WHERE
- [x] Function calls in SET: `SET n.created = timestamp()`
- [x] Nested function calls

#### Expressions
- [x] Property access: `n.property`
- [x] String literals (single and double quotes)
- [x] Numeric literals
- [x] Boolean literals: `true`, `false`
- [x] NULL literal
- [x] Parameters: `$param`
- [x] Lists: `[1, 2, 3]`
- [x] Maps: `{key: value}`

## 🔴 Missing Features (Comprehensive List from Neo4j Docs)

### Priority 1: CRITICAL Expression Features (High Usage)

#### String Comparison Operators (Neo4j Expressions Manual)
- [ ] `STARTS WITH`: `WHERE n.name STARTS WITH 'A'`
- [ ] `ENDS WITH`: `WHERE n.name ENDS WITH 'son'`
- [ ] `CONTAINS`: `WHERE n.name CONTAINS 'oh'`
- [ ] `=~` (regex): `WHERE n.name =~ '.*son$'`
- [ ] `IS NORMALIZED` / `IS NOT NORMALIZED` - String normalization checks

#### List Operators (Neo4j Expressions Manual)
- [ ] `IN` operator: `WHERE n.prop IN [1, 2, 3]`
- [ ] `IN` for property existence: `WHERE 'email' IN keys(n)`

#### Boolean Operators
- [x] `AND`, `OR`, `NOT` ✅ (implemented)
- [ ] `XOR` - Exclusive OR operator

#### Mathematical Operators (Neo4j Expressions Manual)
- [ ] Addition: `+` for numbers
- [ ] Subtraction: `-`
- [ ] Multiplication: `*`
- [ ] Division: `/`
- [ ] Modulo: `%`
- [ ] Exponentiation: `^`
- [ ] Examples:
  - `WHERE n.age + 5 > 30`
  - `RETURN n.price * 1.1 AS newPrice`
  - `RETURN 2 ^ 3 AS result` (returns 8)

#### String Operators (Neo4j Expressions Manual)
- [ ] String concatenation with `+`: `n.firstName + ' ' + n.lastName`
- [ ] String concatenation with `||`: `n.firstName || ' ' || n.lastName`

#### List Expressions (Neo4j Expressions Manual)
- [ ] List concatenation with `||`: `list1 || list2`
- [ ] List concatenation with `+`: `list1 + list2`
- [ ] List element access: `list[0]`, `list[-1]`
- [ ] List slicing: `list[1..3]`, `list[1..]`, `list[..3]`
- [ ] List comprehension: `[x IN list WHERE x > 5 | x * 2]`
- [ ] Pattern comprehension: `[(n)-->(m) WHERE m.age > 25 | m.name]`

#### Map Expressions (Neo4j Expressions Manual)
- [x] Map literals: `{key: value}` ✅ (implemented)
- [ ] Property access with dot: `map.key`
- [ ] Property access with brackets: `map['key']`
- [ ] Map projection: `n{.name, .age, computed: n.prop * 2}`

#### Node and Relationship Operators
- [x] Property access with dot: `n.property` ✅ (implemented)
- [ ] Property access with brackets: `n['property']`
- [ ] Dynamic property access: `n[variableKey]`

#### Temporal Operators (Neo4j Expressions Manual)
- [ ] Date/time arithmetic: `date() + duration({days: 7})`
- [ ] Duration multiplication: `duration({hours: 1}) * 3`
- [ ] Temporal subtraction: `date('2024-12-31') - date('2024-01-01')`

### Priority 2: Advanced Pattern Features

#### Shortest Path
- [ ] `shortestPath()`: `MATCH p = shortestPath((a)-[*]-(b))`
- [ ] `allShortestPaths()`

#### Named Paths
- [ ] Path variables: `MATCH p = (a)-[*]-(b)`
- [ ] Path functions: `length(p)`, `nodes(p)`, `relationships(p)`

#### Pattern Predicates
- [ ] `WHERE (n)-[:KNOWS]->(m)` - inline relationship check

### Priority 3: Aggregation Functions

Currently we parse function calls but need specific aggregation support:
- [ ] `sum()`: `RETURN sum(n.amount)`
- [ ] `avg()`: `RETURN avg(n.age)`
- [ ] `min()` / `max()`
- [ ] `collect()`: `RETURN collect(n.name)`
- [ ] `percentileDisc()` / `percentileCont()`
- [ ] `stDev()` / `stDevP()`

### Priority 4: CASE Expressions

- [ ] Simple CASE: `CASE n.status WHEN 'active' THEN 1 ELSE 0 END`
- [ ] Searched CASE: `CASE WHEN n.age > 18 THEN 'adult' ELSE 'minor' END`

### Priority 5: Advanced Clauses

#### DELETE
- [ ] `DELETE` - Delete nodes/relationships
- [ ] `DETACH DELETE` - Delete node and relationships

#### REMOVE
- [ ] `REMOVE` - Remove properties or labels
- [ ] Example: `REMOVE n.property`
- [ ] Example: `REMOVE n:Label`

#### ORDER BY
- [ ] `ORDER BY` - Sort results
- [ ] `ASC` / `DESC`
- [ ] Multiple sort keys

#### LIMIT / SKIP (partially supported)
- [x] `LIMIT` in RETURN (✅ implemented)
- [x] `SKIP` in RETURN (✅ implemented)
- [ ] `LIMIT` / `SKIP` in WITH

#### DISTINCT
- [ ] `RETURN DISTINCT` - Unique results
- [ ] `WITH DISTINCT` - Unique intermediate results

#### UNION
- [ ] `UNION` - Combine query results
- [ ] `UNION ALL` - Include duplicates

#### FOREACH
- [ ] `FOREACH (x IN list | CREATE (n {prop: x}))`

### Priority 6: Neo4j 25 Features (Cypher 25 Only)

#### Conditional Queries (WHEN) - New in Neo4j 2025.06
- [ ] `WHEN` clause for conditional query execution
- [ ] Branching logic based on criteria

#### Sequential Queries (NEXT) - New in Neo4j 2025.06
- [ ] `NEXT` clause for linear query composition
- [ ] Passing return values between sequential queries

### Priority 7: Subquery Expressions

#### EXISTS Subqueries
- [ ] `WHERE EXISTS { (n)-[:KNOWS]->(m) }`
- [ ] Existential pattern matching

#### COLLECT Subqueries
- [ ] `COLLECT { MATCH ... RETURN ... }`
- [ ] Subquery collection

#### COUNT Subqueries
- [ ] `COUNT { MATCH ... }`
- [ ] Subquery counting (different from `count()` function)

### Priority 8: Advanced Path Features (from Basic Queries Manual)

#### Quantified Path Patterns (GQL Conformant)
- [x] Fixed quantifiers: `--{2}--` ✅ (implemented as `*2..2`)
- [x] Range quantifiers: `--{1,4}--` ✅ (implemented as `*1..4`)
- [ ] Unbounded quantifiers: `--{1,}--`
- [ ] Optional quantifier: `--{0,1}--`

#### SHORTEST Keyword (GQL Conformant - replaces shortestPath())
- [ ] `ALL SHORTEST` paths: `p = ALL SHORTEST (a)--+(b)`
- [ ] `ANY SHORTEST` path: `p = ANY SHORTEST (a)--+(b)`
- [ ] `SHORTEST N` paths: `p = SHORTEST 5 (a)--+(b)`
- [ ] `SHORTEST N GROUPS` paths

#### Path Pattern Expressions
- [ ] Path patterns in WHERE: `WHERE (n)-[:KNOWS]->(m)`
- [ ] Path pattern predicates for filtering

#### Type Predicate Expressions
- [ ] `IS :: TYPE` syntax for type checking
- [ ] Value type verification

### Priority 9: Label Expressions (from Basic Queries Manual)

- [x] Simple labels: `:Label` ✅ (implemented)
- [x] Multiple labels: `:Label1:Label2` ✅ (implemented)
- [ ] NOT label expression: `:!Label` (seen in Tom Hanks example)
- [ ] OR label expression: `:Label1|Label2`
- [ ] AND label expression: `:Label1&Label2`
- [ ] Wildcard label: `:%` (any label)
- [ ] Parenthesized expressions: `:(Label1|Label2)`

### Priority 10: Advanced Features

#### Indexes and Constraints (Schema Commands)
- [ ] `CREATE INDEX`
- [ ] `CREATE CONSTRAINT`
- [ ] `DROP INDEX`
- [ ] `DROP CONSTRAINT`
- [ ] `SHOW INDEXES`
- [ ] `SHOW CONSTRAINTS`

#### LOAD CSV
- [ ] `LOAD CSV FROM 'file.csv' AS row`
- [ ] `LOAD CSV WITH HEADERS`
- [ ] `FIELDTERMINATOR` option

#### EXPLAIN / PROFILE
- [ ] `EXPLAIN MATCH ...`
- [ ] `PROFILE MATCH ...`

#### Transaction Control
- [ ] `BEGIN` / `COMMIT` / `ROLLBACK`
- [ ] Transaction functions

#### SHOW Commands
- [ ] `SHOW DATABASES`
- [ ] `SHOW FUNCTIONS`
- [ ] `SHOW PROCEDURES`

## Test Coverage Recommendations

### Phase 1: Expression Operators (High Priority)
Add comprehensive tests for:
1. `IN` operator with lists
2. String matching: `STARTS WITH`, `ENDS WITH`, `CONTAINS`
3. Arithmetic expressions in WHERE and RETURN
4. String concatenation

### Phase 2: Pattern Enhancements
1. `EXISTS` subqueries
2. Path variables and functions
3. `shortestPath` / `allShortestPaths`

### Phase 3: Aggregation & Sorting
1. All aggregation functions
2. `ORDER BY` with multiple keys
3. `DISTINCT` in RETURN and WITH

### Phase 4: Conditional Logic
1. `CASE` expressions (simple and searched)
2. Complex CASE nesting

### Phase 5: Additional Clauses
1. `DELETE` and `DETACH DELETE`
2. `REMOVE`
3. `UNION` / `UNION ALL`
4. `FOREACH`

## Testing Strategy

### Current Test Files
- `test_comprehensive_queries.rs` - 38 tests covering core patterns
- `validation_typecheck_tests.rs` - Type checking tests
- `test_agent_queries.rs` - Real-world agent query tests
- Unit tests in `clauses.rs` - ~150 tests for individual parsers

### Recommended New Test Files
1. `test_expressions.rs` - All expression types and operators
2. `test_aggregations.rs` - Aggregation functions and grouping
3. `test_advanced_patterns.rs` - Shortest paths, EXISTS, etc.
4. `test_conditional.rs` - CASE expressions
5. `test_string_operations.rs` - String matching and manipulation

## Next Steps

1. **Prioritize features** based on agent query patterns
2. **Create expression parser** for arithmetic/string operations
3. **Implement IN operator** (high usage in real queries)
4. **Add string matching operators** (STARTS WITH, CONTAINS, etc.)
5. **Enhance aggregation support** with specific function validation
6. **Add CASE expression support**

## Performance Considerations

Current parser is fast (350 tests in 0.01s). Key optimizations:
- Use `alt()` efficiently for operator precedence
- Minimize backtracking with clear precedence rules
- Cache common sub-parsers
- Keep AST structure lean

## Documentation Links

- [Neo4j Cypher Manual](https://neo4j.com/docs/cypher-manual/current/)
- [Expressions](https://neo4j.com/docs/cypher-manual/current/expressions/)
- [Functions](https://neo4j.com/docs/cypher-manual/current/functions/)
- [Clauses](https://neo4j.com/docs/cypher-manual/current/clauses/)
- [Advanced Cypher (Medium)](https://medium.com/@pankajwahane/mastering-advanced-cypher-unleashing-the-full-potential-of-neo4j-with-aggregations-counting-and-5f362174c51e)

## Success Metrics

- ✅ Current: 350/350 tests (100%)
- 🎯 Target: 500+ tests covering all Priority 1-3 features
- 🎯 Agent query success rate: >95% for real-world patterns
- 🎯 Parse speed: <0.05s for 500+ tests
