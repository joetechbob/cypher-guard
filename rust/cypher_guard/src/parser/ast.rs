// Root of the AST
#[derive(Debug, PartialEq, Clone)]
pub struct Query {
    pub match_clauses: Vec<MatchClause>,
    pub merge_clauses: Vec<MergeClause>,
    pub create_clauses: Vec<CreateClause>,
    pub with_clauses: Vec<WithClause>,
    pub where_clauses: Vec<WhereClause>,
    pub return_clauses: Vec<ReturnClause>,
    pub unwind_clauses: Vec<UnwindClause>,
    pub call_clauses: Vec<CallClause>,
    pub delete_clauses: Vec<DeleteClause>,
    pub remove_clauses: Vec<RemoveClause>,
    pub set_clauses: Vec<SetClause>,
    pub foreach_clauses: Vec<ForeachClause>,
    pub union_queries: Vec<UnionQuery>,  // UNION and UNION ALL queries
}

// UNION query combining multiple queries
#[derive(Debug, PartialEq, Clone)]
pub struct UnionQuery {
    pub query: Box<Query>,
    pub is_all: bool,  // true for UNION ALL, false for UNION
}

// RETURN clause (simple)
#[derive(Debug, PartialEq, Clone)]
pub struct ReturnClause {
    pub items: Vec<String>,
    pub distinct: bool,
    pub order_by: Vec<OrderByItem>,
    pub limit: Option<u64>,
    pub skip: Option<u64>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct OrderByItem {
    pub expression: String,
    pub direction: Option<OrderDirection>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum OrderDirection {
    Asc,
    Desc,
}

// MATCH clause
#[derive(Debug, PartialEq, Clone)]
pub struct MatchClause {
    pub elements: Vec<MatchElement>,
    pub is_optional: bool,
}

// WHERE clause
#[derive(Debug, PartialEq, Clone)]
pub struct WhereClause {
    pub conditions: Vec<WhereCondition>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum WhereCondition {
    Comparison {
        left: PropertyValue,
        operator: String,
        right: PropertyValue,
    },
    FunctionCall {
        function: String,
        arguments: Vec<String>,
    },
    PathProperty {
        path_var: String,
        property: String,
    },
    PatternPredicate {
        pattern: Vec<PatternElement>,
    },
    And(Box<WhereCondition>, Box<WhereCondition>),
    Or(Box<WhereCondition>, Box<WhereCondition>),
    Xor(Box<WhereCondition>, Box<WhereCondition>),
    Not(Box<WhereCondition>),
    Parenthesized(Box<WhereCondition>),
}

#[derive(Debug, PartialEq, Clone)]
pub struct PathProperty {
    pub path_var: String,
    pub property: String,
    pub value: PropertyValue,
}

// Elements of a MATCH clause
#[derive(Debug, PartialEq, Clone)]
pub struct MatchElement {
    pub path_var: Option<String>,
    pub pattern: Vec<PatternElement>,
    pub path_function: Option<PathFunction>,
}

// Path functions like shortestPath() and allShortestPaths()
#[derive(Debug, PartialEq, Clone)]
pub enum PathFunction {
    ShortestPath,
    AllShortestPaths,
}

// Quantified path pattern details
#[derive(Debug, PartialEq, Clone)]
pub struct QuantifiedPathPattern {
    pub pattern: Vec<PatternElement>,
    pub min: Option<u32>,
    pub max: Option<u32>,
    pub where_clause: Option<WhereClause>,
    pub path_variable: Option<String>,
}

// Nodes and relationships that form a pattern
#[derive(Debug, PartialEq, Clone)]
pub enum PatternElement {
    Node(NodePattern),
    Relationship(RelationshipPattern),
    QuantifiedPathPattern(QuantifiedPathPattern),
}

// Label expression for Neo4j 5.x
#[derive(Debug, PartialEq, Clone)]
pub enum LabelExpression {
    Single(String),                                         // :Person
    Or(Box<LabelExpression>, Box<LabelExpression>),       // :Person|Company
    And(Box<LabelExpression>, Box<LabelExpression>),      // :Person&Manager
    Not(Box<LabelExpression>),                            // :!Deleted
}

// Node pattern
#[derive(Debug, PartialEq, Clone)]
pub struct NodePattern {
    pub variable: Option<String>,
    pub label: Option<String>,                             // Legacy single label
    pub label_expression: Option<LabelExpression>,         // Neo4j 5.x label expressions
    pub properties: Option<Vec<Property>>,
}

// Quantifier
#[derive(Debug, PartialEq, Clone)]
pub struct Quantifier {
    pub min: Option<u32>,
    pub max: Option<u32>,
}

// Relationship pattern
#[derive(Debug, PartialEq, Clone)]
pub struct RelationshipDetails {
    pub variable: Option<String>,
    pub direction: Direction,
    pub properties: Option<Vec<Property>>,
    pub rel_type: Option<String>,
    pub length: Option<LengthRange>,
    pub where_clause: Option<WhereClause>,
    pub quantifier: Option<Quantifier>,
    pub is_optional: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub enum RelationshipPattern {
    Regular(RelationshipDetails),
    OptionalRelationship(RelationshipDetails),
}

impl RelationshipPattern {
    pub fn direction(&self) -> Direction {
        match self {
            RelationshipPattern::Regular(details)
            | RelationshipPattern::OptionalRelationship(details) => details.direction.clone(),
        }
    }

    pub fn rel_type(&self) -> Option<&str> {
        match self {
            RelationshipPattern::Regular(details)
            | RelationshipPattern::OptionalRelationship(details) => details.rel_type.as_deref(),
        }
    }

    pub fn properties(&self) -> Option<&Vec<Property>> {
        match self {
            RelationshipPattern::Regular(details)
            | RelationshipPattern::OptionalRelationship(details) => details.properties.as_ref(),
        }
    }

    pub fn quantifier(&self) -> Option<&Quantifier> {
        match self {
            RelationshipPattern::Regular(details)
            | RelationshipPattern::OptionalRelationship(details) => details.quantifier.as_ref(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Direction {
    Left,
    Right,
    Undirected,
}

#[derive(Debug, PartialEq, Clone)]
pub struct LengthRange {
    pub min: Option<u32>,
    pub max: Option<u32>,
}

// Key-value property pairs
#[derive(Debug, PartialEq, Clone)]
pub struct Property {
    pub key: String,
    pub value: PropertyValue,
}

#[derive(Debug, PartialEq, Clone)]
pub enum PropertyValue {
    String(String),
    Number(i64),
    Boolean(bool),
    Null,
    List(Vec<PropertyValue>),
    Map(std::collections::HashMap<String, PropertyValue>),
    FunctionCall {
        name: String,
        args: Vec<PropertyValue>,
    },
    Parameter(String),
    Identifier(String), // For variable references and property access
    BinaryOp {
        left: Box<PropertyValue>,
        operator: String,
        right: Box<PropertyValue>,
    },
    IndexAccess {
        base: Box<PropertyValue>,
        index: Box<PropertyValue>,
    },
    SliceAccess {
        base: Box<PropertyValue>,
        start: Option<Box<PropertyValue>>,
        end: Option<Box<PropertyValue>>,
    },
    ListComprehension {
        variable: String,
        list: Box<PropertyValue>,
        predicate: Option<Box<WhereCondition>>,
        transform: Option<Box<PropertyValue>>,
    },
    PatternComprehension {
        pattern: Vec<PatternElement>,
        predicate: Option<Box<WhereCondition>>,
        transform: Option<Box<PropertyValue>>,
    },
    MapProjection {
        base: Box<PropertyValue>,
        properties: Vec<MapProjectionItem>,
    },
    // Subquery expressions
    ExistsSubquery {
        query: Box<Query>,
    },
    CollectSubquery {
        query: Box<Query>,
    },
    CountSubquery {
        query: Box<Query>,
    },
}

#[derive(Debug, PartialEq, Clone)]
pub enum MapProjectionItem {
    Property(String),              // .name
    AllProperties,                 // .*
    Computed {
        key: String,
        value: PropertyValue,
    },
}

// MERGE clause
#[derive(Debug, PartialEq, Clone)]
pub struct MergeClause {
    pub elements: Vec<MatchElement>,
    pub on_create: Option<OnCreateClause>,
    pub on_match: Option<OnMatchClause>,
}

// CREATE clause
#[derive(Debug, PartialEq, Clone)]
pub struct CreateClause {
    pub elements: Vec<MatchElement>,
}

// ON CREATE clause
#[derive(Debug, PartialEq, Clone)]
pub struct OnCreateClause {
    pub set_clauses: Vec<SetClause>,
}

// ON MATCH clause
#[derive(Debug, PartialEq, Clone)]
pub struct OnMatchClause {
    pub set_clauses: Vec<SetClause>,
}

// SET clause
#[derive(Debug, PartialEq, Clone)]
pub struct SetClause {
    pub variable: String,
    pub property: String,
    pub value: PropertyValue,
}

#[derive(Debug, PartialEq, Clone)]
pub struct WithClause {
    pub items: Vec<WithItem>,
    pub distinct: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub enum WithExpression {
    Identifier(String),
    PropertyAccess {
        variable: String,
        property: String,
    },
    FunctionCall {
        name: String,
        args: Vec<WithExpression>,
    },
    Wildcard,
}

#[derive(Debug, PartialEq, Clone)]
pub struct WithItem {
    pub expression: WithExpression,
    pub alias: Option<String>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct UnwindClause {
    pub expression: UnwindExpression,
    pub variable: String,
}

#[derive(Debug, PartialEq, Clone)]
pub enum UnwindExpression {
    List(Vec<PropertyValue>),
    Identifier(String),
    FunctionCall {
        name: String,
        args: Vec<PropertyValue>,
    },
    Parameter(String),
}

// CALL clause for subqueries and procedures
#[derive(Debug, PartialEq, Clone)]
pub struct CallClause {
    pub subquery: Option<Query>,           // For CALL { ... } subqueries
    pub procedure: Option<String>,         // For CALL procedure() calls
    pub yield_clause: Option<Vec<String>>, // For YIELD clause
}

// DELETE clause for removing nodes and relationships
#[derive(Debug, PartialEq, Clone)]
pub struct DeleteClause {
    pub expressions: Vec<String>, // Variables to delete (e.g., "n", "r")
    pub detach: bool,              // true for DETACH DELETE, false for DELETE
}

// REMOVE clause for removing properties and labels
#[derive(Debug, PartialEq, Clone)]
pub struct RemoveClause {
    pub items: Vec<RemoveItem>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum RemoveItem {
    Property { variable: String, property: String }, // REMOVE n.prop
    Label { variable: String, label: String },       // REMOVE n:Label
}

// FOREACH clause for iteration over lists
#[derive(Debug, PartialEq, Clone)]
pub struct ForeachClause {
    pub variable: String,                    // The iteration variable (e.g., "x")
    pub expression: ForeachExpression,       // The list to iterate over
    pub clauses: Vec<ForeachUpdateClause>,   // The write operations to perform
}

#[derive(Debug, PartialEq, Clone)]
pub enum ForeachExpression {
    List(Vec<PropertyValue>),          // [1, 2, 3]
    Identifier(String),                // someList
    Parameter(String),                 // $list
    FunctionCall {                     // nodes(p), range(1, 10)
        name: String,
        args: Vec<PropertyValue>,
    },
}

// Clauses allowed inside FOREACH (write operations only)
#[derive(Debug, PartialEq, Clone)]
pub enum ForeachUpdateClause {
    Create(CreateClause),
    Merge(MergeClause),
    Set(SetClause),
    Delete(DeleteClause),
    Remove(RemoveClause),
}
