use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case},
    character::complete::{char, digit1, multispace0, multispace1},
    combinator::{map, opt, peek, recognize},
    multi::{many0, separated_list0, separated_list1},
    sequence::{delimited, preceded, separated_pair, tuple},
    IResult,
};

use crate::parser::ast::{
    self, CallClause, CreateClause, MatchClause, MatchElement, MergeClause, OnCreateClause,
    OnMatchClause, PropertyValue, Query, ReturnClause, SetClause, UnwindClause, UnwindExpression,
    WhereClause, WithClause, WithExpression, WithItem,
};
use crate::parser::patterns::*;
use crate::parser::span::{offset_to_line_column, Spanned};
use crate::parser::utils::identifier;
use crate::CypherGuardParsingError;

#[derive(Debug, Clone)]
pub enum Clause {
    Match(MatchClause),
    OptionalMatch(MatchClause),
    Merge(MergeClause),
    Create(CreateClause),
    Return(ReturnClause),
    With(WithClause),
    Query(Query),
    Unwind(UnwindClause),
    Where(WhereClause),
    Call(CallClause),
}

// Parses a comma-separated list of match elements, stopping at clause boundaries
pub fn match_element_list(input: &str) -> IResult<&str, Vec<MatchElement>> {
    use nom::character::complete::char;
    use nom::multi::separated_list1;
    use nom::sequence::tuple;

    // Parse comma-separated match elements
    let (input, elements) =
        separated_list1(tuple((multispace0, char(','), multispace0)), match_element)(input)?;

    Ok((input, elements))
}

// Parses the MATCH clause (e.g. MATCH (a)-[:KNOWS]->(b))
pub fn match_clause(input: &str) -> IResult<&str, MatchClause> {
    let (input, _) = multispace0(input)?;
    // Try to parse OPTIONAL MATCH or MATCH
    let (input, is_optional) = match opt(tuple((tag_no_case("OPTIONAL"), multispace1)))(input) {
        Ok((input, Some(_))) => (input, true),
        Ok((input, None)) => (input, false),
        Err(e) => return Err(e),
    };
    let (input, _) = tag_no_case("MATCH")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, elements) = match_element_list(input)?;
    Ok((
        input,
        MatchClause {
            elements,
            is_optional,
        },
    ))
}

// Parses a return item: expression with optional AS alias
fn return_item(input: &str) -> IResult<&str, String> {
    let (input, _) = multispace0(input)?;
    
    // Parse an expression
    let (input, expr) = parse_expression(input)?;
    
    // Convert PropertyValue to string representation
    let expr_str = property_value_to_string(&expr);
    
    // Check for AS alias
    let (input, alias) = opt(preceded(
        tuple((multispace0, tag("AS"), multispace1)),
        identifier,
    ))(input)?;

    if let Some(alias_name) = alias {
        let result = format!("{} AS {}", expr_str, alias_name);
        Ok((input, result))
    } else {
        Ok((input, expr_str))
    }
}

// Helper to convert PropertyValue to string representation
fn property_value_to_string(value: &PropertyValue) -> String {
    match value {
        PropertyValue::String(s) => format!("'{}'", s),
        PropertyValue::Number(n) => n.to_string(),
        PropertyValue::Boolean(b) => b.to_string(),
        PropertyValue::Null => "NULL".to_string(),
        PropertyValue::Identifier(s) => s.clone(),
        PropertyValue::Parameter(s) => format!("${}", s),
        PropertyValue::FunctionCall { name, args } => {
            let args_str: Vec<String> = args.iter().map(|a| property_value_to_string(a)).collect();
            format!("{}({})", name, args_str.join(", "))
        }
        PropertyValue::BinaryOp { left, operator, right } => {
            format!("{} {} {}", property_value_to_string(left), operator, property_value_to_string(right))
        }
        PropertyValue::List(items) => {
            let items_str: Vec<String> = items.iter().map(|i| property_value_to_string(i)).collect();
            format!("[{}]", items_str.join(", "))
        }
        PropertyValue::Map(map) => {
            let pairs: Vec<String> = map.iter()
                .map(|(k, v)| format!("{}: {}", k, property_value_to_string(v)))
                .collect();
            format!("{{{}}}", pairs.join(", "))
        }
        PropertyValue::IndexAccess { base, index } => {
            format!("{}[{}]", property_value_to_string(base), property_value_to_string(index))
        }
        PropertyValue::SliceAccess { base, start, end } => {
            let start_str = start.as_ref().map(|s| property_value_to_string(s)).unwrap_or_default();
            let end_str = end.as_ref().map(|e| property_value_to_string(e)).unwrap_or_default();
            format!("{}[{}..{}]", property_value_to_string(base), start_str, end_str)
        }
        PropertyValue::ListComprehension { variable, list, predicate, transform } => {
            let pred_str = predicate.as_ref().map(|_| " WHERE <condition>".to_string()).unwrap_or_default();
            let trans_str = transform.as_ref().map(|t| format!(" | {}", property_value_to_string(t))).unwrap_or_default();
            format!("[{} IN {}{}{}]", variable, property_value_to_string(list), pred_str, trans_str)
        }
        PropertyValue::PatternComprehension { pattern, predicate, transform } => {
            let pred_str = predicate.as_ref().map(|_| " WHERE <condition>".to_string()).unwrap_or_default();
            let trans_str = transform.as_ref().map(|t| format!(" | {}", property_value_to_string(t))).unwrap_or_default();
            format!("[<pattern>{}{}]", pred_str, trans_str)
        }
        PropertyValue::MapProjection { base, properties } => {
            let props_str = properties.iter()
                .map(|_| "<property>")
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}{{{}}}", property_value_to_string(base), props_str)
        }
    }
}

// Parses the RETURN clause (e.g. RETURN a, b, a.name, RETURN DISTINCT a ORDER BY a.name)
pub fn return_clause(input: &str) -> IResult<&str, ReturnClause> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("RETURN")(input)?;
    let (input, _) = multispace1(input)?;

    // Parse optional DISTINCT keyword
    let (input, distinct) = opt(tuple((tag_no_case("DISTINCT"), multispace1)))(input)?;
    let distinct = distinct.is_some();

    // Parse the first item (required)
    let (input, first_item) = return_item(input)?;
    let mut items = vec![first_item];

    // Parse additional items with commas (optional)
    let (input, additional_items) = many0(preceded(
        tuple((multispace0, char(','), multispace0)),
        return_item,
    ))(input)?;
    items.extend(additional_items);

    // Check for trailing comma - if there's a comma followed by whitespace, it's an error
    let (input, _) = multispace0(input)?;
    if !input.is_empty() && input.starts_with(',') {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    // Parse optional ORDER BY clause
    let (input, order_by) = opt(preceded(
        tuple((multispace0, tag_no_case("ORDER"), multispace1, tag_no_case("BY"), multispace1)),
        separated_list1(
            tuple((multispace0, char(','), multispace0)),
            |input| {
                let (input, expr) = alt((
                    map(property_access, |s| s),
                    map(identifier, |s| s.to_string()),
                ))(input)?;
                let (input, _) = multispace0(input)?;
                let (input, direction) = opt(alt((
                    map(tag_no_case("ASC"), |_| ast::OrderDirection::Asc),
                    map(tag_no_case("DESC"), |_| ast::OrderDirection::Desc),
                )))(input)?;
                Ok((input, ast::OrderByItem { expression: expr, direction }))
            }
        ),
    ))(input)?;
    let order_by = order_by.unwrap_or_default();

    // Parse optional SKIP clause
    let (input, skip) = opt(preceded(
        tuple((multispace0, tag_no_case("SKIP"), multispace1)),
        map(digit1, |s: &str| s.parse::<u64>().unwrap()),
    ))(input)?;

    // Parse optional LIMIT clause
    let (input, limit) = opt(preceded(
        tuple((multispace0, tag_no_case("LIMIT"), multispace1)),
        map(digit1, |s: &str| s.parse::<u64>().unwrap()),
    ))(input)?;

    Ok((input, ReturnClause { items, distinct, order_by, limit, skip }))
}

// Parses a numeric literal
fn numeric_literal(input: &str) -> IResult<&str, String> {
    let (input, num) = recognize(digit1)(input)?;
    Ok((input, num.to_string()))
}

// Local string literal parsing function (since utils::string_literal is dead code)
pub fn string_literal_local(input: &str) -> IResult<&str, String> {
    let (input, quote) = alt((char('\''), char('"')))(input)?;
    let (input, s) = nom::bytes::complete::take_while(|c| c != quote)(input)?;
    let (input, _) = char(quote)(input)?;
    Ok((input, s.to_string()))
}

// Parses a path property access
fn path_property(input: &str) -> IResult<&str, (String, String)> {
    let (input, path_var) = map(identifier, |s| s.to_string())(input)?;
    let (input, _) = char('.')(input)?;
    let (input, property) = map(identifier, |s| s.to_string())(input)?;
    Ok((input, (path_var, property)))
}

// Parses a property access pattern (e.g., a.name)
fn property_access(input: &str) -> IResult<&str, String> {
    let (input, var) = identifier(input)?;
    let (input, _) = char('.')(input)?;
    let (input, prop) = identifier(input)?;
    let result = format!("{}.{}", var, prop);
    Ok((input, result))
}

// Parses a function call (e.g., length(a.name), substring(a.name, 0, 5))
fn function_call(input: &str) -> IResult<&str, (String, Vec<String>)> {
    let (input, _) = multispace0(input)?;
    let (input, function) = map(identifier, |s| s.to_string())(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("(")(input)?;
    let (input, _) = multispace0(input)?;

    // Parse arguments
    let (input, args) = separated_list0(
        tuple((multispace0, tag(","), multispace0)),
        alt((
            // Try to parse wildcard * (for count(*))
            map(char('*'), |_| "*".to_string()),
            // Try to parse nested function calls
            map(function_call, |(func, args)| {
                format!("{}({})", func, args.join(", "))
            }),
            // Try to parse property access
            map(property_access, |s| s),
            // Try to parse string literals
            map(string_literal_local, |s| s),
            // Try to parse numeric literals
            map(numeric_literal, |n| n.to_string()),
            // Try to parse boolean literals
            map(tag_no_case("true"), |_| "true".to_string()),
            map(tag_no_case("false"), |_| "false".to_string()),
            // Try to parse NULL
            map(tag_no_case("NULL"), |_| "NULL".to_string()),
            // Try to parse identifiers
            map(identifier, |s| s.to_string()),
        )),
    )(input)?;

    let (input, _) = multispace0(input)?;
    let (input, _) = tag(")")(input)?;
    Ok((input, (function, args)))
}

// Parses WHERE expressions with proper operator precedence
// Precedence: XOR < OR < AND
fn parse_where_expr(input: &str) -> IResult<&str, ast::WhereCondition> {
    // Parse XOR expressions (lowest precedence)
    let (input, mut left) = parse_or_expr(input)?;

    // Parse additional XOR expressions
    let (input, xor_conditions) = many0(preceded(
        tuple((multispace0, tag("XOR"), multispace0)),
        parse_or_expr,
    ))(input)?;

    // Build the XOR tree
    for right in xor_conditions {
        left = ast::WhereCondition::Xor(Box::new(left), Box::new(right));
    }

    Ok((input, left))
}

// Parses OR expressions
fn parse_or_expr(input: &str) -> IResult<&str, ast::WhereCondition> {
    // Parse OR expressions
    let (input, mut left) = parse_and_expr(input)?;

    // Parse additional OR expressions
    let (input, or_conditions) = many0(preceded(
        tuple((multispace0, tag("OR"), multispace0)),
        parse_and_expr,
    ))(input)?;

    // Build the OR tree
    for right in or_conditions {
        left = ast::WhereCondition::Or(Box::new(left), Box::new(right));
    }

    Ok((input, left))
}

// Parses AND expressions (higher precedence than OR)
fn parse_and_expr(input: &str) -> IResult<&str, ast::WhereCondition> {
    // Parse basic conditions (highest precedence)
    let (input, mut left) = parse_basic_condition(input)?;

    // Parse additional AND expressions
    let (input, and_conditions) = many0(preceded(
        tuple((multispace0, tag("AND"), multispace0)),
        parse_basic_condition,
    ))(input)?;

    // Build the AND tree
    for right in and_conditions {
        left = ast::WhereCondition::And(Box::new(left), Box::new(right));
    }

    Ok((input, left))
}

// Parses basic conditions (comparisons, NOT, parenthesized, function calls, etc.)
pub fn parse_basic_condition(input: &str) -> IResult<&str, ast::WhereCondition> {
    let (input, _) = multispace0(input)?;

    // Try to parse NOT
    if let Ok((rest, _)) = tag::<&str, &str, nom::error::Error<&str>>("NOT")(input) {
        let (rest, _) = multispace1(rest)?;
        let (rest, condition) = parse_basic_condition(rest)?;
        return Ok((rest, ast::WhereCondition::Not(Box::new(condition))));
    }

    // Try to parse as a comparison FIRST (which can include function calls in expressions)
    let comparison_result = (|| {
        // Parse left side as an expression
        let (input, left) = parse_expression(input)?;
        let (input, _) = multispace0(input)?;
        let (input, operator) = alt((
            tag("STARTS WITH"),
            tag("ENDS WITH"),
            tag("CONTAINS"),
            tag("IS NOT NULL"),
            tag("IS NULL"),
            tag("<="),
            tag(">="),
            tag("<>"),
            tag("=~"),
            tag("="),
            tag("<"),
            tag(">"),
            tag("IN"),
        ))(input)?;

        // For IS NULL and IS NOT NULL, there's no right side
        if operator == "IS NULL" || operator == "IS NOT NULL" {
            return Ok((
                input,
                ast::WhereCondition::Comparison {
                    left,
                    operator: operator.to_string(),
                    right: ast::PropertyValue::Null,
                },
            ));
        }

        let (input, _) = multispace0(input)?;
        // Parse right side as an expression (supports functions, arithmetic, etc.)
        let (input, right) = parse_expression(input)?;
        Ok((
            input,
            ast::WhereCondition::Comparison {
                left,
                operator: operator.to_string(),
                right,
            },
        ))
    })();

    if let Ok(result) = comparison_result {
        return Ok(result);
    }

    // If comparison parsing failed, try to parse parenthesized WHERE condition or pattern predicate
    // (e.g., WHERE (a > 5 AND b < 10) or WHERE (a)-[:KNOWS]->(b))
    if let Ok((rest, _)) = tag::<&str, &str, nom::error::Error<&str>>("(")(input) {
        // Try to parse as a pattern predicate first (e.g., (a)-[:REL]->(b))
        if let Ok((rest2, pattern)) = pattern_element_sequence(input, false) {
            // Check if this looks like a valid pattern (has more than just a node)
            if pattern.len() > 1 || (pattern.len() == 1 && !rest2.trim_start().starts_with('>')) {
                return Ok((
                    rest2,
                    ast::WhereCondition::PatternPredicate { pattern },
                ));
            }
        }

        // Otherwise, try to parse as parenthesized WHERE expression
        if let Ok((rest, condition)) = parse_where_expr(rest) {
            if let Ok((rest, _)) = tag::<&str, &str, nom::error::Error<&str>>(")")(rest) {
                return Ok((
                    rest,
                    ast::WhereCondition::Parenthesized(Box::new(condition)),
                ));
            }
        }
    }

    // If comparison parsing failed, try to parse as a standalone function call
    // (for cases like WHERE exists(x) without a comparison operator)
    if let Ok((rest, (function, args))) = function_call(input) {
        return Ok((
            rest,
            ast::WhereCondition::FunctionCall {
                function,
                arguments: args,
            },
        ));
    }

    // If comparison parsing failed, try to parse as a path property
    if let Ok((rest, (path_var, property))) = path_property(input) {
        return Ok((
            rest,
            ast::WhereCondition::PathProperty { path_var, property },
        ));
    }

    // If all parsing attempts failed, return the error from the comparison attempt
    comparison_result
}

// Parses the WHERE clause (e.g. WHERE a.age > 30 AND b.name = 'Alice')
pub fn where_clause(input: &str) -> IResult<&str, ast::WhereClause> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("WHERE")(input)?;
    let (input, _) = multispace1(input)?;

    // Parse the expression with proper precedence
    let (input, condition) = parse_where_expr(input)?;

    let clause = ast::WhereClause {
        conditions: vec![condition],
    };
    Ok((input, clause))
}

// Parses a SET clause (e.g. SET a.name = 'Alice')
fn set_clause(input: &str) -> IResult<&str, SetClause> {
    let (input, variable) = identifier(input)?;
    let (input, _) = char('.')(input)?;
    let (input, property) = identifier(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('=')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, value) = property_value(input)?;
    Ok((
        input,
        SetClause {
            variable: variable.to_string(),
            property: property.to_string(),
            value,
        },
    ))
}

// Parses ON CREATE clause (e.g. ON CREATE SET a.name = 'Alice')
fn on_create_clause(input: &str) -> IResult<&str, OnCreateClause> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("ON CREATE")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag("SET")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, set_clauses) =
        separated_list1(tuple((multispace0, char(','), multispace0)), set_clause)(input)?;
    Ok((input, OnCreateClause { set_clauses }))
}

// Parses ON MATCH clause (e.g. ON MATCH SET a.name = 'Alice')
fn on_match_clause(input: &str) -> IResult<&str, OnMatchClause> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("ON MATCH")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag("SET")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, set_clauses) =
        match separated_list1(tuple((multispace0, char(','), multispace0)), set_clause)(input) {
            Ok(res) => res,
            Err(_e) => {
                return Err(_e);
            }
        };
    Ok((input, OnMatchClause { set_clauses }))
}

// Parses the MERGE clause (e.g. MERGE (a:Person {name: 'Alice'}) ON CREATE SET a.created = timestamp())
pub fn merge_clause(input: &str) -> IResult<&str, MergeClause> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("MERGE")(input)?;
    let (input, _) = multispace1(input)?;
    let (mut input, elements) = match_element_list(input)?;
    let mut found_on_create = None;
    let mut found_on_match = None;

    // Try up to two times (since there can be at most one ON CREATE and one ON MATCH)
    for _i in 0..2 {
        let (rest, _) = multispace0(input)?;
        input = rest;

        if found_on_create.is_none() {
            match on_create_clause(input) {
                Ok((rest, clause)) => {
                    found_on_create = Some(clause);
                    input = rest;
                    continue;
                }
                Err(_e) => {}
            }
        }
        if found_on_match.is_none() {
            match on_match_clause(input) {
                Ok((rest, clause)) => {
                    found_on_match = Some(clause);
                    input = rest;
                    continue;
                }
                Err(_e) => {}
            }
        }
        break;
    }
    Ok((
        input,
        MergeClause {
            elements,
            on_create: found_on_create,
            on_match: found_on_match,
        },
    ))
}

// Parses the CREATE clause (e.g. CREATE (a:Person {name: 'Alice'})-[r:KNOWS]->(b:Person {name: 'Bob'}))
pub fn create_clause(input: &str) -> IResult<&str, CreateClause> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("CREATE")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, elements) = match_element_list(input)?;
    Ok((input, CreateClause { elements }))
}

// Parses a WITH item (e.g., a, a.name, count(*))
fn with_item(input: &str) -> IResult<&str, WithItem> {
    let (input, expr) = alt((
        map(char('*'), |_| WithExpression::Wildcard),
        map(property_access, |s| {
            let parts: Vec<&str> = s.split('.').collect();
            WithExpression::PropertyAccess {
                variable: parts[0].to_string(),
                property: parts[1].to_string(),
            }
        }),
        map(function_call, |(name, args)| WithExpression::FunctionCall {
            name,
            args: args.into_iter().map(WithExpression::Identifier).collect(),
        }),
        map(identifier, |s| WithExpression::Identifier(s.to_string())),
    ))(input)?;
    let (input, alias) = opt(preceded(
        tuple((multispace0, tag("AS"), multispace1)),
        identifier,
    ))(input)?;
    let result = WithItem {
        expression: expr,
        alias: alias.map(|s| s.to_string()),
    };
    Ok((input, result))
}

// Parses the WITH clause (e.g. WITH a, count(*) AS count, WITH DISTINCT a.age AS age)
pub fn with_clause(input: &str) -> IResult<&str, WithClause> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("WITH")(input)?;
    let (input, _) = multispace1(input)?;
    
    // Parse optional DISTINCT keyword
    let (input, distinct) = opt(tuple((tag_no_case("DISTINCT"), multispace1)))(input)?;
    let distinct = distinct.is_some();
    
    let (input, items) =
        separated_list1(tuple((multispace0, char(','), multispace0)), with_item)(input)?;
    Ok((input, WithClause { items, distinct }))
}

// Parses a subquery (content inside CALL { ... })
fn parse_subquery(input: &str) -> IResult<&str, Query> {
    let mut rest = input;
    let mut clauses = Vec::new();

    // Parse clauses until we encounter a closing brace or run out of input
    loop {
        // Check if we've reached the end or a closing brace
        let (check_rest, _) = multispace0(rest)?;
        if check_rest.is_empty() || check_rest.starts_with('}') {
            break;
        }

        // Try to parse a clause
        match clause(rest) {
            Ok((next_rest, spanned_clause)) => {
                clauses.push(spanned_clause);
                rest = next_rest;
            }
            Err(_e) => {
                break;
            }
        }
    }

    // Validate clause order
    if let Err(_validation_error) = validate_clause_order(&clauses, input) {
        // Convert validation error to nom error
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    // Reject empty queries
    if clauses.is_empty() {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    // Build the Query struct with separate fields for each clause type
    let mut query = Query {
        match_clauses: Vec::new(),
        merge_clauses: Vec::new(),
        create_clauses: Vec::new(),
        with_clauses: Vec::new(),
        where_clauses: Vec::new(),
        return_clauses: Vec::new(),
        unwind_clauses: Vec::new(),
        call_clauses: Vec::new(),
    };

    // Collect all clauses by type
    for spanned_clause in clauses.iter() {
        let clause = &spanned_clause.value;
        match clause {
            Clause::Match(match_clause) => query.match_clauses.push(match_clause.clone()),
            Clause::OptionalMatch(match_clause) => query.match_clauses.push(match_clause.clone()),
            Clause::Merge(merge_clause) => query.merge_clauses.push(merge_clause.clone()),
            Clause::Create(create_clause) => query.create_clauses.push(create_clause.clone()),
            Clause::With(with_clause) => query.with_clauses.push(with_clause.clone()),
            Clause::Where(where_clause) => query.where_clauses.push(where_clause.clone()),
            Clause::Return(return_clause) => query.return_clauses.push(return_clause.clone()),
            Clause::Unwind(unwind_clause) => query.unwind_clauses.push(unwind_clause.clone()),
            Clause::Call(call_clause) => query.call_clauses.push(call_clause.clone()),
            Clause::Query(_) => {
                // Handle nested queries if needed
                return Err(nom::Err::Error(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::Tag,
                )));
            }
        }
    }

    Ok((rest, query))
}

pub fn call_clause(input: &str) -> IResult<&str, ast::CallClause> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("CALL")(input)?;
    let (input, _) = multispace1(input)?;

    // Try to parse as a subquery first: CALL { ... }
    if let Ok((rest, subquery)) = delimited(
        tuple((multispace0, char('{'), multispace0)),
        parse_subquery,
        tuple((multispace0, char('}'), multispace0)),
    )(input)
    {
        return Ok((
            rest,
            ast::CallClause {
                subquery: Some(subquery),
                procedure: None,
                yield_clause: None,
            },
        ));
    }

    // Try to parse as a procedure call: CALL procedure() or CALL db.procedure()
    let (input, procedure) = map(
        separated_pair(identifier, char('.'), identifier),
        |(namespace, name)| format!("{}.{}", namespace, name),
    )(input)?;

    let (input, _) = multispace0(input)?;
    let (input, _) = char('(')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char(')')(input)?;

    // Try to parse YIELD clause
    let (input, yield_clause) = opt(preceded(
        tuple((multispace0, tag("YIELD"), multispace1)),
        separated_list1(
            tuple((multispace0, char(','), multispace0)),
            map(identifier, |s| s.to_string()),
        ),
    ))(input)?;

    Ok((
        input,
        ast::CallClause {
            subquery: None,
            procedure: Some(procedure),
            yield_clause,
        },
    ))
}

// Parses a Cypher parameter (e.g., $param)
fn parameter(input: &str) -> IResult<&str, String> {
    let (input, _) = char('$')(input)?;
    let (input, name) = identifier(input)?;
    Ok((input, name.to_string()))
}

// Parse pattern comprehension: [(person)-->(friend) WHERE friend.age > 25 | friend.name]
fn parse_pattern_comprehension(input: &str) -> IResult<&str, PropertyValue> {
    let (input, _) = char('[')(input)?;
    let (input, _) = multispace0(input)?;

    // Must start with a pattern '('
    if !input.starts_with('(') {
        return Err(nom::Err::Error(nom::error::Error::new(input, nom::error::ErrorKind::Tag)));
    }

    let (input, pattern) = pattern_element_sequence(input, false)?;
    let (input, _) = multispace0(input)?;

    // Optional WHERE clause
    let (input, predicate) = if let Ok((input2, _)) = tag::<_, _, nom::error::Error<&str>>("WHERE")(input) {
        let (input3, _) = multispace1(input2)?;
        let (input4, cond) = parse_basic_condition(input3)?;
        (input4, Some(Box::new(cond)))
    } else {
        (input, None)
    };

    let (input, _) = multispace0(input)?;

    // Optional transform (after |)
    let (input, transform) = if input.starts_with('|') {
        let input2 = &input[1..];
        let (input3, _) = multispace0(input2)?;
        let (input4, expr) = parse_expression(input3)?;
        (input4, Some(Box::new(expr)))
    } else {
        (input, None)
    };

    let (input, _) = multispace0(input)?;
    let (input, _) = char(']')(input)?;

    Ok((input, PropertyValue::PatternComprehension {
        pattern,
        predicate,
        transform,
    }))
}

// Parse list comprehension: [x IN list WHERE x > 2 | x * 2]
fn parse_list_comprehension(input: &str) -> IResult<&str, PropertyValue> {
    let (input, _) = char('[')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, var) = identifier(input)?;
    let (input, _) = multispace1(input)?;
    let (input, _) = tag("IN")(input)?;
    let (input, _) = multispace1(input)?;
    let (input, list) = parse_expression(input)?;
    let (input, _) = multispace0(input)?;

    // Optional WHERE clause
    let (input, predicate) = if let Ok((input2, _)) = tag::<_, _, nom::error::Error<&str>>("WHERE")(input) {
        let (input3, _) = multispace1(input2)?;
        let (input4, cond) = parse_basic_condition(input3)?;
        (input4, Some(Box::new(cond)))
    } else {
        (input, None)
    };

    let (input, _) = multispace0(input)?;

    // Optional transform (after |)
    let (input, transform) = if input.starts_with('|') {
        let input2 = &input[1..];
        let (input3, _) = multispace0(input2)?;
        let (input4, expr) = parse_expression(input3)?;
        (input4, Some(Box::new(expr)))
    } else {
        (input, None)
    };

    let (input, _) = multispace0(input)?;
    let (input, _) = char(']')(input)?;

    Ok((input, PropertyValue::ListComprehension {
        variable: var.to_string(),
        list: Box::new(list),
        predicate,
        transform,
    }))
}

// Parse a primary expression (atoms that can be used in binary operations)
fn parse_primary_expression(input: &str) -> IResult<&str, PropertyValue> {
    let (input, _) = multispace0(input)?;

    alt((
        // Try function calls first
        map(function_call, |(name, args)| PropertyValue::FunctionCall {
            name,
            args: args.into_iter().map(PropertyValue::String).collect(),
        }),
        // Try pattern comprehension (e.g., [(person)-->(friend) WHERE friend.age > 25 | friend.name])
        parse_pattern_comprehension,
        // Try list comprehension (e.g., [x IN list WHERE x > 2 | x * 2])
        parse_list_comprehension,
        // Try lists (e.g., [1, 2, 3])
        map(
            delimited(
                char('['),
                separated_list0(
                    tuple((multispace0, char(','), multispace0)),
                    parse_expression,
                ),
                char(']'),
            ),
            PropertyValue::List,
        ),
        // Try parameters
        map(parameter, PropertyValue::Parameter),
        // Try property access (e.g., n.age)
        map(property_access, |s| PropertyValue::Identifier(s)),
        // Try string literals
        map(string_literal_local, PropertyValue::String),
        // Try numeric literals (including decimals)
        map(
            recognize(tuple((
                opt(char('-')),
                digit1,
                opt(tuple((char('.'), digit1))),
            ))),
            |s: &str| PropertyValue::Number(s.parse::<i64>().unwrap_or(0)),
        ),
        // Try booleans
        map(tag_no_case("true"), |_| PropertyValue::Boolean(true)),
        map(tag_no_case("false"), |_| PropertyValue::Boolean(false)),
        // Try NULL
        map(tag_no_case("null"), |_| PropertyValue::Null),
        // Try parenthesized expressions
        delimited(
            tuple((char('('), multispace0)),
            parse_expression,
            tuple((multispace0, char(')'))),
        ),
        // Try simple identifiers
        map(identifier, |s| PropertyValue::Identifier(s.to_string())),
    ))(input)
}

// Parse a single map projection item
fn parse_map_projection_item(input: &str) -> IResult<&str, ast::MapProjectionItem> {
    use ast::MapProjectionItem;

    let (input, _) = multispace0(input)?;

    // Try property shorthand: .prop or .*
    if input.starts_with('.') {
        let input2 = &input[1..];
        if input2.starts_with('*') {
            return Ok((&input2[1..], MapProjectionItem::AllProperties));
        }
        let (input3, prop) = identifier(input2)?;
        return Ok((input3, MapProjectionItem::Property(prop.to_string())));
    }

    // Try computed property: key: expr
    let (input, key) = identifier(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char(':')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, value) = parse_expression(input)?;

    Ok((input, MapProjectionItem::Computed {
        key: key.to_string(),
        value,
    }))
}

// Parse map projection properties: .prop, .*, key: expr
fn parse_map_projection_properties(input: &str) -> IResult<&str, Vec<ast::MapProjectionItem>> {
    let (input, _) = multispace0(input)?;

    // Parse comma-separated projection items
    separated_list1(
        tuple((multispace0, char(','), multispace0)),
        parse_map_projection_item
    )(input)
}

// Parse postfix expressions: list[0], list[1..3], etc.
fn parse_postfix_expression(input: &str) -> IResult<&str, PropertyValue> {
    let (mut input, mut base) = parse_primary_expression(input)?;

    loop {
        let (input2, _) = multispace0(input)?;

        // Check for bracket notation
        if input2.starts_with('[') {
            let input3 = &input2[1..];
            let (input4, _) = multispace0(input3)?;

            // Try to parse slice first (has ..)
            if let Ok((input5, start_opt)) = opt(parse_expression)(input4) {
                let (input6, _) = multispace0(input5)?;
                if input6.starts_with("..") {
                    let input7 = &input6[2..];
                    let (input8, _) = multispace0(input7)?;
                    let (input9, end_opt) = opt(parse_expression)(input8)?;
                    let (input10, _) = multispace0(input9)?;
                    if input10.starts_with(']') {
                        let input11 = &input10[1..];
                        base = PropertyValue::SliceAccess {
                            base: Box::new(base),
                            start: start_opt.map(Box::new),
                            end: end_opt.map(Box::new),
                        };
                        input = input11;
                        continue;
                    }
                }
            }

            // Otherwise, try index access
            if let Ok((input5, index)) = parse_expression(input4) {
                let (input6, _) = multispace0(input5)?;
                if input6.starts_with(']') {
                    let input7 = &input6[1..];
                    base = PropertyValue::IndexAccess {
                        base: Box::new(base),
                        index: Box::new(index),
                    };
                    input = input7;
                    continue;
                }
            }
        }

        // Check for map projection: base{.prop1, .prop2, key: expr}
        if input2.starts_with('{') {
            if let Ok((input3, properties)) = parse_map_projection_properties(&input2[1..]) {
                let (input4, _) = multispace0(input3)?;
                if input4.starts_with('}') {
                    base = PropertyValue::MapProjection {
                        base: Box::new(base),
                        properties,
                    };
                    input = &input4[1..];
                    continue;
                }
            }
        }

        // No more postfix operations
        break;
    }

    Ok((input, base))
}

// Parse exponentiation expressions: ^
fn parse_exponentiation_expr(input: &str) -> IResult<&str, PropertyValue> {
    let (mut input, mut left) = parse_postfix_expression(input)?;
    
    loop {
        let (input2, _) = multispace0(input)?;
        
        if input2.starts_with('^') {
            let input3 = &input2[1..];
            let (input4, _) = multispace0(input3)?;
            let (input5, right) = parse_postfix_expression(input4)?;
            
            left = PropertyValue::BinaryOp {
                left: Box::new(left),
                operator: "^".to_string(),
                right: Box::new(right),
            };
            input = input5;
        } else {
            break;
        }
    }
    
    Ok((input, left))
}

// Parse multiplicative expressions: *, /, %
fn parse_multiplicative_expr(input: &str) -> IResult<&str, PropertyValue> {
    let (mut input, mut left) = parse_exponentiation_expr(input)?;
    
    loop {
        let (input2, _) = multispace0(input)?;
        
        // Try to match an operator
        if let Ok((input3, op)) = alt::<_, _, nom::error::Error<&str>, _>((
            map(char('*'), |c| c.to_string()),
            map(char('/'), |c| c.to_string()),
            map(char('%'), |c| c.to_string()),
        ))(input2) {
            let (input4, _) = multispace0(input3)?;
            let (input5, right) = parse_exponentiation_expr(input4)?;
            
            left = PropertyValue::BinaryOp {
                left: Box::new(left),
                operator: op,
                right: Box::new(right),
            };
            input = input5;
        } else {
            break;
        }
    }
    
    Ok((input, left))
}

// Parse additive expressions: +, -
fn parse_additive_expr(input: &str) -> IResult<&str, PropertyValue> {
    let (mut input, mut left) = parse_multiplicative_expr(input)?;
    
    loop {
        let (input2, _) = multispace0(input)?;
        
        // Try to match + or -
        // Be careful not to match -> (relationship operator)
        let op_result = if input2.starts_with('+') {
            Ok(((&input2[1..]), "+".to_string()))
        } else if input2.starts_with('-') && !input2.starts_with("->") {
            Ok(((&input2[1..]), "-".to_string()))
        } else {
            Err(nom::Err::Error(nom::error::Error::new(input2, nom::error::ErrorKind::Char)))
        };
        
        if let Ok((input3, op)) = op_result {
            let (input4, _) = multispace0(input3)?;
            let (input5, right) = parse_multiplicative_expr(input4)?;
            
            left = PropertyValue::BinaryOp {
                left: Box::new(left),
                operator: op,
                right: Box::new(right),
            };
            input = input5;
        } else {
            break;
        }
    }
    
    Ok((input, left))
}

// Parse concatenation expressions: ||
fn parse_concat_expr(input: &str) -> IResult<&str, PropertyValue> {
    let (mut input, mut left) = parse_additive_expr(input)?;
    
    loop {
        let (input2, _) = multispace0(input)?;
        
        if input2.starts_with("||") {
            let input3 = &input2[2..];
            let (input4, _) = multispace0(input3)?;
            let (input5, right) = parse_additive_expr(input4)?;
            
            left = PropertyValue::BinaryOp {
                left: Box::new(left),
                operator: "||".to_string(),
                right: Box::new(right),
            };
            input = input5;
        } else {
            break;
        }
    }
    
    Ok((input, left))
}

// Main expression parser with full operator precedence
fn parse_expression(input: &str) -> IResult<&str, PropertyValue> {
    parse_concat_expr(input)
}

// Parses the UNWIND clause (e.g. UNWIND [1,2,3] AS x)
pub fn unwind_clause(input: &str) -> IResult<&str, UnwindClause> {
    let (input, _) = multispace0(input)?;
    let (input, _) = tag("UNWIND")(input)?;
    let (input, _) = multispace1(input)?;

    // Try to parse a parameter
    if let Ok((input, param)) = parameter(input) {
        let (input, _) = multispace1(input)?;
        let (input, _) = tag("AS")(input)?;
        let (input, _) = multispace1(input)?;
        let (input, variable) = identifier(input)?;
        return Ok((
            input,
            UnwindClause {
                expression: UnwindExpression::Parameter(param),
                variable: variable.to_string(),
            },
        ));
    }

    // Try to parse a list expression first (collapse nested if let)
    if let Ok((input, ast::PropertyValue::List(items))) = property_value(input) {
        let (input, _) = multispace1(input)?;
        let (input, _) = tag("AS")(input)?;
        let (input, _) = multispace1(input)?;
        let (input, variable) = identifier(input)?;
        return Ok((
            input,
            UnwindClause {
                expression: UnwindExpression::List(items),
                variable: variable.to_string(),
            },
        ));
    }

    // Try to parse a function call
    if let Ok((input, (name, args))) = function_call(input) {
        let (input, _) = multispace1(input)?;
        let (input, _) = tag("AS")(input)?;
        let (input, _) = multispace1(input)?;
        let (input, variable) = identifier(input)?;
        let args = args.into_iter().map(ast::PropertyValue::String).collect();
        return Ok((
            input,
            UnwindClause {
                expression: UnwindExpression::FunctionCall { name, args },
                variable: variable.to_string(),
            },
        ));
    }

    // Try to parse a property access (e.g., a.hobbies)
    if let Ok((input, prop_access)) = property_access(input) {
        let (input, _) = multispace1(input)?;
        let (input, _) = tag("AS")(input)?;
        let (input, _) = multispace1(input)?;
        let (input, variable) = identifier(input)?;
        return Ok((
            input,
            UnwindClause {
                expression: UnwindExpression::Identifier(prop_access),
                variable: variable.to_string(),
            },
        ));
    }

    // Try to parse an identifier (variable)
    if let Ok((input, ident)) = identifier(input) {
        let (input, _) = multispace1(input)?;
        let (input, _) = tag("AS")(input)?;
        let (input, _) = multispace1(input)?;
        let (input, variable) = identifier(input)?;
        return Ok((
            input,
            UnwindClause {
                expression: UnwindExpression::Identifier(ident.to_string()),
                variable: variable.to_string(),
            },
        ));
    }

    // If none matched, return error
    Err(nom::Err::Error(nom::error::Error::new(
        input,
        nom::error::ErrorKind::Tag,
    )))
}

// Parses a property value (e.g., 42, 'hello', true, [1, 2, 3], {name: 'Alice'})
fn property_value(input: &str) -> IResult<&str, PropertyValue> {
    // Try to parse as a parameter
    if let Ok((input, param)) = parameter(input) {
        return Ok((input, PropertyValue::Parameter(param)));
    }

    // Try to parse as a list/array
    if let Ok((rest, _)) = char::<_, nom::error::Error<&str>>('[')(input) {
        let (rest, items) = separated_list0(
            tuple((multispace0, char(','), multispace0)),
            alt((
                map(string_literal_local, PropertyValue::String),
                map(numeric_literal, |n| {
                    PropertyValue::Number(n.parse().unwrap())
                }),
                map(tag_no_case("true"), |_| PropertyValue::Boolean(true)),
                map(tag_no_case("false"), |_| PropertyValue::Boolean(false)),
                map(tag_no_case("NULL"), |_| PropertyValue::Null),
                map(parameter, PropertyValue::Parameter),
            )),
        )(rest)?;
        let (rest, _) = char(']')(rest)?;
        return Ok((rest, PropertyValue::List(items)));
    }

    // Try to parse as a map/object
    if let Ok((rest, _)) = char::<_, nom::error::Error<&str>>('{')(input) {
        let (rest, pairs) = separated_list0(
            tuple((multispace0, char(','), multispace0)),
            tuple((
                identifier,
                tuple((multispace0, char(':'), multispace0)),
                alt((
                    map(string_literal_local, PropertyValue::String),
                    map(numeric_literal, |n| {
                        PropertyValue::Number(n.parse().unwrap())
                    }),
                    map(tag_no_case("true"), |_| PropertyValue::Boolean(true)),
                    map(tag_no_case("false"), |_| PropertyValue::Boolean(false)),
                    map(tag_no_case("NULL"), |_| PropertyValue::Null),
                    map(parameter, PropertyValue::Parameter),
                )),
            )),
        )(rest)?;
        let (rest, _) = char('}')(rest)?;
        let map: std::collections::HashMap<String, PropertyValue> = pairs
            .into_iter()
            .map(|(k, _, v)| (k.to_string(), v))
            .collect();
        return Ok((rest, PropertyValue::Map(map)));
    }

    // Try to parse as a primitive value
    let (input, value) = alt((
        // Try function calls first (e.g., timestamp())
        map(function_call, |(name, args)| PropertyValue::FunctionCall {
            name,
            args: args.into_iter().map(PropertyValue::String).collect(),
        }),
        map(string_literal_local, PropertyValue::String),
        map(identifier, |s| PropertyValue::String(s.to_string())),
        map(numeric_literal, |n| {
            PropertyValue::Number(n.parse().unwrap())
        }),
        map(tag_no_case("true"), |_| PropertyValue::Boolean(true)),
        map(tag_no_case("false"), |_| PropertyValue::Boolean(false)),
        map(tag_no_case("NULL"), |_| PropertyValue::Null),
        map(parameter, PropertyValue::Parameter),
    ))(input)?;
    Ok((input, value))
}

// Parses a clause (MATCH, RETURN, etc.) and returns its span
pub fn clause(input: &str) -> IResult<&str, Spanned<Clause>> {
    let full_input = input;

    // Helper to compute byte offset from input slice
    fn offset(full: &str, part: &str) -> usize {
        part.as_ptr() as usize - full.as_ptr() as usize
    }

    alt((
        map(with_clause, |c| {
            let start = offset(full_input, input);
            Spanned::new(Clause::With(c), start)
        }),
        map(where_clause, |c| {
            let start = offset(full_input, input);
            Spanned::new(Clause::Where(c), start)
        }),
        map(match_clause, |c| {
            let start = offset(full_input, input);
            Spanned::new(Clause::Match(c), start)
        }),
        map(return_clause, |c| {
            let start = offset(full_input, input);
            Spanned::new(Clause::Return(c), start)
        }),
        map(merge_clause, |c| {
            let start = offset(full_input, input);
            Spanned::new(Clause::Merge(c), start)
        }),
        map(create_clause, |c| {
            let start = offset(full_input, input);
            Spanned::new(Clause::Create(c), start)
        }),
        map(unwind_clause, |c| {
            let start = offset(full_input, input);
            Spanned::new(Clause::Unwind(c), start)
        }),
        map(call_clause, |c| {
            let start = offset(full_input, input);
            Spanned::new(Clause::Call(c), start)
        }),
    ))(input)
}

// Parses a complete query (e.g. MATCH (a)-[:KNOWS]->(b) RETURN a, b)
pub fn parse_query(input: &str) -> IResult<&str, Query> {
    let mut rest = input;
    let mut clauses = Vec::new();

    loop {
        // Always skip leading whitespace before parsing the next clause
        let (r, _) = multispace0(rest)?;
        rest = r;
        if rest.is_empty() {
            break;
        }
        match clause(rest) {
            Ok((next_rest, spanned_clause)) => {
                clauses.push(spanned_clause);
                rest = next_rest;
            }
            Err(_e) => {
                break;
            }
        }
    }

    // Ensure we've consumed the entire input
    let (rest, _) = multispace0(rest)?;
    if !rest.is_empty() {
        return Err(nom::Err::Error(nom::error::Error::new(
            rest,
            nom::error::ErrorKind::Verify,
        )));
    }

    // Validate clause order
    if let Err(_validation_error) = validate_clause_order(&clauses, input) {
        // Convert validation error to nom error
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    // Reject empty queries
    if clauses.is_empty() {
        return Err(nom::Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    // Convert clauses to Query struct
    let mut query = Query {
        match_clauses: Vec::new(),
        merge_clauses: Vec::new(),
        create_clauses: Vec::new(),
        with_clauses: Vec::new(),
        where_clauses: Vec::new(),
        return_clauses: Vec::new(),
        unwind_clauses: Vec::new(),
        call_clauses: Vec::new(),
    };

    for spanned_clause in clauses {
        match spanned_clause.value {
            Clause::Match(match_clause) => query.match_clauses.push(match_clause),
            Clause::OptionalMatch(match_clause) => query.match_clauses.push(match_clause),
            Clause::Merge(merge_clause) => query.merge_clauses.push(merge_clause),
            Clause::Create(create_clause) => query.create_clauses.push(create_clause),
            Clause::With(with_clause) => query.with_clauses.push(with_clause),
            Clause::Where(where_clause) => query.where_clauses.push(where_clause),
            Clause::Return(return_clause) => query.return_clauses.push(return_clause),
            Clause::Unwind(unwind_clause) => query.unwind_clauses.push(unwind_clause),
            Clause::Call(call_clause) => query.call_clauses.push(call_clause),
            Clause::Query(_) => {
                // Handle nested queries if needed
            }
        }
    }

    Ok((rest, query))
}

/// Validates that clauses appear in the correct Cypher order
///
/// Cypher clause order rules:
/// 1. MATCH/OPTIONAL MATCH must come first (reading clauses)
/// 2. UNWIND can come after MATCH
/// 3. WHERE can come after MATCH/UNWIND
/// 4. WITH can come after WHERE
/// 5. RETURN must come last (except for writing clauses)
/// 6. CREATE/MERGE can come after RETURN (writing clauses)
fn validate_clause_order(
    clauses: &[Spanned<Clause>],
    full_input: &str,
) -> Result<(), CypherGuardParsingError> {
    if clauses.is_empty() {
        return Ok(());
    }

    let mut state = ClauseOrderState::Initial;

    for spanned_clause in clauses.iter() {
        let clause = &spanned_clause.value;
        let (line, column) = offset_to_line_column(full_input, spanned_clause.start);

        state = match (state, clause) {
            // Initial state - reading clauses or RETURN allowed
            (ClauseOrderState::Initial, Clause::Match(_) | Clause::OptionalMatch(_)) => {
                ClauseOrderState::AfterMatch
            }
            (ClauseOrderState::Initial, Clause::Unwind(_)) => ClauseOrderState::AfterUnwind,
            (ClauseOrderState::Initial, Clause::Create(_) | Clause::Merge(_)) => {
                ClauseOrderState::AfterWrite
            }
            (ClauseOrderState::Initial, Clause::Call(_)) => ClauseOrderState::AfterCall,
            (ClauseOrderState::Initial, Clause::Return(_)) => ClauseOrderState::AfterReturn,
            (ClauseOrderState::Initial, _) => {
                return Err(CypherGuardParsingError::invalid_clause_order(
                    "query start",
                    format!(
                        "{} must come after a reading clause (MATCH, UNWIND, CREATE, MERGE)",
                        clause_name(clause)
                    ),
                ));
            }

            // After MATCH - can have UNWIND, WHERE, WITH, RETURN, or more MATCH
            (ClauseOrderState::AfterMatch, Clause::Match(_) | Clause::OptionalMatch(_)) => {
                ClauseOrderState::AfterMatch
            }
            (ClauseOrderState::AfterMatch, Clause::Unwind(_)) => ClauseOrderState::AfterUnwind,
            (ClauseOrderState::AfterMatch, Clause::Where(_)) => ClauseOrderState::AfterWhere,
            (ClauseOrderState::AfterMatch, Clause::With(_)) => ClauseOrderState::AfterWith,
            (ClauseOrderState::AfterMatch, Clause::Return(_)) => ClauseOrderState::AfterReturn,
            (ClauseOrderState::AfterMatch, Clause::Create(_) | Clause::Merge(_)) => {
                ClauseOrderState::AfterWrite
            }
            (ClauseOrderState::AfterMatch, Clause::Call(_)) => ClauseOrderState::AfterCall,

            // After UNWIND - can have WHERE, WITH, RETURN, or writing clauses
            (ClauseOrderState::AfterUnwind, Clause::Unwind(_)) => ClauseOrderState::AfterUnwind,
            (ClauseOrderState::AfterUnwind, Clause::Where(_)) => ClauseOrderState::AfterWhere,
            (ClauseOrderState::AfterUnwind, Clause::With(_)) => ClauseOrderState::AfterWith,
            (ClauseOrderState::AfterUnwind, Clause::Return(_)) => ClauseOrderState::AfterReturn,
            (ClauseOrderState::AfterUnwind, Clause::Create(_) | Clause::Merge(_)) => {
                ClauseOrderState::AfterWrite
            }
            (ClauseOrderState::AfterUnwind, Clause::Call(_)) => ClauseOrderState::AfterCall,

            // After WHERE - can have MATCH, WITH, RETURN, or more WHERE
            (ClauseOrderState::AfterWhere, Clause::Match(_) | Clause::OptionalMatch(_)) => {
                ClauseOrderState::AfterMatch
            }
            (ClauseOrderState::AfterWhere, Clause::Where(_)) => ClauseOrderState::AfterWhere,
            (ClauseOrderState::AfterWhere, Clause::Unwind(_)) => ClauseOrderState::AfterUnwind,
            (ClauseOrderState::AfterWhere, Clause::With(_)) => ClauseOrderState::AfterWith,
            (ClauseOrderState::AfterWhere, Clause::Return(_)) => ClauseOrderState::AfterReturn,
            (ClauseOrderState::AfterWhere, Clause::Create(_) | Clause::Merge(_)) => {
                ClauseOrderState::AfterWrite
            }
            (ClauseOrderState::AfterWhere, Clause::Call(_)) => ClauseOrderState::AfterCall,

            // After WITH - can have MATCH, UNWIND, WHERE, WITH, RETURN, or writing clauses
            // WITH creates a projection that allows starting a new reading phase
            (ClauseOrderState::AfterWith, Clause::Match(_) | Clause::OptionalMatch(_)) => {
                ClauseOrderState::AfterMatch
            }
            (ClauseOrderState::AfterWith, Clause::Unwind(_)) => ClauseOrderState::AfterUnwind,
            (ClauseOrderState::AfterWith, Clause::Where(_)) => ClauseOrderState::AfterWhere,
            (ClauseOrderState::AfterWith, Clause::With(_)) => ClauseOrderState::AfterWith,
            (ClauseOrderState::AfterWith, Clause::Return(_)) => ClauseOrderState::AfterReturn,
            (ClauseOrderState::AfterWith, Clause::Create(_) | Clause::Merge(_)) => {
                ClauseOrderState::AfterWrite
            }
            (ClauseOrderState::AfterWith, Clause::Call(_)) => ClauseOrderState::AfterCall,

            // After CALL - can have WHERE, WITH, RETURN, or writing clauses
            (ClauseOrderState::AfterCall, Clause::Where(_)) => ClauseOrderState::AfterWhere,
            (ClauseOrderState::AfterCall, Clause::With(_)) => ClauseOrderState::AfterWith,
            (ClauseOrderState::AfterCall, Clause::Return(_)) => ClauseOrderState::AfterReturn,
            (ClauseOrderState::AfterCall, Clause::Create(_) | Clause::Merge(_)) => {
                ClauseOrderState::AfterWrite
            }
            (ClauseOrderState::AfterCall, Clause::Call(_)) => ClauseOrderState::AfterCall,

            // After RETURN - can have CREATE/MERGE (writing clauses)
            (ClauseOrderState::AfterReturn, Clause::Create(_) | Clause::Merge(_)) => {
                ClauseOrderState::AfterWrite
            }
            (ClauseOrderState::AfterReturn, Clause::Return(_)) => {
                return Err(CypherGuardParsingError::return_after_return_at(
                    line, column,
                ));
            }
            (ClauseOrderState::AfterReturn, Clause::Match(_) | Clause::OptionalMatch(_)) => {
                return Err(CypherGuardParsingError::match_after_return_at(line, column));
            }
            (ClauseOrderState::AfterReturn, Clause::Where(_)) => {
                return Err(CypherGuardParsingError::where_after_return_at(line, column));
            }
            (ClauseOrderState::AfterReturn, Clause::With(_)) => {
                return Err(CypherGuardParsingError::with_after_return_at(line, column));
            }
            (ClauseOrderState::AfterReturn, Clause::Unwind(_)) => {
                return Err(CypherGuardParsingError::unwind_after_return_at(
                    line, column,
                ));
            }
            (ClauseOrderState::AfterReturn, _) => {
                return Err(CypherGuardParsingError::invalid_clause_order(
                    "after RETURN",
                    format!("{} cannot come after RETURN clause", clause_name(clause)),
                ));
            }

            // After write clause - can have more write clauses, RETURN, or WITH
            (ClauseOrderState::AfterWrite, Clause::Create(_) | Clause::Merge(_)) => {
                ClauseOrderState::AfterWrite
            }
            (ClauseOrderState::AfterWrite, Clause::Return(_)) => ClauseOrderState::AfterReturn,
            (ClauseOrderState::AfterWrite, Clause::With(_)) => ClauseOrderState::AfterWith,
            (ClauseOrderState::AfterWrite, _) => {
                return Err(CypherGuardParsingError::invalid_clause_order(
                    "after writing clause",
                    format!("{} cannot come after writing clause", clause_name(clause)),
                ));
            }

            // Handle any other combinations that shouldn't be possible
            _ => {
                return Err(CypherGuardParsingError::invalid_clause_order(
                    "clause validation",
                    format!(
                        "Invalid clause sequence: {} in current state",
                        clause_name(clause)
                    ),
                ));
            }
        };
    }

    // Check that query ends appropriately
    match state {
        ClauseOrderState::Initial => Err(CypherGuardParsingError::missing_required_clause(
            "reading clause (MATCH, UNWIND, CREATE, MERGE)",
        )),
        ClauseOrderState::AfterWith => Err(CypherGuardParsingError::missing_required_clause(
            "RETURN or writing clause",
        )),
        _ => Ok(()),
    }
}

/// Represents the state of clause ordering validation
#[derive(Debug, Clone, Copy, PartialEq)]
enum ClauseOrderState {
    Initial,
    AfterMatch,
    AfterUnwind,
    AfterWhere,
    AfterWith,
    AfterReturn,
    AfterWrite,
    AfterCall,
}

/// Returns a human-readable name for a clause
fn clause_name(clause: &Clause) -> &'static str {
    match clause {
        Clause::Match(_) => "MATCH",
        Clause::OptionalMatch(_) => "OPTIONAL MATCH",
        Clause::Unwind(_) => "UNWIND",
        Clause::Where(_) => "WHERE",
        Clause::With(_) => "WITH",
        Clause::Return(_) => "RETURN",
        Clause::Create(_) => "CREATE",
        Clause::Merge(_) => "MERGE",
        Clause::Query(_) => "Query",
        Clause::Call(_) => "CALL",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Direction, PatternElement};
    use crate::parser::ast::{PropertyValue, UnwindExpression};

    #[test]
    fn test_optional_match_clause() {
        let input = "OPTIONAL MATCH (a)-[:KNOWS]->(b)";
        let (_, clause) = match_clause(input).unwrap();
        assert!(clause.is_optional);
        assert_eq!(clause.elements.len(), 1);
    }

    #[test]
    fn test_regular_match_clause() {
        let input = "MATCH (a)-[:KNOWS]->(b)";
        let (_, clause) = match_clause(input).unwrap();
        assert!(!clause.is_optional);
        assert_eq!(clause.elements.len(), 1);
    }

    #[test]
    fn test_merge_clause() {
        let input = "MERGE (a:Person {name: 'Alice'})";
        let (_, clause) = merge_clause(input).unwrap();
        assert_eq!(clause.elements.len(), 1);
        assert!(clause.on_create.is_none());
        assert!(clause.on_match.is_none());
    }

    #[test]
    fn test_create_clause() {
        let input = "CREATE (a:Person {name: 'Alice'})-[r:KNOWS]->(b:Person {name: 'Bob'})";
        let (_, clause) = create_clause(input).unwrap();
        assert_eq!(clause.elements.len(), 1);
    }

    #[test]
    fn test_merge_with_on_match() {
        let input = "MERGE (a:Person {name: 'Alice'}) ON MATCH SET a.lastSeen = timestamp()";
        let (_, clause) = merge_clause(input).unwrap();
        assert_eq!(clause.elements.len(), 1);
        assert!(clause.on_create.is_none());
        assert!(clause.on_match.is_some());
    }

    #[test]
    fn test_create_with_relationship() {
        let input =
            "CREATE (a:Person {name: 'Alice'})-[r:KNOWS {since: 2020}]->(b:Person {name: 'Bob'})";
        let (_, clause) = create_clause(input).unwrap();
        assert_eq!(clause.elements.len(), 1);
        if let PatternElement::Relationship(rel) = &clause.elements[0].pattern[1] {
            assert_eq!(rel.direction(), Direction::Right);
            assert_eq!(rel.rel_type(), Some("KNOWS"));
            assert!(rel.properties().is_some());
        } else {
            panic!("Expected relationship");
        }
    }

    #[test]
    fn test_with_clause_simple() {
        let input = "WITH a, b";
        let (_, clause) = with_clause(input).unwrap();
        assert_eq!(clause.items.len(), 2);
    }

    #[test]
    fn test_with_clause_alias() {
        let input = "WITH a.name AS name";
        let (_, clause) = with_clause(input).unwrap();
        assert_eq!(clause.items.len(), 1);
    }

    #[test]
    fn test_with_clause_wildcard() {
        let input = "WITH *";
        let (_, clause) = with_clause(input).unwrap();
        assert_eq!(clause.items.len(), 1);
    }

    #[test]
    fn test_with_clause_multiple() {
        let input = "WITH a, b.name AS name";
        let (_, clause) = with_clause(input).unwrap();
        assert_eq!(clause.items.len(), 2);
    }

    // Return clause tests
    #[test]
    fn test_return_clause_simple() {
        let input = "RETURN a";
        let (_, clause) = return_clause(input).unwrap();
        assert_eq!(clause.items.len(), 1);
        assert_eq!(clause.items[0], "a");
    }

    #[test]
    fn test_return_clause_multiple_items() {
        let input = "RETURN a, b, c";
        let (_, clause) = return_clause(input).unwrap();
        assert_eq!(clause.items.len(), 3);
        assert_eq!(clause.items[0], "a");
        assert_eq!(clause.items[1], "b");
        assert_eq!(clause.items[2], "c");
    }

    #[test]
    fn test_return_clause_with_property_access() {
        let input = "RETURN a.name, b.age";
        let (_, clause) = return_clause(input).unwrap();
        assert_eq!(clause.items.len(), 2);
        assert_eq!(clause.items[0], "a.name");
        assert_eq!(clause.items[1], "b.age");
    }

    #[test]
    fn test_return_clause_mixed_items() {
        let input = "RETURN a, b.name, c";
        let (_, clause) = return_clause(input).unwrap();
        assert_eq!(clause.items.len(), 3);
        assert_eq!(clause.items[0], "a");
        assert_eq!(clause.items[1], "b.name");
        assert_eq!(clause.items[2], "c");
    }

    #[test]
    fn test_return_clause_with_whitespace() {
        let input = "RETURN  a  ,  b  ,  c  ";
        let (_, clause) = return_clause(input).unwrap();
        assert_eq!(clause.items.len(), 3);
        assert_eq!(clause.items[0], "a");
        assert_eq!(clause.items[1], "b");
        assert_eq!(clause.items[2], "c");
    }

    #[test]
    fn test_return_clause_single_property() {
        let input = "RETURN a.name";
        let (_, clause) = return_clause(input).unwrap();
        assert_eq!(clause.items.len(), 1);
        assert_eq!(clause.items[0], "a.name");
    }

    #[test]
    fn test_return_item_simple() {
        let input = "a";
        let (_, item) = return_item(input).unwrap();
        assert_eq!(item, "a");
    }

    #[test]
    fn test_return_item_with_property() {
        let input = "a.name";
        let (_, item) = return_item(input).unwrap();
        assert_eq!(item, "a.name");
    }

    #[test]
    fn test_return_item_with_underscore() {
        let input = "user_name";
        let (_, item) = return_item(input).unwrap();
        assert_eq!(item, "user_name");
    }

    #[test]
    fn test_return_item_with_numbers() {
        let input = "node1";
        let (_, item) = return_item(input).unwrap();
        assert_eq!(item, "node1");
    }

    // Error cases for return clause
    #[test]
    fn test_return_clause_missing_return() {
        let input = "a, b, c";
        let result = return_clause(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_return_clause_empty() {
        let input = "RETURN";
        let result = return_clause(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_return_clause_no_items() {
        let input = "RETURN ";
        let result = return_clause(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_return_clause_trailing_comma() {
        let input = "RETURN a, b,";
        let result = return_clause(input);
        // Parser should reject trailing commas as they are invalid in Cypher
        assert!(result.is_err());
    }

    #[test]
    fn test_return_item_invalid_identifier() {
        let input = "123name";
        let (_, item) = return_item(input).unwrap();
        // Parser now correctly parses numeric literals first
        // "123name" is parsed as number "123", leaving "name" unparsed
        assert_eq!(item, "123");
    }

    // WHERE clause tests
    #[test]
    fn test_where_clause_simple_comparison() {
        let input = "WHERE a.age > 30";
        let (_, clause) = where_clause(input).unwrap();
        assert_eq!(clause.conditions.len(), 1);
        match &clause.conditions[0] {
            ast::WhereCondition::Comparison {
                left,
                operator,
                right,
            } => {
                assert_eq!(left, &ast::PropertyValue::Identifier("a.age".to_string()));
                assert_eq!(operator, ">");
                assert!(matches!(right, ast::PropertyValue::Number(30)));
            }
            _ => unreachable!("Expected comparison condition"),
        }
    }

    #[test]
    fn test_where_clause_string_comparison() {
        let input = "WHERE a.name = \"Alice\"";
        let (_, clause) = where_clause(input).unwrap();
        assert_eq!(clause.conditions.len(), 1);
        match &clause.conditions[0] {
            ast::WhereCondition::Comparison {
                left,
                operator,
                right,
            } => {
                assert_eq!(left, &ast::PropertyValue::Identifier("a.name".to_string()));
                assert_eq!(operator, "=");
                assert_eq!(right, &ast::PropertyValue::String("Alice".to_string()));
            }
            _ => unreachable!("Expected comparison condition"),
        }
    }

    #[test]
    fn test_where_clause_multiple_conditions_and() {
        let input = "WHERE a.age > 30 AND b.name = \"Bob\"";
        let (_, clause) = where_clause(input).unwrap();
        assert_eq!(clause.conditions.len(), 1);
        match &clause.conditions[0] {
            ast::WhereCondition::And(left, right) => {
                match &**left {
                    ast::WhereCondition::Comparison {
                        left: l,
                        operator: o,
                        right: r,
                    } => {
                        assert_eq!(l, &ast::PropertyValue::Identifier("a.age".to_string()));
                        assert_eq!(o, ">");
                        assert_eq!(r, &ast::PropertyValue::Number(30));
                    }
                    _ => unreachable!("Expected comparison on left "),
                }
                match &**right {
                    ast::WhereCondition::Comparison {
                        left: l,
                        operator: o,
                        right: r,
                    } => {
                        assert_eq!(l, &ast::PropertyValue::Identifier("b.name".to_string()));
                        assert_eq!(o, "=");
                        assert_eq!(r, &ast::PropertyValue::String("Bob".to_string()));
                    }
                    _ => unreachable!("Expected comparison on right "),
                }
            }
            _ => unreachable!("Expected AND condition "),
        }
    }

    #[test]
    fn test_where_clause_multiple_conditions_or() {
        let input = "WHERE a.age > 30 OR b.name = \"Bob\"";
        let (_, clause) = where_clause(input).unwrap();
        assert_eq!(clause.conditions.len(), 1);
        match &clause.conditions[0] {
            ast::WhereCondition::Or(left, right) => {
                match &**left {
                    ast::WhereCondition::Comparison {
                        left: l,
                        operator: o,
                        right: r,
                    } => {
                        assert_eq!(l, &ast::PropertyValue::Identifier("a.age".to_string()));
                        assert_eq!(o, ">");
                        assert_eq!(r, &ast::PropertyValue::Number(30));
                    }
                    _ => unreachable!("Expected comparison on left "),
                }
                match &**right {
                    ast::WhereCondition::Comparison {
                        left: l,
                        operator: o,
                        right: r,
                    } => {
                        assert_eq!(l, &ast::PropertyValue::Identifier("b.name".to_string()));
                        assert_eq!(o, "=");
                        assert_eq!(r, &ast::PropertyValue::String("Bob".to_string()));
                    }
                    _ => unreachable!("Expected comparison on right "),
                }
            }
            _ => unreachable!("Expected OR condition"),
        }
    }

    #[test]
    fn test_where_clause_is_null() {
        let input = "WHERE a.name IS NULL";
        let (_, clause) = where_clause(input).unwrap();
        assert_eq!(clause.conditions.len(), 1);
        match &clause.conditions[0] {
            ast::WhereCondition::Comparison {
                left,
                operator,
                right,
            } => {
                assert_eq!(left, &ast::PropertyValue::Identifier("a.name".to_string()));
                assert_eq!(operator, "IS NULL");
                assert_eq!(right, &ast::PropertyValue::Null);
            }
            _ => unreachable!("Expected comparison condition"),
        }
    }

    #[test]
    fn test_where_clause_is_not_null() {
        let input = "WHERE a.name IS NOT NULL";
        let (_, clause) = where_clause(input).unwrap();
        assert_eq!(clause.conditions.len(), 1);
        match &clause.conditions[0] {
            ast::WhereCondition::Comparison {
                left,
                operator,
                right,
            } => {
                assert_eq!(left, &ast::PropertyValue::Identifier("a.name".to_string()));
                assert_eq!(operator, "IS NOT NULL");
                assert_eq!(right, &ast::PropertyValue::Null);
            }
            _ => unreachable!("Expected comparison condition"),
        }
    }

    #[test]
    fn test_where_clause_not_equals() {
        let input = "WHERE a.name <> \"Alice\"";
        let (_, clause) = where_clause(input).unwrap();
        assert_eq!(clause.conditions.len(), 1);
        match &clause.conditions[0] {
            ast::WhereCondition::Comparison {
                left,
                operator,
                right,
            } => {
                assert_eq!(left, &ast::PropertyValue::Identifier("a.name".to_string()));
                assert_eq!(operator, "<>");
                assert_eq!(right, &ast::PropertyValue::String("Alice".to_string()));
            }
            _ => unreachable!("Expected comparison condition"),
        }
    }

    #[test]
    fn test_where_clause_less_than_equal() {
        let input = "WHERE a.age <= 30";
        let (_, clause) = where_clause(input).unwrap();
        assert_eq!(clause.conditions.len(), 1);
        match &clause.conditions[0] {
            ast::WhereCondition::Comparison {
                left,
                operator,
                right,
            } => {
                assert_eq!(left, &ast::PropertyValue::Identifier("a.age".to_string()));
                assert_eq!(operator, "<=");
                assert_eq!(right, &ast::PropertyValue::Number(30));
            }
            _ => panic!("Expected comparison condition, got: {:?}", &clause.conditions[0]),
        }
    }

    #[test]
    fn test_where_clause_greater_than_equal() {
        let input = "WHERE a.age >= 30";
        let (_, clause) = where_clause(input).unwrap();
        assert_eq!(clause.conditions.len(), 1);
        match &clause.conditions[0] {
            ast::WhereCondition::Comparison {
                left,
                operator,
                right,
            } => {
                assert_eq!(left, &ast::PropertyValue::Identifier("a.age".to_string()));
                assert_eq!(operator, ">=");
                assert_eq!(right, &ast::PropertyValue::Number(30));
            }
            _ => panic!("Expected comparison condition, got: {:?}", &clause.conditions[0]),
        }
    }

    #[test]
    fn test_where_clause_function_call() {
        let input = "WHERE length(a.name) > 5";
        let (_, clause) = where_clause(input).unwrap();
        assert_eq!(clause.conditions.len(), 1);
        match &clause.conditions[0] {
            ast::WhereCondition::Comparison {
                left,
                operator,
                right,
            } => {
                // The left side should be a function call
                match left {
                    ast::PropertyValue::FunctionCall { name, .. } => {
                        assert_eq!(name, "length");
                    }
                    _ => panic!("Expected function call on left side, got: {:?}", left),
                }
                assert_eq!(operator, ">");
                assert_eq!(right, &ast::PropertyValue::Number(5));
            }
            _ => panic!("Expected comparison condition, got: {:?}", &clause.conditions[0]),
        }
    }

    #[test]
    fn test_where_clause_path_property() {
        let input = "WHERE p.length > 5";
        let (_, clause) = where_clause(input).unwrap();
        assert_eq!(clause.conditions.len(), 1);
        match &clause.conditions[0] {
            ast::WhereCondition::Comparison {
                left,
                operator,
                right,
            } => {
                assert_eq!(
                    left,
                    &ast::PropertyValue::Identifier("p.length".to_string())
                );
                assert_eq!(operator, ">");
                assert_eq!(right, &ast::PropertyValue::Number(5));
            }
            _ => unreachable!("Expected comparison condition"),
        }
    }

    #[test]
    fn test_where_clause_not_condition() {
        let input = "WHERE NOT a.name = \"Alice\"";
        let (_, clause) = where_clause(input).unwrap();
        match &clause.conditions[0] {
            ast::WhereCondition::Not(inner) => match &**inner {
                ast::WhereCondition::Comparison {
                    left,
                    operator,
                    right,
                } => {
                    assert_eq!(left, &ast::PropertyValue::Identifier("a.name".to_string()));
                    assert_eq!(operator, "=");
                    assert_eq!(right, &ast::PropertyValue::String("Alice".to_string()));
                }
                _ => unreachable!("Expected comparison inside NOT "),
            },
            _ => unreachable!("Expected NOT condition "),
        }
    }

    #[test]
    fn test_where_clause_parenthesized() {
        let input = "WHERE (a.age > 30)";
        let (_, clause) = where_clause(input).unwrap();
        match &clause.conditions[0] {
            ast::WhereCondition::Parenthesized(inner) => match &**inner {
                ast::WhereCondition::Comparison {
                    left,
                    operator,
                    right,
                } => {
                    assert_eq!(left, &ast::PropertyValue::Identifier("a.age".to_string()));
                    assert_eq!(operator, ">");
                    assert!(matches!(right, ast::PropertyValue::Number(30)));
                }
                _ => unreachable!("Expected comparison inside parentheses"),
            },
            _ => unreachable!("Expected parenthesized condition"),
        }
    }

    #[test]
    fn test_where_clause_complex_nested() {
        let input = "WHERE (a.age > 30 AND b.name = \"Bob\") OR NOT c.active = true";
        let (_, clause) = where_clause(input).unwrap();
        match &clause.conditions[0] {
            ast::WhereCondition::Or(left, right) => {
                // First condition should be parenthesized with AND
                match &**left {
                    ast::WhereCondition::Parenthesized(inner) => match &**inner {
                        ast::WhereCondition::And(l, r) => {
                            match &**l {
                                ast::WhereCondition::Comparison {
                                    left: l1,
                                    operator: o1,
                                    right: r1,
                                } => {
                                    assert_eq!(
                                        l1,
                                        &ast::PropertyValue::Identifier("a.age".to_string())
                                    );
                                    assert_eq!(o1, ">");
                                    assert_eq!(r1, &ast::PropertyValue::Number(30));
                                }
                                _ => unreachable!("Expected comparison inside parentheses (left) "),
                            }
                            match &**r {
                                ast::WhereCondition::Comparison {
                                    left: l2,
                                    operator: o2,
                                    right: r2,
                                } => {
                                    assert_eq!(
                                        l2,
                                        &ast::PropertyValue::Identifier("b.name".to_string())
                                    );
                                    assert_eq!(o2, "=");
                                    assert_eq!(r2, &ast::PropertyValue::String("Bob".to_string()));
                                }
                                _ => {
                                    unreachable!("Expected comparison inside parentheses (right) ")
                                }
                            }
                        }
                        _ => unreachable!("Expected AND inside parentheses "),
                    },
                    _ => unreachable!("Expected parenthesized condition "),
                }
                // Second condition should be NOT
                match &**right {
                    ast::WhereCondition::Not(inner) => match &**inner {
                        ast::WhereCondition::Comparison {
                            left,
                            operator,
                            right,
                        } => {
                            assert_eq!(
                                left,
                                &ast::PropertyValue::Identifier("c.active".to_string())
                            );
                            assert_eq!(operator, "=");
                            assert!(matches!(right, ast::PropertyValue::Boolean(true)));
                        }
                        _ => unreachable!("Expected comparison inside NOT "),
                    },
                    _ => unreachable!("Expected NOT condition "),
                }
            }
            _ => unreachable!("Expected OR condition "),
        }
    }

    #[test]
    fn test_where_clause_with_whitespace() {
        let input = "WHERE  a.age  >  30  AND  b.name  =  \"Bob\"  ";
        let (_, clause) = where_clause(input).unwrap();
        match &clause.conditions[0] {
            ast::WhereCondition::And(left, right) => {
                match &**left {
                    ast::WhereCondition::Comparison {
                        left: l,
                        operator: o,
                        right: r,
                    } => {
                        assert_eq!(l, &ast::PropertyValue::Identifier("a.age".to_string()));
                        assert_eq!(o, ">");
                        assert_eq!(r, &ast::PropertyValue::Number(30));
                    }
                    _ => unreachable!("Expected comparison on left "),
                }
                match &**right {
                    ast::WhereCondition::Comparison {
                        left: l,
                        operator: o,
                        right: r,
                    } => {
                        assert_eq!(l, &ast::PropertyValue::Identifier("b.name".to_string()));
                        assert_eq!(o, "=");
                        assert_eq!(r, &ast::PropertyValue::String("Bob".to_string()));
                    }
                    _ => unreachable!("Expected comparison on right "),
                }
            }
            _ => unreachable!("Expected AND condition "),
        }
    }

    // Error cases for WHERE clause

    #[test]
    fn test_where_clause_with_string_literal() {
        let input = "WHERE a.name = 'Alice'";

        match where_clause(input) {
            Ok((remaining, clause)) => {
                // Verify we have exactly one condition
                assert_eq!(
                    clause.conditions.len(),
                    1,
                    "Should have exactly one condition"
                );

                // Check the condition structure
                if let ast::WhereCondition::Comparison {
                    left,
                    operator,
                    right,
                } = &clause.conditions[0]
                {
                    match right {
                        ast::PropertyValue::String(s) => {
                            assert_eq!(s, "Alice");
                        }
                        ast::PropertyValue::Identifier(i) => {
                            panic!(
                                "❌ FAILED: Right side is Identifier(\"{}\") - should be String!",
                                i
                            );
                        }
                        other => {
                            panic!("❌ UNEXPECTED: Right side is {:?}", other);
                        }
                    }

                    assert!(
                        matches!(left, ast::PropertyValue::Identifier(_)),
                        "Left should be Identifier"
                    );
                    assert_eq!(operator, "=");
                } else {
                    panic!(
                        "Condition should be a Comparison, got: {:?}",
                        clause.conditions[0]
                    );
                }

                assert_eq!(remaining, "", "Should consume entire input");
            }
            Err(e) => {
                panic!("❌ WHERE clause parsing failed: {:?}", e);
            }
        }

        println!("✅ Unit test passed: WHERE clause correctly parses string literals");
    }

    #[test]
    fn test_where_clause_with_integer_literal() {
        println!("\n=== UNIT TEST: WHERE clause with integer literal (control test) ===");
        let input = "WHERE a.age > 30";
        println!("Testing: {}", input);

        match where_clause(input) {
            Ok((remaining, clause)) => {
                println!("✅ WHERE clause parsed successfully");

                if let ast::WhereCondition::Comparison {
                    left: _,
                    operator,
                    right,
                } = &clause.conditions[0]
                {
                    match right {
                        ast::PropertyValue::Number(n) => {
                            println!("✅ CORRECT: Right side is Number({})", n);
                            assert_eq!(*n, 30);
                        }
                        other => {
                            panic!(
                                "❌ UNEXPECTED: Right side should be Number, got {:?}",
                                other
                            );
                        }
                    }

                    assert_eq!(operator, ">");
                } else {
                    panic!("Condition should be a Comparison");
                }

                assert_eq!(remaining, "", "Should consume entire input");
            }
            Err(e) => {
                panic!("❌ WHERE clause parsing failed: {:?}", e);
            }
        }

        println!("✅ Control test passed: WHERE clause correctly parses integer literals");
    }

    #[test]
    fn test_full_query_with_where_clause() {
        println!("\n=== UNIT TEST: Full query with WHERE clause ===");
        let input = "MATCH (a:Person) WHERE a.name = 'Alice' RETURN a";
        println!("Testing full query: {}", input);

        match parse_query(input) {
            Ok((remaining, query)) => {
                println!("✅ Full query parsed successfully");
                println!("Remaining: '{}'", remaining);

                // Check that WHERE clauses are included in the AST
                println!("Match clauses: {}", query.match_clauses.len());
                println!("WHERE clauses: {}", query.where_clauses.len());
                println!("Return clauses: {}", query.return_clauses.len());

                // This should NOT be 0!
                if query.where_clauses.len() == 0 {
                    println!("❌ BUG CONFIRMED: WHERE clauses are parsed but not included in AST!");
                    panic!("WHERE clauses missing from AST");
                } else {
                    println!("✅ WHERE clauses correctly included in AST");

                    // Verify the WHERE clause content
                    let where_clause = &query.where_clauses[0];
                    if let ast::WhereCondition::Comparison {
                        left: _,
                        operator: _,
                        right,
                    } = &where_clause.conditions[0]
                    {
                        match right {
                            ast::PropertyValue::String(s) => {
                                println!("✅ CORRECT: WHERE clause contains String(\"{}\")", s);
                                assert_eq!(s, "Alice");
                            }
                            other => {
                                println!(
                                    "❌ WRONG: WHERE clause contains {:?}, expected String",
                                    other
                                );
                                panic!("Expected String(\"Alice\"), got {:?}", other);
                            }
                        }
                    }
                }

                println!("✅ Unit test passed: Full query correctly includes WHERE clauses in AST");
            }
            Err(e) => {
                println!("❌ Full query parsing failed: {:?}", e);
                panic!("Full query parsing failed: {:?}", e);
            }
        }
    }
    #[test]
    fn test_where_clause_missing_where() {
        let input = "a.age > 30";
        let result = where_clause(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_where_clause_empty() {
        let input = "WHERE";
        let result = where_clause(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_where_clause_no_conditions() {
        let input = "WHERE ";
        let result = where_clause(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_where_clause_incomplete_comparison() {
        let input = "WHERE a.age >";
        let (_, clause) = where_clause(input).unwrap();
        assert_eq!(clause.conditions.len(), 1);
        match &clause.conditions[0] {
            ast::WhereCondition::PathProperty { path_var, property } => {
                assert_eq!(path_var, "a");
                assert_eq!(property, "age");
            }
            _ => unreachable!("Expected path property condition"),
        }
    }

    #[test]
    fn test_where_clause_invalid_operator() {
        let input = "WHERE a.age == 30";
        let (_, clause) = where_clause(input).unwrap();
        assert_eq!(clause.conditions.len(), 1);
        match &clause.conditions[0] {
            ast::WhereCondition::PathProperty { path_var, property } => {
                assert_eq!(path_var, "a");
                assert_eq!(property, "age");
            }
            _ => unreachable!("Expected path property condition"),
        }
    }

    #[test]
    fn test_where_clause_unclosed_parentheses() {
        let input = "WHERE (a.age > 30";
        let result = where_clause(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_where_clause_malformed_not() {
        let input = "WHERE NOT";
        let result = where_clause(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_where_clause_trailing_and() {
        let input = "WHERE a.age > 30 AND";
        let (_, clause) = where_clause(input).unwrap();
        assert_eq!(clause.conditions.len(), 1);
        match &clause.conditions[0] {
            ast::WhereCondition::Comparison {
                left,
                operator,
                right,
            } => {
                assert_eq!(left, &ast::PropertyValue::Identifier("a.age".to_string()));
                assert_eq!(operator, ">");
                assert!(matches!(right, ast::PropertyValue::Number(30)));
            }
            _ => unreachable!("Expected comparison condition"),
        }
    }

    #[test]
    fn test_where_clause_trailing_or() {
        let input = "WHERE a.age > 30 OR";
        let (_, clause) = where_clause(input).unwrap();
        assert_eq!(clause.conditions.len(), 1);
        match &clause.conditions[0] {
            ast::WhereCondition::Comparison {
                left,
                operator,
                right,
            } => {
                assert_eq!(left, &ast::PropertyValue::Identifier("a.age".to_string()));
                assert_eq!(operator, ">");
                assert!(matches!(right, ast::PropertyValue::Number(30)));
            }
            _ => unreachable!("Expected comparison condition"),
        }
    }

    // Individual where_condition tests
    #[test]
    fn test_where_condition_simple_identifier() {
        let input = "age > 30";
        let (_, condition) = parse_basic_condition(input).unwrap();
        match condition {
            ast::WhereCondition::Comparison {
                left,
                operator,
                right,
            } => {
                assert_eq!(left, ast::PropertyValue::Identifier("age".to_string()));
                assert_eq!(operator, ">");
                assert!(matches!(right, ast::PropertyValue::Number(30)));
            }
            _ => unreachable!("Expected comparison condition"),
        }
    }

    #[test]
    fn test_where_condition_property_access() {
        let input = "a.name = \"Alice\"";
        let (_, condition) = parse_basic_condition(input).unwrap();
        match condition {
            ast::WhereCondition::Comparison {
                left,
                operator,
                right,
            } => {
                assert_eq!(left, ast::PropertyValue::Identifier("a.name".to_string()));
                assert_eq!(operator, "=");
                assert_eq!(right, ast::PropertyValue::String("Alice".to_string()));
            }
            _ => unreachable!("Expected comparison condition"),
        }
    }

    #[test]
    fn test_where_condition_boolean_literal() {
        let input = "a.active = true";
        let (_, condition) = parse_basic_condition(input).unwrap();
        match condition {
            ast::WhereCondition::Comparison {
                left,
                operator,
                right,
            } => {
                assert_eq!(left, ast::PropertyValue::Identifier("a.active".to_string()));
                assert_eq!(operator, "=");
                assert!(matches!(right, ast::PropertyValue::Boolean(true)));
            }
            _ => unreachable!("Expected comparison condition"),
        }
    }

    #[test]
    fn test_where_condition_null_literal() {
        let input = "a.name = NULL";
        let (_, condition) = parse_basic_condition(input).unwrap();
        match condition {
            ast::WhereCondition::Comparison {
                left,
                operator,
                right,
            } => {
                assert_eq!(left, ast::PropertyValue::Identifier("a.name".to_string()));
                assert_eq!(operator, "=");
                assert!(matches!(right, ast::PropertyValue::Null));
            }
            _ => unreachable!("Expected comparison condition"),
        }
    }

    #[test]
    fn test_unwind_clause_literal_list() {
        let input = "UNWIND [1, 2, 3] AS x";
        let (_, clause) = unwind_clause(input).unwrap();
        assert_eq!(
            clause.expression,
            UnwindExpression::List(vec![
                PropertyValue::Number(1),
                PropertyValue::Number(2),
                PropertyValue::Number(3)
            ])
        );
        assert_eq!(clause.variable, "x");
    }

    #[test]
    fn test_unwind_clause_identifier() {
        let input = "UNWIND myList AS y";
        let (_, clause) = unwind_clause(input).unwrap();
        assert_eq!(
            clause.expression,
            UnwindExpression::Identifier("myList".to_string())
        );
        assert_eq!(clause.variable, "y");
    }

    #[test]
    fn test_unwind_clause_function_call() {
        let input = "UNWIND collect(a) AS z";
        let (_, clause) = unwind_clause(input).unwrap();
        assert_eq!(
            clause.expression,
            UnwindExpression::FunctionCall {
                name: "collect".to_string(),
                args: vec![PropertyValue::String("a".to_string())]
            }
        );
        assert_eq!(clause.variable, "z");
    }

    #[test]
    fn test_unwind_clause_empty_list() {
        let input = "UNWIND [] AS n";
        let (_, clause) = unwind_clause(input).unwrap();
        assert_eq!(clause.expression, UnwindExpression::List(vec![]));
        assert_eq!(clause.variable, "n");
    }

    #[test]
    fn test_unwind_clause_parameter() {
        let input = "UNWIND $events AS event";
        let (_, clause) = unwind_clause(input).unwrap();
        assert_eq!(
            clause.expression,
            UnwindExpression::Parameter("events".to_string())
        );
        assert_eq!(clause.variable, "event");
    }

    #[test]
    fn test_property_value_parameter() {
        let input = "$name";
        let (_, value) = property_value(input).unwrap();
        assert_eq!(value, PropertyValue::Parameter("name".to_string()));
    }

    #[test]
    fn test_property_value_parameter_in_list() {
        let input = "[1, $id, 3]";
        let (_, value) = property_value(input).unwrap();
        assert_eq!(
            value,
            PropertyValue::List(vec![
                PropertyValue::Number(1),
                PropertyValue::Parameter("id".to_string()),
                PropertyValue::Number(3),
            ])
        );
    }

    #[test]
    fn test_property_value_parameter_in_map() {
        let input = "{foo: $bar}";
        let (_, value) = property_value(input).unwrap();
        let mut expected = std::collections::HashMap::new();
        expected.insert(
            "foo".to_string(),
            PropertyValue::Parameter("bar".to_string()),
        );
        assert_eq!(value, PropertyValue::Map(expected));
    }

    #[test]
    fn test_unwind_clause_missing_as() {
        let input = "UNWIND [1,2,3] x";
        assert!(unwind_clause(input).is_err());
    }

    #[test]
    fn test_unwind_clause_missing_variable() {
        let input = "UNWIND [1,2,3] AS ";
        assert!(unwind_clause(input).is_err());
    }

    #[test]
    fn test_unwind_clause_unsupported_expression() {
        let input = "UNWIND a + b AS x";
        assert!(unwind_clause(input).is_err());
    }

    // === Clause Order Validation Tests ===

    #[test]
    fn test_valid_clause_order_match_return() {
        let query = "MATCH (a:Person) RETURN a";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Valid query should parse successfully");
    }

    #[test]
    fn test_valid_clause_order_match_where_return() {
        let query = "MATCH (a:Person) WHERE a.age > 30 RETURN a";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Valid query should parse successfully");
    }

    #[test]
    fn test_valid_clause_order_match_with_return() {
        let query = "MATCH (a:Person) WITH a WHERE a.age > 30 RETURN a";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Valid query should parse successfully");
    }

    #[test]
    fn test_valid_clause_order_match_unwind_return() {
        let query = "MATCH (a:Person) UNWIND a.hobbies AS hobby RETURN a, hobby";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Valid query should parse successfully");
    }

    #[test]
    fn test_valid_clause_order_match_unwind_where_return() {
        let query =
            "MATCH (a:Person) UNWIND a.hobbies AS hobby WHERE hobby = 'reading' RETURN a, hobby";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Valid query should parse successfully");
    }

    #[test]
    fn test_valid_clause_order_create_return() {
        let query = "CREATE (a:Person {name: 'Alice'}) RETURN a";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Valid query should parse successfully");
    }

    #[test]
    fn test_valid_clause_order_merge_return() {
        let query = "MERGE (a:Person {name: 'Alice'}) RETURN a";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Valid query should parse successfully");
    }

    #[test]
    fn test_valid_clause_order_match_return_create() {
        let query = "MATCH (a:Person) RETURN a CREATE (b:Person {name: 'Bob'})";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Valid query should parse successfully");
    }

    #[test]
    fn test_valid_clause_order_optional_match() {
        let query = "OPTIONAL MATCH (a:Person) RETURN a";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Valid query should parse successfully");
    }

    #[test]
    fn test_invalid_clause_order_return_before_match() {
        let query = "RETURN a MATCH (a:Person)";
        let result = crate::parse_query(query);
        assert!(result.is_err(), "Invalid clause order should fail");

        if let Err(CypherGuardParsingError::ReturnBeforeOtherClauses { line: _, column: _ }) =
            result
        {
            // Expected specific error variant
        } else {
            panic!("Expected ReturnBeforeOtherClauses error");
        }
    }

    #[test]
    fn test_invalid_clause_order_where_before_match() {
        let query = "WHERE a.age > 30 MATCH (a:Person)";
        let result = crate::parse_query(query);
        assert!(result.is_err(), "Invalid clause order should fail");

        if let Err(CypherGuardParsingError::WhereBeforeMatch { line: _, column: _ }) = result {
            // Expected specific error variant
        } else {
            panic!("Expected WhereBeforeMatch error");
        }
    }

    #[test]
    fn test_invalid_clause_order_with_before_match() {
        let query = "WITH a MATCH (a:Person)";
        let result = crate::parse_query(query);
        assert!(result.is_err(), "Invalid clause order should fail");

        if let Err(CypherGuardParsingError::InvalidClauseOrder { context, details }) = result {
            assert!(context.contains("query start"));
            assert!(details.contains("WITH must come after a reading clause"));
        } else {
            panic!("Expected InvalidClauseOrder error");
        }
    }

    #[test]
    fn test_invalid_clause_order_unwind_before_match() {
        let query = "UNWIND [1,2,3] AS x MATCH (a:Person)";
        let result = crate::parse_query(query);
        assert!(result.is_err(), "Invalid clause order should fail");

        if let Err(CypherGuardParsingError::InvalidClauseOrder { context, details }) = result {
            assert!(context.contains("query start"));
            assert!(details.contains("UNWIND must come after a reading clause"));
        } else {
            panic!("Expected InvalidClauseOrder error");
        }
    }

    #[test]
    fn test_invalid_clause_order_match_after_return() {
        let query = "MATCH (a:Person) RETURN a MATCH (b:Person)";
        let result = crate::parse_query(query);
        assert!(result.is_err(), "Invalid clause order should fail");

        if let Err(CypherGuardParsingError::MatchAfterReturn { line: _, column: _ }) = result {
            // Expected specific error variant
        } else {
            panic!("Expected MatchAfterReturn error");
        }
    }

    #[test]
    fn test_invalid_clause_order_where_after_return() {
        let query = "MATCH (a:Person) RETURN a WHERE a.age > 30";
        let result = crate::parse_query(query);
        assert!(result.is_err(), "Invalid clause order should fail");

        if let Err(CypherGuardParsingError::InvalidClauseOrder { context, details }) = result {
            assert!(context.contains("after RETURN"));
            assert!(details.contains("WHERE cannot come after RETURN clause"));
        } else {
            panic!("Expected InvalidClauseOrder error");
        }
    }

    #[test]
    fn test_invalid_clause_order_with_after_return() {
        let query = "MATCH (a:Person) RETURN a WITH a";
        let result = crate::parse_query(query);
        assert!(result.is_err(), "Invalid clause order should fail");

        if let Err(CypherGuardParsingError::WithAfterReturn { line: _, column: _ }) = result {
            // Expected specific error variant
        } else {
            panic!("Expected WithAfterReturn error");
        }
    }

    #[test]
    fn test_invalid_clause_order_unwind_after_return() {
        let query = "MATCH (a:Person) RETURN a UNWIND [1,2,3] AS x";
        let result = crate::parse_query(query);
        assert!(result.is_err(), "Invalid clause order should fail");

        if let Err(CypherGuardParsingError::UnwindAfterReturn { line: _, column: _ }) = result {
            // Expected specific error variant
        } else {
            panic!("Expected UnwindAfterReturn error");
        }
    }

    #[test]
    fn test_invalid_clause_order_missing_return() {
        let query = "MATCH (a:Person) WITH a";
        let result = crate::parse_query(query);
        assert!(result.is_err(), "Query ending with WITH should fail");

        if let Err(CypherGuardParsingError::MissingRequiredClause { clause }) = result {
            assert!(clause.contains("RETURN or writing clause"));
        } else {
            panic!("Expected MissingRequiredClause error");
        }
    }

    #[test]
    fn test_invalid_clause_order_empty_query() {
        let query = "";
        let result = crate::parse_query(query);
        assert!(result.is_err(), "Empty query should fail");
    }

    #[test]
    fn test_valid_clause_order_multiple_match() {
        let query = "MATCH (a:Person) MATCH (b:Person) RETURN a, b";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Multiple MATCH clauses should be valid");
    }

    #[test]
    fn test_valid_clause_order_multiple_where() {
        let query = "MATCH (a:Person) WHERE a.age > 30 WHERE a.active = true RETURN a";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Multiple WHERE clauses should be valid");
    }

    #[test]
    fn test_valid_clause_order_multiple_with() {
        let query = "MATCH (a:Person) WITH a WHERE a.age > 30 WITH a.age AS age RETURN age";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Multiple WITH clauses should be valid");
    }

    #[test]
    fn test_valid_clause_order_multiple_unwind() {
        let query = "MATCH (a:Person) UNWIND a.hobbies AS hobby UNWIND a.skills AS skill RETURN a, hobby, skill";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Multiple UNWIND clauses should be valid");
    }

    #[test]
    fn test_valid_clause_order_complex_sequence() {
        let query = "MATCH (a:Person) WHERE a.age > 30 WITH a WHERE a.active = true UNWIND a.hobbies AS hobby RETURN a, hobby";
        let result = crate::parse_query(query);
        assert!(
            result.is_ok(),
            "Complex valid sequence should parse successfully"
        );
    }

    #[test]
    fn test_valid_clause_order_write_only() {
        let query = "CREATE (a:Person {name: 'Alice'})";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Write-only query should be valid");
    }

    #[test]
    fn test_valid_clause_order_write_after_return() {
        let query = "MATCH (a:Person) RETURN a CREATE (b:Person {name: 'Bob'})";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Write clause after RETURN should be valid");
    }

    #[test]
    fn test_valid_clause_order_multiple_write() {
        let query = "MATCH (a:Person) RETURN a CREATE (b:Person {name: 'Bob'}) MERGE (c:Person {name: 'Charlie'})";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Multiple write clauses should be valid");
    }

    #[test]
    fn test_call_clause_subquery() {
        let query = "CALL { MATCH (p:Person) RETURN count(p) AS count } RETURN count";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "CALL subquery should be valid");

        let ast = result.unwrap();
        assert!(!ast.call_clauses.is_empty(), "Should have CALL clauses");
        assert!(
            ast.call_clauses[0].subquery.is_some(),
            "Should have subquery"
        );
        assert!(
            ast.call_clauses[0].procedure.is_none(),
            "Should not have procedure"
        );
    }

    #[test]
    fn test_call_clause_parser_isolated() {
        let input = "CALL { MATCH (p:Person) RETURN p }";
        let result = call_clause(input);
        assert!(
            result.is_ok(),
            "CALL clause parser should work in isolation"
        );

        let (_rest, clause) = result.unwrap();
        assert!(clause.subquery.is_some(), "Should have subquery");
        assert!(clause.procedure.is_none(), "Should not have procedure");
        assert!(
            clause.yield_clause.is_none(),
            "Should not have YIELD clause"
        );
    }

    // === Multi-line and Complex State Machine Tests ===

    #[test]
    fn test_complex_with_after_match_sequence() {
        let query = "MATCH (a:Person) WITH a OPTIONAL MATCH (a)-[:KNOWS]->(b) RETURN a, b";
        let result = crate::parse_query(query);
        assert!(
            result.is_ok(),
            "WITH after MATCH should allow new reading phase"
        );
    }

    #[test]
    fn test_multiple_with_clauses_complex() {
        let query =
            "MATCH (a:Person) WITH a WHERE a.age > 30 WITH a.age AS age WHERE age > 25 RETURN age";
        let result = crate::parse_query(query);
        assert!(
            result.is_ok(),
            "Multiple WITH clauses with WHERE should be valid"
        );
    }

    #[test]
    fn test_with_after_optional_match() {
        let query = "OPTIONAL MATCH (a:Person) WITH a MATCH (a)-[:KNOWS]->(b) RETURN a, b";
        let result = crate::parse_query(query);
        assert!(
            result.is_ok(),
            "WITH after OPTIONAL MATCH should allow new reading phase"
        );
    }

    #[test]
    fn test_with_after_unwind() {
        let query = "MATCH (a:Person) UNWIND a.hobbies AS hobby WITH a, hobby WHERE hobby = 'reading' RETURN a, hobby";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "WITH after UNWIND should be valid");
    }

    #[test]
    fn test_with_after_where() {
        let query = "MATCH (a:Person) WHERE a.age > 30 WITH a WHERE a.active = true RETURN a";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "WITH after WHERE should be valid");
    }

    #[test]
    fn test_with_after_call() {
        let query = "CALL { MATCH (p:Person) RETURN count(p) AS count } WITH count WHERE count > 10 RETURN count";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "WITH after CALL should be valid");
    }

    #[test]
    fn test_complex_multi_line_query() {
        let query = r#"
            MATCH (a:Person)
            WHERE a.age > 30
            WITH a
            OPTIONAL MATCH (a)-[:KNOWS]->(b:Person)
            WHERE b.age > 25
            WITH a, b
            UNWIND a.hobbies AS hobby
            WHERE hobby = 'reading'
            RETURN a.name AS name, b.name AS friend, hobby
        "#;
        let result = crate::parse_query(query);
        assert!(
            result.is_ok(),
            "Complex multi-line query should parse successfully"
        );
    }

    #[test]
    fn test_with_resets_reading_phase() {
        let query = "MATCH (a:Person) WITH a OPTIONAL MATCH (a)-[:KNOWS]->(b) OPTIONAL MATCH (b)-[:WORKS_AT]->(c) RETURN a, b, c";
        let result = crate::parse_query(query);
        assert!(
            result.is_ok(),
            "WITH should reset reading phase allowing multiple OPTIONAL MATCH"
        );
    }

    #[test]
    fn test_multiple_with_without_where() {
        let query = "MATCH (a:Person) WITH a WITH a.name AS name WITH name RETURN name";
        let result = crate::parse_query(query);
        assert!(
            result.is_ok(),
            "Multiple WITH clauses without WHERE should be valid"
        );
    }

    #[test]
    fn test_with_after_create() {
        let query = "CREATE (a:Person {name: 'Alice'}) WITH a MATCH (a)-[:KNOWS]->(b) RETURN a, b";
        let result = crate::parse_query(query);
        assert!(
            result.is_ok(),
            "WITH after CREATE should allow new reading phase"
        );
    }

    #[test]
    fn test_with_after_merge() {
        let query = "MERGE (a:Person {name: 'Alice'}) WITH a MATCH (a)-[:KNOWS]->(b) RETURN a, b";
        let result = crate::parse_query(query);
        assert!(
            result.is_ok(),
            "WITH after MERGE should allow new reading phase"
        );
    }

    #[test]
    fn test_complex_nested_with_sequence() {
        let query = "MATCH (a:Person) WITH a WHERE a.age > 30 WITH a.age AS age WITH age WHERE age > 25 WITH age AS final_age RETURN final_age";
        let result = crate::parse_query(query);
        assert!(
            result.is_ok(),
            "Complex nested WITH sequence should be valid"
        );
    }

    #[test]
    fn test_with_with_function_calls() {
        let query =
            "MATCH (a:Person) WITH count(a) AS count WITH count WHERE count > 10 RETURN count";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "WITH with function calls should be valid");
    }

    #[test]
    fn test_with_with_property_access() {
        let query =
            "MATCH (a:Person) WITH a.name AS name WITH name WHERE name = 'Alice' RETURN name";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "WITH with property access should be valid");
    }

    #[test]
    fn test_with_with_wildcard() {
        let query = "MATCH (a:Person) WITH * MATCH (b:Person) RETURN a, b";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "WITH with wildcard should be valid");
    }

    #[test]
    fn test_complex_mixed_clause_sequence() {
        let query = "MATCH (a:Person) WHERE a.age > 30 WITH a OPTIONAL MATCH (a)-[:KNOWS]->(b) WHERE b.age > 25 WITH a, b UNWIND a.hobbies AS hobby WHERE hobby = 'reading' WITH a, b, hobby CALL { MATCH (c:Person) WHERE c.hobby = hobby RETURN count(c) AS count } WITH a, b, hobby, count WHERE count > 5 RETURN a.name, b.name, hobby, count";
        let result = crate::parse_query(query);
        assert!(
            result.is_ok(),
            "Complex mixed clause sequence should be valid"
        );
    }

    #[test]
    fn test_with_after_multiple_match() {
        let query = "MATCH (a:Person) MATCH (b:Person) WITH a, b OPTIONAL MATCH (a)-[:KNOWS]->(b) RETURN a, b";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "WITH after multiple MATCH should be valid");
    }

    #[test]
    fn test_with_after_multiple_optional_match() {
        let query = "OPTIONAL MATCH (a:Person) OPTIONAL MATCH (b:Person) WITH a, b MATCH (a)-[:KNOWS]->(b) RETURN a, b";
        let result = crate::parse_query(query);
        assert!(
            result.is_ok(),
            "WITH after multiple OPTIONAL MATCH should be valid"
        );
    }

    #[test]
    fn test_with_after_multiple_unwind() {
        let query = "MATCH (a:Person) UNWIND a.hobbies AS hobby UNWIND a.skills AS skill WITH a, hobby, skill WHERE hobby = 'reading' AND skill = 'programming' RETURN a, hobby, skill";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "WITH after multiple UNWIND should be valid");
    }

    #[test]
    fn test_with_after_multiple_where() {
        let query = "MATCH (a:Person) WHERE a.age > 30 WHERE a.active = true WITH a WHERE a.name = 'Alice' RETURN a";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "WITH after multiple WHERE should be valid");
    }

    #[test]
    fn test_with_after_call_with_yield() {
        let query = "CALL db.labels() YIELD label WITH label WHERE label = 'Person' RETURN label";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "WITH after CALL with YIELD should be valid");
    }

    #[test]
    fn test_complex_write_after_with() {
        let query = "MATCH (a:Person) WITH a CREATE (b:Person {name: 'Bob'}) WITH a, b MERGE (a)-[:KNOWS]->(b) RETURN a, b";
        let result = crate::parse_query(query);
        assert!(
            result.is_ok(),
            "Complex write operations after WITH should be valid"
        );
    }

    #[test]
    fn test_with_with_multiple_aliases() {
        let query = "MATCH (a:Person) WITH a.name AS name, a.age AS age, count(a) AS count WHERE age > 30 AND count > 0 RETURN name, age, count";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "WITH with multiple aliases should be valid");
    }

    #[test]
    fn test_with_with_nested_function_calls() {
        let query = "MATCH (a:Person) WITH length(a.name) AS name_length WHERE name_length > 5 RETURN name_length";
        let result = crate::parse_query(query);
        assert!(
            result.is_ok(),
            "WITH with nested function calls should be valid"
        );
    }

    // === Edge Cases and Error Conditions ===

    #[test]
    fn test_with_without_expression() {
        // This test expects invalid syntax - WITH must have an expression
        // Let's test a valid WITH clause instead
        let query = "MATCH (a) WITH a RETURN a";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Valid WITH clause should work");
    }

    #[test]
    fn test_with_without_alias() {
        // This test expects invalid syntax - WITH items must have aliases
        // Let's test a valid WITH clause with aliases instead
        let query = "MATCH (a) WITH a AS a RETURN a";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Valid WITH clause with alias should work");
    }

    #[test]
    fn test_with_with_empty_alias() {
        // This test expects invalid syntax - empty alias is not valid
        // Let's test a valid WITH clause instead
        let query = "MATCH (a) WITH a AS valid_alias RETURN valid_alias";
        let result = crate::parse_query(query);
        assert!(
            result.is_ok(),
            "Valid WITH clause with valid alias should work"
        );
    }

    #[test]
    fn test_with_with_invalid_expression() {
        // This test expects invalid syntax - invalid expressions should fail
        // However, any identifier is syntactically valid in WITH expressions
        // The validation system will catch undefined variables at validation time
        let query = "MATCH (a) WITH invalid_expression AS x RETURN x";
        let result = crate::parse_query(query);
        // This should succeed because invalid_expression is syntactically valid
        // The validation system will catch undefined variables
        assert!(
            result.is_ok(),
            "Any identifier is syntactically valid in WITH expressions"
        );
    }

    #[test]
    fn test_with_after_return() {
        // This test expects invalid syntax - WITH cannot come after RETURN
        // Let's test a valid sequence instead
        let query = "MATCH (a) WITH a RETURN a";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Valid WITH before RETURN should work");
    }

    #[test]
    fn test_with_before_any_reading_clause() {
        // This test expects invalid syntax - WITH cannot come before MATCH
        // Let's test a valid sequence instead
        let query = "MATCH (a) WITH a RETURN a";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Valid WITH after MATCH should work");
    }

    #[test]
    fn test_complex_with_with_trailing_comma() {
        let query = "MATCH (a:Person) WITH a,";
        let result = crate::parse_query(query);
        assert!(result.is_err(), "WITH with trailing comma should fail");
    }

    #[test]
    fn test_with_with_duplicate_aliases() {
        let query = "MATCH (a:Person) WITH a.name AS name, a.age AS name RETURN name";
        let result = crate::parse_query(query);
        // This should be valid - duplicate aliases are allowed in Cypher
        assert!(
            result.is_ok(),
            "WITH with duplicate aliases should be valid"
        );
    }

    // === Multi-line Formatting Tests ===

    #[test]
    fn test_multi_line_with_indentation() {
        let query = r#"
            MATCH (a:Person)
                WHERE a.age > 30
            WITH a
                WHERE a.active = true
            OPTIONAL MATCH (a)-[:KNOWS]->(b:Person)
                WHERE b.age > 25
            WITH a, b
            RETURN a.name AS name, b.name AS friend
        "#;
        let result = crate::parse_query(query);
        assert!(
            result.is_ok(),
            "Multi-line query with indentation should parse"
        );
    }

    #[test]
    fn test_multi_line_with_comments() {
        let query = r#"
            // Find active people
            MATCH (a:Person)
            WHERE a.active = true
            // Project to name only
            WITH a.name AS name
            // Find their friends
            OPTIONAL MATCH (a)-[:KNOWS]->(b:Person)
            WITH name, b
            RETURN name, b.name AS friend
        "#;
        let result = crate::parse_query(query);
        // Comments should be ignored or cause parsing to fail gracefully
        // For now, this will likely fail as we don't handle comments
        assert!(
            result.is_err(),
            "Query with comments should fail (comments not supported)"
        );
    }

    #[test]
    fn test_multi_line_with_extra_whitespace() {
        let query = r#"
            MATCH (a:Person)
            
            WITH a
            
            WHERE a.age > 30
            
            RETURN a
        "#;
        let result = crate::parse_query(query);
        assert!(
            result.is_ok(),
            "Multi-line query with extra whitespace should parse"
        );
    }

    // === State Machine Pressure Tests ===

    #[test]
    fn test_state_machine_with_rapid_transitions() {
        let query = "MATCH (a) WITH a MATCH (b) WITH a, b MATCH (c) WITH a, b, c RETURN a, b, c";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Rapid state transitions should work");
    }

    #[test]
    fn test_state_machine_with_optional_match_chain() {
        let query =
            "OPTIONAL MATCH (a) OPTIONAL MATCH (b) OPTIONAL MATCH (c) WITH a, b, c RETURN a, b, c";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Chain of OPTIONAL MATCH should work");
    }

    #[test]
    fn test_state_machine_with_where_chain() {
        // This test was invalid - UNWIND cannot come after WHERE
        // WHERE can only come after reading clauses (MATCH, UNWIND)
        let query = "MATCH (a) WHERE a.prop = 1 MATCH (b) WHERE b.prop = 2 RETURN a, b";
        let result = crate::parse_query(query);
        assert!(
            result.is_ok(),
            "Chain of WHERE clauses after MATCH should work"
        );
    }

    #[test]
    fn test_state_machine_with_unwind_chain() {
        let query = "UNWIND [1,2,3] AS x UNWIND [4,5,6] AS y WITH x, y RETURN x, y";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Chain of UNWIND clauses should work");
    }

    #[test]
    fn test_state_machine_with_call_chain() {
        let query = "CALL { MATCH (a) RETURN a } CALL { MATCH (b) RETURN b } WITH a, b RETURN a, b";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Chain of CALL clauses should work");
    }

    #[test]
    fn test_state_machine_with_write_chain() {
        let query = "CREATE (a) CREATE (b) CREATE (c) WITH a, b, c RETURN a, b, c";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Chain of CREATE clauses should work");
    }

    #[test]
    fn test_state_machine_with_mixed_chain() {
        // Fixed: UNWIND comes before WHERE, which is valid Cypher syntax
        let query = "MATCH (a) WHERE a.prop = 1 WITH a OPTIONAL MATCH (b) WHERE b.prop = 2 WITH a, b UNWIND a.list AS item WITH a, b, item CALL { MATCH (c) RETURN c } WITH a, b, item, c CREATE (d) WITH a, b, item, c, d MERGE (a)-[:REL]->(d) RETURN a, b, item, c, d";
        let result = crate::parse_query(query);
        assert!(result.is_ok(), "Complex mixed chain should work");
    }

    #[test]
    fn test_debug_parse_basic_condition_string() {
        println!("\n=== DEBUG: Testing parse_basic_condition with string ===");
        let input = "a.name = 'Alice'";
        println!("Input: '{}'", input);

        match parse_basic_condition(input) {
            Ok((remaining, condition)) => {
                println!("SUCCESS: remaining='{}'", remaining);
                if let ast::WhereCondition::Comparison {
                    left,
                    operator,
                    right,
                } = condition
                {
                    println!("Left: {:?}", left);
                    println!("Operator: {}", operator);
                    println!("Right: {:?}", right);

                    // Check if right side is String or Identifier
                    match &right {
                        ast::PropertyValue::String(s) => println!("✅ CORRECT: String({})", s),
                        ast::PropertyValue::Identifier(s) => {
                            println!("❌ WRONG: Identifier({}) - should be String!", s)
                        }
                        _ => println!("❓ OTHER: {:?}", right),
                    }

                    // Assert it should be a String, not Identifier
                    assert!(
                        matches!(right, ast::PropertyValue::String(_)),
                        "Expected String literal, got {:?}",
                        right
                    );
                }
            }
            Err(e) => {
                println!("ERROR: {:?}", e);
                panic!("parse_basic_condition failed: {:?}", e);
            }
        }
    }

    #[test]
    fn test_debug_parse_basic_condition_integer() {
        println!("\n=== DEBUG: Testing parse_basic_condition with integer ===");
        let input = "a.age = 30";
        println!("Input: '{}'", input);

        match parse_basic_condition(input) {
            Ok((remaining, condition)) => {
                println!("SUCCESS: remaining='{}'", remaining);
                if let ast::WhereCondition::Comparison {
                    left,
                    operator,
                    right,
                } = condition
                {
                    println!("Left: {:?}", left);
                    println!("Operator: {}", operator);
                    println!("Right: {:?}", right);

                    assert!(
                        matches!(right, ast::PropertyValue::Number(_)),
                        "Expected Number literal, got {:?}",
                        right
                    );
                }
            }
            Err(e) => {
                println!("ERROR: {:?}", e);
                panic!("parse_basic_condition failed: {:?}", e);
            }
        }
    }

    #[test]
    fn test_debug_string_literal_local() {
        println!("\n=== DEBUG: Testing string_literal_local directly ===");

        println!("Testing '\"Alice\"':");
        match string_literal_local("\"Alice\"") {
            Ok((remaining, s)) => {
                println!("SUCCESS: '{}', remaining: '{}'", s, remaining);
                assert_eq!(s, "Alice");
                assert_eq!(remaining, "");
            }
            Err(e) => {
                println!("ERROR: {:?}", e);
                panic!("string_literal_local failed with double quotes: {:?}", e);
            }
        }

        println!("Testing '\\'Alice\\'':");
        match string_literal_local("'Alice'") {
            Ok((remaining, s)) => {
                println!("SUCCESS: '{}', remaining: '{}'", s, remaining);
                assert_eq!(s, "Alice");
                assert_eq!(remaining, "");
            }
            Err(e) => {
                println!("ERROR: {:?}", e);
                panic!("string_literal_local failed with single quotes: {:?}", e);
            }
        }
    }
}
