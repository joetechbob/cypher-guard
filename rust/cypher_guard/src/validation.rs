use crate::errors::CypherGuardValidationError;
use crate::parser::ast::*;
use crate::schema::DbSchema;
use crate::types::{check_type_compatibility, parse_neo4j_type, Neo4jType, TypeCheckLevel, TypeIssue, TypeMismatchSeverity};
use std::collections::{HashMap, HashSet};

/// Represents the extracted elements from a Cypher query that need validation
#[derive(Debug, Clone)]
pub struct QueryElements {
    pub node_labels: HashSet<String>,
    pub relationship_types: HashSet<String>,
    pub node_properties: HashMap<String, HashSet<String>>, // label -> set of property names
    pub relationship_properties: HashMap<String, HashSet<String>>, // rel_type -> set of property names
    pub property_accesses: Vec<PropertyAccess>,                    // Property access with context
    pub property_comparisons: Vec<PropertyComparison>, // Property comparisons for type validation
    pub defined_variables: HashSet<String>, // Variables that are defined (from MATCH, UNWIND, etc.)
    pub referenced_variables: HashSet<String>, // Variables that are referenced (from WITH, WHERE, RETURN, etc.)
    pub pattern_sequences: Vec<Vec<PatternElement>>, // Track complete pattern sequences for validation
    pub variable_node_bindings: HashMap<String, String>, // variable -> node label bindings
    pub variable_relationship_bindings: HashMap<String, String>, // variable -> relationship type bindings
}

#[derive(Debug, Clone)]
pub struct PropertyAccess {
    pub variable: String,
    pub property: String,
    pub context: PropertyContext,
}

#[derive(Debug, Clone)]
pub struct PropertyComparison {
    pub variable: String,
    pub property: String,
    pub value: String,
    pub value_type: PropertyValueType,
}

#[derive(Debug, Clone)]
pub enum PropertyValueType {
    String,
    Number,
    Boolean,
    Date,
    DateTime,
    Null,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub enum PropertyContext {
    Where,
    Return,
    With,
}

impl QueryElements {
    pub fn new() -> Self {
        Self {
            node_labels: HashSet::new(),
            relationship_types: HashSet::new(),
            node_properties: HashMap::new(),
            relationship_properties: HashMap::new(),
            property_accesses: Vec::new(),
            property_comparisons: Vec::new(),
            defined_variables: HashSet::new(),
            referenced_variables: HashSet::new(),
            pattern_sequences: Vec::new(),
            variable_node_bindings: HashMap::new(),
            variable_relationship_bindings: HashMap::new(),
        }
    }

    /// Add a node label to the set
    pub fn add_node_label(&mut self, label: String) {
        self.node_labels.insert(label);
    }

    /// Add a relationship type to the set
    pub fn add_relationship_type(&mut self, rel_type: String) {
        self.relationship_types.insert(rel_type);
    }

    /// Add a node property to the set
    pub fn add_node_property(&mut self, label: String, property: String) {
        self.node_properties
            .entry(label)
            .or_default()
            .insert(property);
    }

    /// Add a relationship property to the set
    pub fn add_relationship_property(&mut self, rel_type: String, property: String) {
        self.relationship_properties
            .entry(rel_type)
            .or_default()
            .insert(property);
    }

    /// Add a variable to the set
    pub fn add_variable(&mut self, variable: String) {
        self.referenced_variables.insert(variable);
    }

    /// Add a variable that is defined
    pub fn add_defined_variable(&mut self, variable: String) {
        self.defined_variables.insert(variable);
    }

    /// Add property access with context
    pub fn add_property_access(&mut self, access: PropertyAccess) {
        self.property_accesses.push(access);
    }

    /// Add a pattern sequence for validation
    pub fn add_pattern_sequence(&mut self, pattern: Vec<PatternElement>) {
        self.pattern_sequences.push(pattern);
    }

    /// Add a property comparison for type validation
    pub fn add_property_comparison(&mut self, comparison: PropertyComparison) {
        self.property_comparisons.push(comparison);
    }

    /// Add a variable to node label binding
    pub fn add_variable_node_binding(&mut self, variable: String, label: String) {
        self.variable_node_bindings.insert(variable, label);
    }

    /// Add a variable to relationship type binding
    pub fn add_variable_relationship_binding(&mut self, variable: String, rel_type: String) {
        self.variable_relationship_bindings
            .insert(variable, rel_type);
    }
}

/// Convert PropertyValue to PropertyValueType
fn property_value_to_type(value: &PropertyValue) -> PropertyValueType {
    match value {
        PropertyValue::String(_) => PropertyValueType::String,
        PropertyValue::Number(_) => PropertyValueType::Number,
        PropertyValue::Boolean(_) => PropertyValueType::Boolean,
        PropertyValue::Null => PropertyValueType::Null,
        PropertyValue::FunctionCall { name, .. } => {
            // Detect date/datetime functions
            match name.to_lowercase().as_str() {
                "date" => PropertyValueType::Date,
                "datetime" => PropertyValueType::DateTime,
                _ => PropertyValueType::Unknown,
            }
        }
        PropertyValue::Identifier(_) => PropertyValueType::Unknown,
        _ => PropertyValueType::Unknown,
    }
}

/// Convert PropertyValue to String representation
fn property_value_to_string(value: &PropertyValue) -> String {
    match value {
        PropertyValue::String(s) => s.clone(),
        PropertyValue::Number(n) => n.to_string(),
        PropertyValue::Boolean(b) => b.to_string(),
        PropertyValue::Null => "null".to_string(),
        PropertyValue::Identifier(id) => id.clone(),
        PropertyValue::Parameter(p) => format!("${}", p),
        _ => "unknown".to_string(),
    }
}

/// Extract elements from a PropertyValue (for WHERE conditions)
fn extract_from_property_value(
    value: &PropertyValue,
    elements: &mut QueryElements,
    context: PropertyContext,
) {
    match value {
        PropertyValue::Identifier(id) => {
            extract_property_access_from_string(id, elements, context);
        }
        PropertyValue::String(_s) => {
            // Literals don't contribute variables
        }
        PropertyValue::Number(_) => {
            // Literals don't contribute variables
        }
        PropertyValue::Boolean(_) => {
            // Literals don't contribute variables
        }
        PropertyValue::Null => {
            // Literals don't contribute variables
        }
        PropertyValue::Parameter(_) => {
            // Parameters don't contribute variables (they're external)
        }
        PropertyValue::FunctionCall { args, .. } => {
            for arg in args {
                extract_from_property_value(arg, elements, context);
            }
        }
        _ => {
            // Handle other cases as needed
        }
    }
}

/// Helper function to extract all individual labels from a label expression
fn extract_labels_from_expression(expr: &LabelExpression, labels: &mut Vec<String>) {
    match expr {
        LabelExpression::Single(label) => {
            labels.push(label.clone());
        }
        LabelExpression::Or(left, right) | LabelExpression::And(left, right) => {
            extract_labels_from_expression(left, labels);
            extract_labels_from_expression(right, labels);
        }
        LabelExpression::Not(inner) => {
            extract_labels_from_expression(inner, labels);
        }
    }
}

/// Helper function to get labels from a node pattern (handles both legacy and label expressions)
fn get_node_labels(node: &NodePattern) -> Vec<String> {
    let mut labels = Vec::new();

    // Try label_expression first (Neo4j 5.x)
    if let Some(label_expr) = &node.label_expression {
        extract_labels_from_expression(label_expr, &mut labels);
    } else if let Some(label) = &node.label {
        // Fall back to legacy single label
        labels.push(label.clone());
    }

    labels
}

/// Extract all elements from a parsed query that need validation
pub fn extract_query_elements(query: &Query) -> QueryElements {
    let mut elements = QueryElements::new();

    // Extract from MATCH clauses
    for match_clause in &query.match_clauses {
        for element in &match_clause.elements {
            extract_from_match_element(element, &mut elements);
        }
    }

    // Extract from MERGE clauses
    for merge_clause in &query.merge_clauses {
        for element in &merge_clause.elements {
            extract_from_match_element(element, &mut elements);
        }
    }

    // Extract from CREATE clauses
    for create_clause in &query.create_clauses {
        for element in &create_clause.elements {
            extract_from_match_element(element, &mut elements);
        }
    }

    // Extract from WHERE clauses
    for where_clause in &query.where_clauses {
        for condition in &where_clause.conditions {
            extract_from_where_condition(condition, &mut elements);
        }
    }

    // Extract from RETURN clauses
    for return_clause in &query.return_clauses {
        for item in &return_clause.items {
            extract_from_return_item(item, &mut elements);
        }
    }

    // Extract from WITH clauses
    for with_clause in &query.with_clauses {
        for item in &with_clause.items {
            extract_from_with_item(item, &mut elements);
        }
    }

    // Extract from UNWIND clauses
    for unwind_clause in &query.unwind_clauses {
        elements.add_defined_variable(unwind_clause.variable.clone());
        // Optionally, could track type info here in the future
    }

    elements
}

/// Extract elements from a single match element
fn extract_from_match_element(element: &MatchElement, elements: &mut QueryElements) {
    // Extract the path variable if it exists
    if let Some(path_var) = &element.path_var {
        elements.add_defined_variable(path_var.clone());
    }

    // Track the complete pattern sequence for validation
    elements.add_pattern_sequence(element.pattern.clone());

    for pattern_element in &element.pattern {
        match pattern_element {
            PatternElement::Node(node) => {
                // Extract variable from node
                if let Some(variable) = &node.variable {
                    elements.add_defined_variable(variable.clone());

                    // Track variable to node label binding (use first label from expression)
                    let labels = get_node_labels(node);
                    if let Some(label) = labels.first() {
                        elements.add_variable_node_binding(variable.clone(), label.clone());
                    }
                }

                // Extract all labels from label expression or single label
                let labels = get_node_labels(node);
                for label in &labels {
                    elements.add_node_label(label.clone());

                    // Extract properties from node pattern
                    if let Some(properties) = &node.properties {
                        for prop in properties {
                            elements.add_node_property(label.clone(), prop.key.clone());
                        }
                    }
                }
            }
            PatternElement::Relationship(rel) => {
                // Extract variable from relationship
                match rel {
                    RelationshipPattern::Regular(details)
                    | RelationshipPattern::OptionalRelationship(details) => {
                        if let Some(variable) = &details.variable {
                            elements.add_defined_variable(variable.clone());

                            // Track variable to relationship type binding
                            if let Some(rel_type) = &details.rel_type {
                                elements.add_variable_relationship_binding(
                                    variable.clone(),
                                    rel_type.clone(),
                                );
                            }
                        }
                    }
                }

                if let Some(rel_type) = rel.rel_type() {
                    elements.add_relationship_type(rel_type.to_string());

                    // Extract properties from relationship pattern
                    if let Some(properties) = rel.properties() {
                        for prop in properties {
                            elements
                                .add_relationship_property(rel_type.to_string(), prop.key.clone());
                        }
                    }
                }
            }
            PatternElement::QuantifiedPathPattern(qpp) => {
                // Extract path variable if it exists
                if let Some(path_var) = &qpp.path_variable {
                    elements.add_defined_variable(path_var.clone());
                }

                // Extract from the pattern inside the QPP
                for pattern_element in &qpp.pattern {
                    match pattern_element {
                        PatternElement::Node(node) => {
                            if let Some(variable) = &node.variable {
                                elements.add_defined_variable(variable.clone());
                            }
                            let labels = get_node_labels(node);
                            for label in &labels {
                                elements.add_node_label(label.clone());
                            }
                            // Extract node properties
                            if let Some(props) = &node.properties {
                                for prop in props {
                                    for label in &labels {
                                        elements.add_node_property(label.clone(), prop.key.clone());
                                    }
                                }
                            }
                        }
                        PatternElement::Relationship(rel) => {
                            match rel {
                                RelationshipPattern::Regular(details)
                                | RelationshipPattern::OptionalRelationship(details) => {
                                    if let Some(variable) = &details.variable {
                                        elements.add_defined_variable(variable.clone());
                                    }
                                }
                            }
                            if let Some(rel_type) = rel.rel_type() {
                                elements.add_relationship_type(rel_type.to_string());
                            }
                            // Extract relationship properties
                            if let Some(props) = rel.properties() {
                                for prop in props {
                                    if let Some(rel_type) = rel.rel_type() {
                                        elements.add_relationship_property(rel_type.to_string(), prop.key.clone());
                                    }
                                }
                            }
                        }
                        PatternElement::QuantifiedPathPattern(_) => {
                            // Nested QPPs are not supported in this implementation
                        }
                    }
                }
            }
        }
    }
}

/// Extract elements from a WHERE condition
fn extract_from_where_condition(condition: &WhereCondition, elements: &mut QueryElements) {
    match condition {
        WhereCondition::Comparison {
            left,
            right,
            operator: _,
        } => {
            extract_from_property_value(left, elements, PropertyContext::Where);
            extract_from_property_value(right, elements, PropertyContext::Where);

            // Track property comparisons for type validation
            if let PropertyValue::Identifier(left_str) = left {
                if left_str.contains('.') {
                    // Left side is a property access
                    let parts: Vec<&str> = left_str.split('.').collect();
                    if parts.len() == 2 {
                        let variable = parts[0].trim();
                        let property = parts[1].trim();

                        elements.add_property_comparison(PropertyComparison {
                            variable: variable.to_string(),
                            property: property.to_string(),
                            value: property_value_to_string(right),
                            value_type: property_value_to_type(right),
                        });
                    }
                }
            }

            if let PropertyValue::Identifier(right_str) = right {
                if right_str.contains('.') {
                    // Right side is a property access
                    let parts: Vec<&str> = right_str.split('.').collect();
                    if parts.len() == 2 {
                        let variable = parts[0].trim();
                        let property = parts[1].trim();

                        elements.add_property_comparison(PropertyComparison {
                            variable: variable.to_string(),
                            property: property.to_string(),
                            value: property_value_to_string(left),
                            value_type: property_value_to_type(left),
                        });
                    }
                }
            }
        }
        WhereCondition::FunctionCall { arguments, .. } => {
            for arg in arguments {
                extract_property_access_from_string(arg, elements, PropertyContext::Where);
            }
        }
        WhereCondition::PathProperty { path_var, property } => {
            elements.add_variable(path_var.clone());
            elements.add_property_access(PropertyAccess {
                variable: path_var.clone(),
                property: property.clone(),
                context: PropertyContext::Where,
            });
        }
        WhereCondition::PatternPredicate { pattern } => {
            // Extract variables and relationships from the pattern
            for element in pattern {
                match element {
                    PatternElement::Node(node) => {
                        if let Some(var) = &node.variable {
                            elements.add_variable(var.clone());
                        }
                        if let Some(props) = &node.properties {
                            for prop in props {
                                extract_from_property_value(&prop.value, elements, PropertyContext::Where);
                            }
                        }
                    }
                    PatternElement::Relationship(rel) => {
                        let details = match rel {
                            RelationshipPattern::Regular(d) => d,
                            RelationshipPattern::OptionalRelationship(d) => d,
                        };
                        if let Some(var) = &details.variable {
                            elements.add_variable(var.clone());
                        }
                        if let Some(props) = &details.properties {
                            for prop in props {
                                extract_from_property_value(&prop.value, elements, PropertyContext::Where);
                            }
                        }
                    }
                    PatternElement::QuantifiedPathPattern(qpp) => {
                        // Validate QPP quantifiers
                        if let (Some(min), Some(max)) = (qpp.min, qpp.max) {
                            if min > max {
                                // Invalid quantifier range - min cannot be greater than max
                                // This is a structural error that should be caught during parsing
                                // but we check it here for completeness
                            }
                        }

                        // Extract and validate nodes and relationships within QPP
                        for pattern_element in &qpp.pattern {
                            match pattern_element {
                                PatternElement::Node(node) => {
                                    if let Some(var) = &node.variable {
                                        elements.add_defined_variable(var.clone());
                                    }
                                    let labels = get_node_labels(node);
                                    for label in &labels {
                                        elements.add_node_label(label.clone());
                                    }
                                    if let Some(props) = &node.properties {
                                        for prop in props {
                                            for label in &labels {
                                                elements.add_node_property(label.clone(), prop.key.clone());
                                            }
                                            extract_from_property_value(&prop.value, elements, PropertyContext::Where);
                                        }
                                    }
                                }
                                PatternElement::Relationship(rel) => {
                                    // Extract relationship variable if present
                                    match rel {
                                        RelationshipPattern::Regular(details)
                                        | RelationshipPattern::OptionalRelationship(details) => {
                                            if let Some(variable) = &details.variable {
                                                elements.add_defined_variable(variable.clone());
                                            }
                                        }
                                    }
                                    // Add relationship type
                                    if let Some(rel_type) = rel.rel_type() {
                                        elements.add_relationship_type(rel_type.to_string());
                                    }
                                    // Extract properties
                                    if let Some(props) = rel.properties() {
                                        for prop in props {
                                            if let Some(rel_type) = rel.rel_type() {
                                                elements.add_relationship_property(rel_type.to_string(), prop.key.clone());
                                            }
                                            extract_from_property_value(&prop.value, elements, PropertyContext::Where);
                                        }
                                    }
                                }
                                PatternElement::QuantifiedPathPattern(_) => {
                                    // Nested QPPs not supported in this implementation
                                }
                            }
                        }

                        // Validate WHERE clause within QPP if present
                        if let Some(where_clause) = &qpp.where_clause {
                            extract_from_where_condition(&where_clause.conditions[0], elements);
                        }
                    }
                }
            }
        }
        WhereCondition::And(left, right) => {
            extract_from_where_condition(left, elements);
            extract_from_where_condition(right, elements);
        }
        WhereCondition::Or(left, right) => {
            extract_from_where_condition(left, elements);
            extract_from_where_condition(right, elements);
        }
        WhereCondition::Xor(left, right) => {
            extract_from_where_condition(left, elements);
            extract_from_where_condition(right, elements);
        }
        WhereCondition::Not(condition) => {
            extract_from_where_condition(condition, elements);
        }
        WhereCondition::Parenthesized(condition) => {
            extract_from_where_condition(condition, elements);
        }
    }
}

/// Extract elements from a RETURN item
fn extract_from_return_item(item: &str, elements: &mut QueryElements) {
    println!("🔍 RETURN_ITEM: processing '{}'", item);
    extract_property_access_from_string(item, elements, PropertyContext::Return);
}

/// Extract elements from a WITH item
fn extract_from_with_item(item: &WithItem, elements: &mut QueryElements) {
    extract_from_with_expression(&item.expression, elements);

    // If there's an alias, add it as a defined variable
    if let Some(alias) = &item.alias {
        elements.add_defined_variable(alias.clone());
    }
}

/// Extract elements from a WITH expression
fn extract_from_with_expression(expression: &WithExpression, elements: &mut QueryElements) {
    match expression {
        WithExpression::Identifier(var) => {
            elements.add_variable(var.clone());
        }
        WithExpression::PropertyAccess { variable, property } => {
            elements.add_variable(variable.clone());
            elements.add_property_access(PropertyAccess {
                variable: variable.clone(),
                property: property.clone(),
                context: PropertyContext::With,
            });
        }
        WithExpression::FunctionCall { args, .. } => {
            for arg in args {
                extract_from_with_expression(arg, elements);
            }
        }
        WithExpression::Wildcard => {
            // No specific extraction needed for wildcard
        }
    }
}

/// Validation configuration options
#[derive(Debug, Clone)]
pub struct ValidationOptions {
    pub type_checking: TypeCheckLevel,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            type_checking: TypeCheckLevel::Off,
        }
    }
}

/// Check a property comparison for type mismatches
fn check_property_comparison_types(
    comparison: &PropertyComparison,
    elements: &QueryElements,
    schema: &DbSchema,
    type_check_level: TypeCheckLevel,
) -> Option<TypeIssue> {
    // Get the node label for this variable
    let label = elements.variable_node_bindings.get(&comparison.variable)?;
    
    // Get the property type from schema
    let properties = schema.node_props.get(label)?;
    let prop_def = properties.iter().find(|p| p.name == comparison.property)?;
    
    // Parse the property type
    let prop_type = parse_neo4j_type(&prop_def.neo4j_type.to_string());
    
    // Infer the comparison value type
    let value_type = match comparison.value_type {
        PropertyValueType::String => Neo4jType::String,
        PropertyValueType::Number => Neo4jType::Integer,  // Simplified
        PropertyValueType::Boolean => Neo4jType::Boolean,
        PropertyValueType::Date => Neo4jType::Date,
        PropertyValueType::DateTime => Neo4jType::DateTime,
        PropertyValueType::Null | PropertyValueType::Unknown => return None,  // Skip
    };
    
    // Check compatibility (blocklist approach)
    if let Some(base_severity) = check_type_compatibility(&prop_type, &value_type) {
        let message = format!(
            "Type mismatch: {}.{} is {}, compared with {}",
            comparison.variable,
            comparison.property,
            prop_type,
            value_type
        );
        
        let suggestion = match (&prop_type, &value_type) {
            (Neo4jType::String, Neo4jType::Date) => {
                Some(format!("Convert string to date: WHERE date({}.{}) <= date(...)", 
                    comparison.variable, comparison.property))
            }
            _ => None,
        };
        
        // Note: severity from check_type_compatibility is already appropriate
        // (Error for silent failures, Warning for likely unintentional)
        // No need to downgrade based on TypeCheckLevel - the base severity is correct
        
        return Some(TypeIssue {
            severity: base_severity,
            message,
            suggestion,
        });
    }
    
    None
}

/// Extract property access from a string (e.g., "a.name", "r.role")
fn extract_property_access_from_string(
    s: &str,
    elements: &mut QueryElements,
    context: PropertyContext,
) {
    let trimmed = s.trim();
    println!(
        "DEBUG: extract_property_access_from_string called with: '{}'",
        trimmed
    );

    // Skip string literals (quoted strings)
    if trimmed.starts_with('"') && trimmed.ends_with('"') {
        println!("DEBUG: Skipping double-quoted string: {}", trimmed);
        return;
    }
    if trimmed.starts_with('\'') && trimmed.ends_with('\'') {
        println!("DEBUG: Skipping single-quoted string: {}", trimmed);
        return;
    }

    // Skip function calls (e.g., "nodes(p)", "length(p)")
    // Function calls contain parentheses and should not be treated as variables
    if trimmed.contains('(') && trimmed.contains(')') {
        println!("DEBUG: Skipping function call: {}", trimmed);
        return;
    }

    // Simple pattern matching for property access: variable.property
    if let Some(dot_pos) = trimmed.find('.') {
        let variable = trimmed[..dot_pos].trim();
        let property = trimmed[dot_pos + 1..].trim();

        if !variable.is_empty() && !property.is_empty() {
            elements.add_variable(variable.to_string());
            elements.add_property_access(PropertyAccess {
                variable: variable.to_string(),
                property: property.to_string(),
                context,
            });
        }
    } else {
        // Only add as a variable if it looks like a variable reference
        // (not a string literal, number, or other literal)
        if !trimmed.is_empty()
            && !trimmed.contains(' ')
            && !trimmed.chars().all(|c| c.is_ascii_digit())
            && !trimmed.eq_ignore_ascii_case("true")
            && !trimmed.eq_ignore_ascii_case("false")
            && !trimmed.eq_ignore_ascii_case("null")
            && !trimmed.starts_with('"')
            && !trimmed.starts_with('\'')
            && !trimmed.ends_with('"')
            && !trimmed.ends_with('\'')
        {
            println!("DEBUG: Adding variable: {}", trimmed);
            elements.add_variable(trimmed.to_string());
        }
    }
}

/// Validate extracted query elements against the schema with optional type checking
pub fn validate_query_elements_with_options(
    elements: &QueryElements,
    schema: &DbSchema,
    options: &ValidationOptions,
) -> (Vec<CypherGuardValidationError>, Vec<TypeIssue>) {
    eprintln!("DEBUG: validate_query_elements called");
    eprintln!(
        "DEBUG: elements.referenced_variables: {:?}",
        elements.referenced_variables
    );
    eprintln!(
        "DEBUG: elements.defined_variables: {:?}",
        elements.defined_variables
    );
    let mut errors = Vec::new();

    // Validate that all referenced variables are defined
    for referenced_var in &elements.referenced_variables {
        if !elements.defined_variables.contains(referenced_var) {
            errors.push(CypherGuardValidationError::UndefinedVariable(
                referenced_var.clone(),
            ));
        }
    }

    // Validate node labels
    for label in &elements.node_labels {
        if !schema.has_label(label) {
            errors.push(CypherGuardValidationError::InvalidNodeLabel(label.clone()));
        }
    }

    // Validate relationship types
    for rel_type in &elements.relationship_types {
        if !schema.has_relationship_type(rel_type) {
            errors.push(CypherGuardValidationError::InvalidRelationshipType(
                rel_type.clone(),
            ));
        }
    }

    // Validate relationship directions
    for pattern_sequence in &elements.pattern_sequences {
        // Extract nodes and relationships from the pattern sequence, flattening QPPs
        let mut nodes = Vec::new();
        let mut relationships = Vec::new();

        for element in pattern_sequence {
            match element {
                PatternElement::Node(node) => {
                    let labels = get_node_labels(node);
                    for label in labels {
                        nodes.push(label);
                    }
                }
                PatternElement::Relationship(rel) => {
                    if let Some(rel_type) = rel.rel_type() {
                        relationships.push((rel_type.to_string(), rel.direction()));
                    }
                }
                PatternElement::QuantifiedPathPattern(qpp) => {
                    // Extract nodes and relationships from inside the QPP
                    // The QPP connects the previous node to the next node in the sequence
                    for pattern_element in &qpp.pattern {
                        match pattern_element {
                            PatternElement::Node(node) => {
                                let labels = get_node_labels(node);
                                for label in labels {
                                    nodes.push(label);
                                }
                            }
                            PatternElement::Relationship(rel) => {
                                if let Some(rel_type) = rel.rel_type() {
                                    relationships.push((rel_type.to_string(), rel.direction()));
                                }
                            }
                            PatternElement::QuantifiedPathPattern(_) => {
                                // Nested QPPs are not supported in this implementation
                                continue;
                            }
                        }
                    }
                }
            }
        }

        // Validate each relationship in the sequence
        for (i, (rel_type, direction)) in relationships.iter().enumerate() {
            if let Some(schema_rel) = schema
                .relationships
                .iter()
                .find(|r| r.rel_type == *rel_type)
            {
                // Get the nodes connected by this relationship
                if i < nodes.len() - 1 && !nodes.is_empty() {
                    let node1 = &nodes[i];
                    let node2 = &nodes[i + 1];

                    match direction {
                        Direction::Right => {
                            // Right direction: node1 -> node2
                            // Check if this matches the schema direction
                            if node1 != &schema_rel.start || node2 != &schema_rel.end {
                                errors.push(CypherGuardValidationError::InvalidRelationship(
                                    format!("Relationship '{}' direction mismatch: expected {}->{}, got {}->{}", 
                                        rel_type, schema_rel.start, schema_rel.end, node1, node2)
                                ));
                            }
                        }
                        Direction::Left => {
                            // Left direction: node1 <- node2 (equivalent to node2 -> node1)
                            // Check if this matches the schema direction
                            if node1 != &schema_rel.end || node2 != &schema_rel.start {
                                errors.push(CypherGuardValidationError::InvalidRelationship(
                                    format!("Relationship '{}' direction mismatch: expected {}->{}, got {}->{}", 
                                        rel_type, schema_rel.start, schema_rel.end, node2, node1)
                                ));
                            }
                        }
                        Direction::Undirected => {
                            // Undirected: check if both nodes are valid for this relationship
                            // This is always valid since relationships are stored undirected
                            let valid_combination = (node1 == &schema_rel.start
                                && node2 == &schema_rel.end)
                                || (node1 == &schema_rel.end && node2 == &schema_rel.start);
                            if !valid_combination {
                                errors.push(CypherGuardValidationError::InvalidRelationship(
                                    format!("Relationship '{}' invalid node combination: expected {} and {}, got {} and {}", 
                                        rel_type, schema_rel.start, schema_rel.end, node1, node2)
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // Validate node properties
    for (label, properties) in &elements.node_properties {
        if !schema.has_label(label) {
            errors.push(CypherGuardValidationError::InvalidNodeLabel(label.clone()));
            continue;
        }
        for property in properties {
            if !schema.has_node_property(label, property) {
                errors.push(CypherGuardValidationError::InvalidNodeProperty {
                    label: label.clone(),
                    property: property.clone(),
                });
            }
        }
    }

    // Validate relationship properties
    for (rel_type, properties) in &elements.relationship_properties {
        if !schema.has_relationship_type(rel_type) {
            errors.push(CypherGuardValidationError::InvalidRelationshipType(
                rel_type.clone(),
            ));
            continue;
        }
        for property in properties {
            if !schema.has_relationship_property(rel_type, property) {
                errors.push(CypherGuardValidationError::InvalidRelationshipProperty {
                    rel_type: rel_type.clone(),
                    property: property.clone(),
                });
            }
        }
    }

    // Validate property access
    for access in &elements.property_accesses {
        let context_str = match access.context {
            PropertyContext::Where => "WHERE clause",
            PropertyContext::Return => "RETURN clause",
            PropertyContext::With => "WITH clause",
        };

        let mut found = false;

        // Check if the property exists in any node label
        for properties in schema.node_props.values() {
            if properties.iter().any(|p| p.name == access.property) {
                found = true;
                break;
            }
        }

        // If not found in nodes, check relationship properties
        if !found {
            for properties in schema.rel_props.values() {
                if properties.iter().any(|p| p.name == access.property) {
                    found = true;
                    break;
                }
            }
        }

        if !found {
            errors.push(CypherGuardValidationError::InvalidPropertyAccess {
                variable: access.variable.clone(),
                property: access.property.clone(),
                context: context_str.to_string(),
            });
        }
    }

    // Validate property type comparisons
    for comparison in &elements.property_comparisons {
        // Find the property definition in the schema - context-aware search
        let mut property_def = None;

        // First, try context-aware search using variable bindings
        if let Some(bound_node_label) = elements.variable_node_bindings.get(&comparison.variable) {
            // Variable is bound to a specific node label - search only within that label
            if let Some(properties) = schema.node_props.get(bound_node_label) {
                property_def = properties.iter().find(|p| p.name == comparison.property);
            }
        } else if let Some(bound_rel_type) = elements
            .variable_relationship_bindings
            .get(&comparison.variable)
        {
            // Variable is bound to a specific relationship type - search only within that type
            if let Some(properties) = schema.rel_props.get(bound_rel_type) {
                property_def = properties.iter().find(|p| p.name == comparison.property);
            }
        } else {
            // Fallback: No binding found, use global search (for backward compatibility)
            // Check node properties first
            for properties in schema.node_props.values() {
                if let Some(prop) = properties.iter().find(|p| p.name == comparison.property) {
                    property_def = Some(prop);
                    break;
                }
            }

            // If not found in nodes, check relationship properties
            if property_def.is_none() {
                for properties in schema.rel_props.values() {
                    if let Some(prop) = properties.iter().find(|p| p.name == comparison.property) {
                        property_def = Some(prop);
                        break;
                    }
                }
            }
        }

        if let Some(prop_def) = property_def {
            // OLD VALIDATION: Only check if property exists, not types
            // Type checking is now handled by the NEW type checking system
            // This avoids duplicate error messages
        } else {
            // Property not found in schema - this is also an error
            errors.push(CypherGuardValidationError::InvalidPropertyAccess {
                variable: comparison.variable.clone(),
                property: comparison.property.clone(),
                context: "property comparison".to_string(),
            });
        }
    }

    // NEW: Type checking logic (only if enabled)
    let mut type_issues = Vec::new();
    if options.type_checking != TypeCheckLevel::Off {
        for comparison in &elements.property_comparisons {
            if let Some(mut issue) = check_property_comparison_types(comparison, elements, schema, options.type_checking) {
                // In Warnings mode, downgrade all issues to Warning severity
                if options.type_checking == TypeCheckLevel::Warnings {
                    issue.severity = TypeMismatchSeverity::Warning;
                }
                type_issues.push(issue);
            }
        }
    }
    
    (errors, type_issues)
}

/// Backward compatible wrapper (no type checking)
pub fn validate_query_elements(
    elements: &QueryElements,
    schema: &DbSchema,
) -> Vec<CypherGuardValidationError> {
    let options = ValidationOptions::default();  // Type checking OFF
    let (errors, _type_issues) = validate_query_elements_with_options(elements, schema, &options);
    errors
}

// Include comprehensive type checking integration tests
#[cfg(test)]
#[path = "validation_typecheck_tests.rs"]
mod typecheck_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{PropertyValue, Query, UnwindClause, UnwindExpression};
    use crate::schema::{DbSchema, DbSchemaProperty, DbSchemaRelationshipPattern, PropertyType};

    fn create_test_schema() -> DbSchema {
        let mut schema = DbSchema::new();

        // Add node labels and properties
        schema.add_label("Person").unwrap();
        schema.add_label("Place").unwrap();

        let name_prop = DbSchemaProperty::new("name", PropertyType::STRING);
        let age_prop = DbSchemaProperty::new("age", PropertyType::INTEGER);
        let _height_prop = DbSchemaProperty::new("height", PropertyType::FLOAT);

        schema.add_node_property("Person", &name_prop).unwrap();
        schema.add_node_property("Person", &age_prop).unwrap();
        schema.add_node_property("Place", &name_prop).unwrap();

        // Add relationship types and properties
        let knows_rel = DbSchemaRelationshipPattern {
            start: "Person".to_string(),
            end: "Person".to_string(),
            rel_type: "KNOWS".to_string(),
        };
        let lives_in_rel = DbSchemaRelationshipPattern {
            start: "Person".to_string(),
            end: "Place".to_string(),
            rel_type: "LIVES_IN".to_string(),
        };

        schema.add_relationship_pattern(knows_rel).unwrap();
        schema.add_relationship_pattern(lives_in_rel).unwrap();

        let since_prop = DbSchemaProperty::new("since", PropertyType::STRING);
        schema
            .add_relationship_property("KNOWS", &since_prop)
            .unwrap();

        schema
    }

    #[test]
    fn test_extract_query_elements_basic() {
        let query = Query {
            match_clauses: vec![MatchClause {
                elements: vec![MatchElement {
                    path_function: None,
                    path_var: Some("a".to_string()),
                    pattern: vec![PatternElement::Node(NodePattern {
                        variable: Some("a".to_string()),
                        label: Some("Person".to_string()),
                        label_expression: None,
                        properties: None,
                    })],
                }],
                is_optional: false,
            }],
            merge_clauses: vec![],
            create_clauses: vec![],
            with_clauses: vec![],
            where_clauses: vec![],
            return_clauses: vec![],
            unwind_clauses: vec![],
            call_clauses: vec![],
            delete_clauses: vec![],
            remove_clauses: vec![],
            set_clauses: vec![],
            foreach_clauses: vec![],
            load_csv_clauses: vec![],
            union_queries: vec![],
        };

        let elements = extract_query_elements(&query);

        assert!(elements.node_labels.contains("Person"));
        assert!(elements.defined_variables.contains("a"));
        assert_eq!(elements.property_accesses.len(), 0);
    }

    #[test]
    fn test_extract_query_elements_with_where() {
        let query = Query {
            match_clauses: vec![MatchClause {
                elements: vec![MatchElement {
                    path_function: None,
                    path_var: Some("a".to_string()),
                    pattern: vec![PatternElement::Node(NodePattern {
                        variable: Some("a".to_string()),
                        label: Some("Person".to_string()),
                        label_expression: None,
                        properties: None,
                    })],
                }],
                is_optional: false,
            }],
            merge_clauses: vec![],
            create_clauses: vec![],
            with_clauses: vec![],
            where_clauses: vec![WhereClause {
                conditions: vec![WhereCondition::Comparison {
                    left: crate::parser::ast::PropertyValue::Identifier("a.age".to_string()),
                    operator: ">".to_string(),
                    right: crate::parser::ast::PropertyValue::Number(18),
                }],
            }],
            return_clauses: vec![],
            unwind_clauses: vec![],
            call_clauses: vec![],
            delete_clauses: vec![],
            remove_clauses: vec![],
            set_clauses: vec![],
            foreach_clauses: vec![],
            load_csv_clauses: vec![],
            union_queries: vec![],
        };

        let elements = extract_query_elements(&query);

        assert!(elements.node_labels.contains("Person"));
        assert!(elements.referenced_variables.contains("a"));
        assert_eq!(elements.property_accesses.len(), 1);
        assert_eq!(elements.property_accesses[0].variable, "a");
        assert_eq!(elements.property_accesses[0].property, "age");
        assert!(matches!(
            elements.property_accesses[0].context,
            PropertyContext::Where
        ));
    }

    #[test]
    fn test_extract_query_elements_with_return() {
        let query = Query {
            match_clauses: vec![MatchClause {
                elements: vec![MatchElement {
                    path_function: None,
                    path_var: Some("a".to_string()),
                    pattern: vec![PatternElement::Node(NodePattern {
                        variable: Some("a".to_string()),
                        label: Some("Person".to_string()),
                        label_expression: None,
                        properties: None,
                    })],
                }],
                is_optional: false,
            }],
            merge_clauses: vec![],
            create_clauses: vec![],
            with_clauses: vec![],
            where_clauses: vec![],
            return_clauses: vec![ReturnClause {
                items: vec!["a.name".to_string(), "a.age".to_string()],
                distinct: false,
                order_by: vec![],
                limit: None,
                skip: None,
            }],
            unwind_clauses: vec![],
            call_clauses: vec![],
            delete_clauses: vec![],
            remove_clauses: vec![],
            set_clauses: vec![],
            foreach_clauses: vec![],
            load_csv_clauses: vec![],
            union_queries: vec![],
        };

        let elements = extract_query_elements(&query);

        assert!(elements.node_labels.contains("Person"));
        assert!(elements.referenced_variables.contains("a"));
        assert_eq!(elements.property_accesses.len(), 2);

        let return_access: Vec<_> = elements
            .property_accesses
            .iter()
            .filter(|pa| matches!(pa.context, PropertyContext::Return))
            .collect();
        assert_eq!(return_access.len(), 2);
    }

    #[test]
    fn test_extract_query_elements_with_with() {
        let query = Query {
            match_clauses: vec![MatchClause {
                elements: vec![MatchElement {
                    path_function: None,
                    path_var: Some("a".to_string()),
                    pattern: vec![PatternElement::Node(NodePattern {
                        variable: Some("a".to_string()),
                        label: Some("Person".to_string()),
                        label_expression: None,
                        properties: None,
                    })],
                }],
                is_optional: false,
            }],
            merge_clauses: vec![],
            create_clauses: vec![],
            with_clauses: vec![WithClause {
                items: vec![WithItem {
                    expression: WithExpression::PropertyAccess {
                        variable: "a".to_string(),
                        property: "name".to_string(),
                    },
                    alias: Some("person_name".to_string()),
                }],
                distinct: false,
            }],
            where_clauses: vec![],
            return_clauses: vec![],
            unwind_clauses: vec![],
            call_clauses: vec![],
            delete_clauses: vec![],
            remove_clauses: vec![],
            set_clauses: vec![],
            foreach_clauses: vec![],
            load_csv_clauses: vec![],
            union_queries: vec![],
        };

        let elements = extract_query_elements(&query);

        assert!(elements.node_labels.contains("Person"));
        assert!(elements.referenced_variables.contains("a"));
        assert_eq!(elements.property_accesses.len(), 1);
        assert_eq!(elements.property_accesses[0].variable, "a");
        assert_eq!(elements.property_accesses[0].property, "name");
        assert!(matches!(
            elements.property_accesses[0].context,
            PropertyContext::With
        ));
    }

    #[test]
    fn test_extract_query_elements_with_unwind() {
        let query = Query {
            match_clauses: vec![],
            merge_clauses: vec![],
            create_clauses: vec![],
            with_clauses: vec![],
            where_clauses: vec![],
            return_clauses: vec![],
            unwind_clauses: vec![UnwindClause {
                expression: UnwindExpression::List(vec![
                    PropertyValue::Number(1),
                    PropertyValue::Number(2),
                ]),
                variable: "x".to_string(),
            }],
            call_clauses: vec![],
            delete_clauses: vec![],
            remove_clauses: vec![],
            set_clauses: vec![],
            foreach_clauses: vec![],
            load_csv_clauses: vec![],
            union_queries: vec![],
        };
        let elements = extract_query_elements(&query);
        assert!(elements.defined_variables.contains("x"));
    }

    #[test]
    fn test_validate_query_elements_valid() {
        let schema = create_test_schema();
        let mut elements = QueryElements::new();

        elements.add_node_label("Person".to_string());
        elements.add_defined_variable("a".to_string());
        elements.add_property_access(PropertyAccess {
            variable: "a".to_string(),
            property: "name".to_string(),
            context: PropertyContext::Return,
        });

        let errors = validate_query_elements(&elements, &schema);
        assert!(
            errors.is_empty(),
            "Expected no validation errors, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validate_query_elements_invalid_node_label() {
        let schema = create_test_schema();
        let mut elements = QueryElements::new();

        elements.add_node_label("InvalidLabel".to_string());

        let errors = validate_query_elements(&elements, &schema);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            CypherGuardValidationError::InvalidNodeLabel(_)
        ));
    }

    #[test]
    fn test_validate_query_elements_invalid_relationship_type() {
        let schema = create_test_schema();
        let mut elements = QueryElements::new();

        elements.add_relationship_type("INVALID_REL".to_string());

        let errors = validate_query_elements(&elements, &schema);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            CypherGuardValidationError::InvalidRelationshipType(_)
        ));
    }

    #[test]
    fn test_validate_query_elements_invalid_node_property() {
        let schema = create_test_schema();
        let mut elements = QueryElements::new();

        elements.add_node_property("Person".to_string(), "invalid_prop".to_string());

        let errors = validate_query_elements(&elements, &schema);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            CypherGuardValidationError::InvalidNodeProperty { .. }
        ));
    }

    #[test]
    fn test_validate_query_elements_invalid_relationship_property() {
        let schema = create_test_schema();
        let mut elements = QueryElements::new();

        elements.add_relationship_property("KNOWS".to_string(), "invalid_prop".to_string());

        let errors = validate_query_elements(&elements, &schema);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            CypherGuardValidationError::InvalidRelationshipProperty { .. }
        ));
    }

    #[test]
    fn test_validate_query_elements_invalid_property_access() {
        let schema_json = r#"{
            "node_props": {
                "Person": [
                    {"name": "name", "neo4j_type": "STRING"},
                    {"name": "age", "neo4j_type": "INTEGER"}
                ]
            },
            "rel_props": {},
            "relationships": [],
            "metadata": {"index": [], "constraint": []}
        }"#;

        let schema = DbSchema::from_json_string(schema_json).unwrap();
        let _height_prop = DbSchemaProperty::new("height", PropertyType::FLOAT);

        let mut elements = QueryElements::new();
        elements.add_property_access(PropertyAccess {
            variable: "a".to_string(),
            property: "height".to_string(),
            context: PropertyContext::Return,
        });

        let errors = validate_query_elements(&elements, &schema);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            CypherGuardValidationError::InvalidPropertyAccess { .. }
        ));
    }

    #[test]
    fn test_validate_query_elements_multiple_errors() {
        let schema = create_test_schema();
        let mut elements = QueryElements::new();

        elements.add_node_label("InvalidLabel".to_string());
        elements.add_relationship_type("INVALID_REL".to_string());
        elements.add_property_access(PropertyAccess {
            variable: "a".to_string(),
            property: "invalid_prop".to_string(),
            context: PropertyContext::Return,
        });

        let errors = validate_query_elements(&elements, &schema);
        assert_eq!(errors.len(), 3);

        let error_types: Vec<_> = errors.iter().map(std::mem::discriminant).collect();
        assert!(error_types.contains(&std::mem::discriminant(
            &CypherGuardValidationError::InvalidNodeLabel("".to_string())
        )));
        assert!(error_types.contains(&std::mem::discriminant(
            &CypherGuardValidationError::InvalidRelationshipType("".to_string())
        )));
        assert!(error_types.contains(&std::mem::discriminant(
            &CypherGuardValidationError::InvalidPropertyAccess {
                variable: "".to_string(),
                property: "".to_string(),
                context: "".to_string()
            }
        )));
    }

    #[test]
    fn test_validate_query_elements_property_access_context() {
        let schema = create_test_schema();
        let mut elements = QueryElements::new();

        // Add valid property access in different contexts
        elements.add_property_access(PropertyAccess {
            variable: "a".to_string(),
            property: "name".to_string(),
            context: PropertyContext::Where,
        });
        elements.add_property_access(PropertyAccess {
            variable: "a".to_string(),
            property: "age".to_string(),
            context: PropertyContext::Return,
        });
        elements.add_property_access(PropertyAccess {
            variable: "r".to_string(),
            property: "since".to_string(),
            context: PropertyContext::With,
        });

        let errors = validate_query_elements(&elements, &schema);
        assert!(
            errors.is_empty(),
            "Expected no validation errors for valid property access, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validate_query_elements_complex_where_condition() {
        let schema = create_test_schema();
        let mut elements = QueryElements::new();

        // Simulate complex WHERE condition extraction
        elements.add_property_access(PropertyAccess {
            variable: "a".to_string(),
            property: "age".to_string(),
            context: PropertyContext::Where,
        });
        elements.add_property_access(PropertyAccess {
            variable: "a".to_string(),
            property: "name".to_string(),
            context: PropertyContext::Where,
        });
        elements.add_property_access(PropertyAccess {
            variable: "r".to_string(),
            property: "since".to_string(),
            context: PropertyContext::Where,
        });

        let errors = validate_query_elements(&elements, &schema);
        assert!(
            errors.is_empty(),
            "Expected no validation errors for valid WHERE conditions, got: {:?}",
            errors
        );
    }

    #[test]
    fn test_validate_query_elements_with_undefined_variables() {
        let schema = create_test_schema();
        let mut elements = QueryElements::new();

        // Add property access for undefined variables - this should still validate the property
        elements.add_property_access(PropertyAccess {
            variable: "undefined_var".to_string(),
            property: "name".to_string(),
            context: PropertyContext::Return,
        });

        let errors = validate_query_elements(&elements, &schema);
        // The property "name" exists in the schema, so no error should be generated
        assert_eq!(errors.len(), 0);

        // Now test with a property that doesn't exist
        elements.add_property_access(PropertyAccess {
            variable: "undefined_var".to_string(),
            property: "nonexistent_prop".to_string(),
            context: PropertyContext::Return,
        });

        let errors = validate_query_elements(&elements, &schema);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0],
            CypherGuardValidationError::InvalidPropertyAccess { .. }
        ));
    }

    #[test]
    fn test_validate_unwind_expression_type() {
        let mut query = Query {
            match_clauses: vec![],
            merge_clauses: vec![],
            create_clauses: vec![],
            with_clauses: vec![],
            where_clauses: vec![],
            return_clauses: vec![],
            unwind_clauses: vec![UnwindClause {
                expression: UnwindExpression::Identifier("foo".to_string()),
                variable: "x".to_string(),
            }],
            call_clauses: vec![],
            delete_clauses: vec![],
            remove_clauses: vec![],
            set_clauses: vec![],
            foreach_clauses: vec![],
            load_csv_clauses: vec![],
            union_queries: vec![],
        };
        let elements = QueryElements::new();
        let schema = DbSchema::new();
        let errors = validate_query_elements(&elements, &schema);
        // All UNWIND expression types are now considered valid
        assert!(errors.is_empty());

        // Valid: list
        query.unwind_clauses = vec![UnwindClause {
            expression: UnwindExpression::List(vec![PropertyValue::Number(1)]),
            variable: "x".to_string(),
        }];
        let errors = validate_query_elements(&elements, &schema);
        assert!(errors.is_empty());

        // Valid: parameter
        query.unwind_clauses = vec![UnwindClause {
            expression: UnwindExpression::Parameter("foo".to_string()),
            variable: "x".to_string(),
        }];
        let errors = validate_query_elements(&elements, &schema);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_relationship_direction() {
        // Create a schema with ACTED_IN relationship: Person -> Movie
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();
        schema.add_label("Movie").unwrap();

        let acted_in_rel = DbSchemaRelationshipPattern {
            start: "Person".to_string(),
            end: "Movie".to_string(),
            rel_type: "ACTED_IN".to_string(),
        };
        schema.add_relationship_pattern(acted_in_rel).unwrap();

        // Test valid direction: Person -> Movie (Right direction)
        let valid_query = Query {
            match_clauses: vec![MatchClause {
                elements: vec![MatchElement {
                    path_function: None,
                    path_var: None,
                    pattern: vec![
                        PatternElement::Node(NodePattern {
                            variable: Some("a".to_string()),
                            label: Some("Person".to_string()),
                        label_expression: None,
                            properties: None,
                        }),
                        PatternElement::Relationship(RelationshipPattern::Regular(
                            RelationshipDetails {
                                variable: Some("r".to_string()),
                                direction: Direction::Right,
                                properties: None,
                                rel_type: Some("ACTED_IN".to_string()),
                                length: None,
                                where_clause: None,
                                quantifier: None,
                                is_optional: false,
                            },
                        )),
                        PatternElement::Node(NodePattern {
                            variable: Some("b".to_string()),
                            label: Some("Movie".to_string()),
                        label_expression: None,
                            properties: None,
                        }),
                    ],
                }],
                is_optional: false,
            }],
            merge_clauses: vec![],
            create_clauses: vec![],
            with_clauses: vec![],
            where_clauses: vec![],
            return_clauses: vec![],
            unwind_clauses: vec![],
            call_clauses: vec![],
            delete_clauses: vec![],
            remove_clauses: vec![],
            set_clauses: vec![],
            foreach_clauses: vec![],
            load_csv_clauses: vec![],
            union_queries: vec![],
        };

        let elements = extract_query_elements(&valid_query);
        let errors = validate_query_elements(&elements, &schema);
        assert!(
            errors.is_empty(),
            "Valid direction should not produce errors: {:?}",
            errors
        );

        // Test invalid direction: Person <- Movie (Left direction, but should be Person -> Movie)
        let invalid_query = Query {
            match_clauses: vec![MatchClause {
                elements: vec![MatchElement {
                    path_function: None,
                    path_var: None,
                    pattern: vec![
                        PatternElement::Node(NodePattern {
                            variable: Some("a".to_string()),
                            label: Some("Person".to_string()),
                        label_expression: None,
                            properties: None,
                        }),
                        PatternElement::Relationship(RelationshipPattern::Regular(
                            RelationshipDetails {
                                variable: Some("r".to_string()),
                                direction: Direction::Left,
                                properties: None,
                                rel_type: Some("ACTED_IN".to_string()),
                                length: None,
                                where_clause: None,
                                quantifier: None,
                                is_optional: false,
                            },
                        )),
                        PatternElement::Node(NodePattern {
                            variable: Some("b".to_string()),
                            label: Some("Movie".to_string()),
                        label_expression: None,
                            properties: None,
                        }),
                    ],
                }],
                is_optional: false,
            }],
            merge_clauses: vec![],
            create_clauses: vec![],
            with_clauses: vec![],
            where_clauses: vec![],
            return_clauses: vec![],
            unwind_clauses: vec![],
            call_clauses: vec![],
            delete_clauses: vec![],
            remove_clauses: vec![],
            set_clauses: vec![],
            foreach_clauses: vec![],
            load_csv_clauses: vec![],
            union_queries: vec![],
        };

        let elements = extract_query_elements(&invalid_query);
        let errors = validate_query_elements(&elements, &schema);
        assert!(
            !errors.is_empty(),
            "Invalid direction should produce errors"
        );
        assert!(errors
            .iter()
            .any(|e| matches!(e, CypherGuardValidationError::InvalidRelationship(_))));
    }

    // === QPP (Quantified Path Pattern) Validation Tests ===

    #[test]
    fn test_qpp_basic_validation() {
        // Test basic QPP: ((a)-[r:KNOWS]->(b)){1,3}
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();

        let name_prop = DbSchemaProperty::new("name", PropertyType::STRING);
        schema.add_node_property("Person", &name_prop).unwrap();

        let knows_rel = DbSchemaRelationshipPattern::new("Person", "Person", "KNOWS");
        schema.add_relationship_pattern(knows_rel).unwrap();

        let qpp = QuantifiedPathPattern {
            pattern: vec![
                PatternElement::Node(NodePattern {
                    variable: Some("a".to_string()),
                    label: Some("Person".to_string()),
                        label_expression: None,
                    properties: None,
                }),
                PatternElement::Relationship(RelationshipPattern::Regular(RelationshipDetails {
                    variable: Some("r".to_string()),
                    direction: Direction::Right,
                    properties: None,
                    rel_type: Some("KNOWS".to_string()),
                    length: None,
                    where_clause: None,
                    quantifier: None,
                    is_optional: false,
                })),
                PatternElement::Node(NodePattern {
                    variable: Some("b".to_string()),
                    label: Some("Person".to_string()),
                        label_expression: None,
                    properties: None,
                }),
            ],
            min: Some(1),
            max: Some(3),
            where_clause: None,
            path_variable: Some("p".to_string()),
        };

        let query = Query {
            match_clauses: vec![MatchClause {
                elements: vec![MatchElement {
                    path_function: None,
                    path_var: None,
                    pattern: vec![PatternElement::QuantifiedPathPattern(qpp)],
                }],
                is_optional: false,
            }],
            merge_clauses: vec![],
            create_clauses: vec![],
            with_clauses: vec![],
            where_clauses: vec![],
            return_clauses: vec![],
            unwind_clauses: vec![],
            call_clauses: vec![],
            delete_clauses: vec![],
            remove_clauses: vec![],
            set_clauses: vec![],
            foreach_clauses: vec![],
            load_csv_clauses: vec![],
            union_queries: vec![],
        };

        let elements = extract_query_elements(&query);
        let errors = validate_query_elements(&elements, &schema);

        // Should have no errors - valid QPP
        if !errors.is_empty() {
            println!("Unexpected errors: {:?}", errors);
        }
        assert!(errors.is_empty(), "Valid QPP should not produce errors");

        // Verify extraction worked correctly
        assert!(elements.defined_variables.contains("p"), "Path variable should be defined");
        assert!(elements.defined_variables.contains("a"), "Node variable a should be defined");
        assert!(elements.defined_variables.contains("b"), "Node variable b should be defined");
        assert!(elements.defined_variables.contains("r"), "Relationship variable r should be defined");
        assert!(elements.node_labels.contains("Person"), "Person label should be extracted");
        assert!(elements.relationship_types.contains("KNOWS"), "KNOWS relationship should be extracted");
    }

    #[test]
    fn test_qpp_invalid_relationship() {
        // Test QPP with invalid relationship type
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();

        let knows_rel = DbSchemaRelationshipPattern::new("Person", "Person", "KNOWS");
        schema.add_relationship_pattern(knows_rel).unwrap();

        let qpp = QuantifiedPathPattern {
            pattern: vec![
                PatternElement::Node(NodePattern {
                    variable: Some("a".to_string()),
                    label: Some("Person".to_string()),
                        label_expression: None,
                    properties: None,
                }),
                PatternElement::Relationship(RelationshipPattern::Regular(RelationshipDetails {
                    variable: None,
                    direction: Direction::Right,
                    properties: None,
                    rel_type: Some("INVALID_REL".to_string()),
                    length: None,
                    where_clause: None,
                    quantifier: None,
                    is_optional: false,
                })),
                PatternElement::Node(NodePattern {
                    variable: Some("b".to_string()),
                    label: Some("Person".to_string()),
                        label_expression: None,
                    properties: None,
                }),
            ],
            min: Some(1),
            max: Some(3),
            where_clause: None,
            path_variable: None,
        };

        let query = Query {
            match_clauses: vec![MatchClause {
                elements: vec![MatchElement {
                    path_function: None,
                    path_var: None,
                    pattern: vec![PatternElement::QuantifiedPathPattern(qpp)],
                }],
                is_optional: false,
            }],
            merge_clauses: vec![],
            create_clauses: vec![],
            with_clauses: vec![],
            where_clauses: vec![],
            return_clauses: vec![],
            unwind_clauses: vec![],
            call_clauses: vec![],
            delete_clauses: vec![],
            remove_clauses: vec![],
            set_clauses: vec![],
            foreach_clauses: vec![],
            load_csv_clauses: vec![],
            union_queries: vec![],
        };

        let elements = extract_query_elements(&query);
        let errors = validate_query_elements(&elements, &schema);

        // Should have errors - invalid relationship type
        assert!(!errors.is_empty(), "Invalid relationship in QPP should produce errors");
        assert!(
            errors.iter().any(|e| matches!(e, CypherGuardValidationError::InvalidRelationshipType(_))),
            "Should have InvalidRelationshipType error"
        );
    }

    #[test]
    fn test_qpp_with_properties() {
        // Test QPP with node and relationship properties
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();

        let name_prop = DbSchemaProperty::new("name", PropertyType::STRING);
        schema.add_node_property("Person", &name_prop).unwrap();

        let knows_rel = DbSchemaRelationshipPattern::new("Person", "Person", "KNOWS");
        schema.add_relationship_pattern(knows_rel).unwrap();

        let since_prop = DbSchemaProperty::new("since", PropertyType::INTEGER);
        schema.add_relationship_property("KNOWS", &since_prop).unwrap();

        let qpp = QuantifiedPathPattern {
            pattern: vec![
                PatternElement::Node(NodePattern {
                    variable: Some("a".to_string()),
                    label: Some("Person".to_string()),
                        label_expression: None,
                    properties: Some(vec![Property {
                        key: "name".to_string(),
                        value: PropertyValue::String("Alice".to_string()),
                    }]),
                }),
                PatternElement::Relationship(RelationshipPattern::Regular(RelationshipDetails {
                    variable: None,
                    direction: Direction::Right,
                    properties: Some(vec![Property {
                        key: "since".to_string(),
                        value: PropertyValue::Number(2020),
                    }]),
                    rel_type: Some("KNOWS".to_string()),
                    length: None,
                    where_clause: None,
                    quantifier: None,
                    is_optional: false,
                })),
                PatternElement::Node(NodePattern {
                    variable: Some("b".to_string()),
                    label: Some("Person".to_string()),
                        label_expression: None,
                    properties: None,
                }),
            ],
            min: Some(1),
            max: Some(5),
            where_clause: None,
            path_variable: None,
        };

        let query = Query {
            match_clauses: vec![MatchClause {
                elements: vec![MatchElement {
                    path_function: None,
                    path_var: None,
                    pattern: vec![PatternElement::QuantifiedPathPattern(qpp)],
                }],
                is_optional: false,
            }],
            merge_clauses: vec![],
            create_clauses: vec![],
            with_clauses: vec![],
            where_clauses: vec![],
            return_clauses: vec![],
            unwind_clauses: vec![],
            call_clauses: vec![],
            delete_clauses: vec![],
            remove_clauses: vec![],
            set_clauses: vec![],
            foreach_clauses: vec![],
            load_csv_clauses: vec![],
            union_queries: vec![],
        };

        let elements = extract_query_elements(&query);
        let errors = validate_query_elements(&elements, &schema);

        // Should have no errors - all properties are valid
        if !errors.is_empty() {
            println!("Unexpected errors: {:?}", errors);
        }
        assert!(errors.is_empty(), "Valid QPP with properties should not produce errors");

        // Verify property extraction
        assert!(elements.node_properties.get("Person").unwrap().contains("name"));
        assert!(elements.relationship_properties.get("KNOWS").unwrap().contains("since"));
    }

    #[test]
    fn test_qpp_unbounded() {
        // Test QPP with no max (unbounded): ((a)-[r:KNOWS]->(b)){1,}
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();

        let knows_rel = DbSchemaRelationshipPattern::new("Person", "Person", "KNOWS");
        schema.add_relationship_pattern(knows_rel).unwrap();

        let qpp = QuantifiedPathPattern {
            pattern: vec![
                PatternElement::Node(NodePattern {
                    variable: Some("a".to_string()),
                    label: Some("Person".to_string()),
                        label_expression: None,
                    properties: None,
                }),
                PatternElement::Relationship(RelationshipPattern::Regular(RelationshipDetails {
                    variable: None,
                    direction: Direction::Right,
                    properties: None,
                    rel_type: Some("KNOWS".to_string()),
                    length: None,
                    where_clause: None,
                    quantifier: None,
                    is_optional: false,
                })),
                PatternElement::Node(NodePattern {
                    variable: Some("b".to_string()),
                    label: Some("Person".to_string()),
                        label_expression: None,
                    properties: None,
                }),
            ],
            min: Some(1),
            max: None, // Unbounded
            where_clause: None,
            path_variable: None,
        };

        let query = Query {
            match_clauses: vec![MatchClause {
                elements: vec![MatchElement {
                    path_function: None,
                    path_var: None,
                    pattern: vec![PatternElement::QuantifiedPathPattern(qpp)],
                }],
                is_optional: false,
            }],
            merge_clauses: vec![],
            create_clauses: vec![],
            with_clauses: vec![],
            where_clauses: vec![],
            return_clauses: vec![],
            unwind_clauses: vec![],
            call_clauses: vec![],
            delete_clauses: vec![],
            remove_clauses: vec![],
            set_clauses: vec![],
            foreach_clauses: vec![],
            load_csv_clauses: vec![],
            union_queries: vec![],
        };

        let elements = extract_query_elements(&query);
        let errors = validate_query_elements(&elements, &schema);

        // Unbounded QPP should be valid
        assert!(errors.is_empty(), "Unbounded QPP should be valid");
    }

    #[test]
    fn test_qpp_zero_or_more() {
        // Test QPP with min=0 (zero or more): ((a)-[r:KNOWS]->(b)){0,}
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();

        let knows_rel = DbSchemaRelationshipPattern::new("Person", "Person", "KNOWS");
        schema.add_relationship_pattern(knows_rel).unwrap();

        let qpp = QuantifiedPathPattern {
            pattern: vec![
                PatternElement::Node(NodePattern {
                    variable: Some("a".to_string()),
                    label: Some("Person".to_string()),
                        label_expression: None,
                    properties: None,
                }),
                PatternElement::Relationship(RelationshipPattern::Regular(RelationshipDetails {
                    variable: None,
                    direction: Direction::Right,
                    properties: None,
                    rel_type: Some("KNOWS".to_string()),
                    length: None,
                    where_clause: None,
                    quantifier: None,
                    is_optional: false,
                })),
                PatternElement::Node(NodePattern {
                    variable: Some("b".to_string()),
                    label: Some("Person".to_string()),
                        label_expression: None,
                    properties: None,
                }),
            ],
            min: Some(0),
            max: None,
            where_clause: None,
            path_variable: None,
        };

        let query = Query {
            match_clauses: vec![MatchClause {
                elements: vec![MatchElement {
                    path_function: None,
                    path_var: None,
                    pattern: vec![PatternElement::QuantifiedPathPattern(qpp)],
                }],
                is_optional: false,
            }],
            merge_clauses: vec![],
            create_clauses: vec![],
            with_clauses: vec![],
            where_clauses: vec![],
            return_clauses: vec![],
            unwind_clauses: vec![],
            call_clauses: vec![],
            delete_clauses: vec![],
            remove_clauses: vec![],
            set_clauses: vec![],
            foreach_clauses: vec![],
            load_csv_clauses: vec![],
            union_queries: vec![],
        };

        let elements = extract_query_elements(&query);
        let errors = validate_query_elements(&elements, &schema);

        // Zero or more QPP should be valid
        assert!(errors.is_empty(), "Zero or more QPP should be valid");
    }

    #[test]
    fn test_qpp_complex_pattern() {
        // Test QPP with complex multi-hop pattern: ((a)-[:KNOWS]->(b)-[:WORKS_AT]->(c)){1,2}
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();
        schema.add_label("Company").unwrap();

        let knows_rel = DbSchemaRelationshipPattern::new("Person", "Person", "KNOWS");
        schema.add_relationship_pattern(knows_rel).unwrap();

        let works_at_rel = DbSchemaRelationshipPattern::new("Person", "Company", "WORKS_AT");
        schema.add_relationship_pattern(works_at_rel).unwrap();

        let qpp = QuantifiedPathPattern {
            pattern: vec![
                PatternElement::Node(NodePattern {
                    variable: Some("a".to_string()),
                    label: Some("Person".to_string()),
                        label_expression: None,
                    properties: None,
                }),
                PatternElement::Relationship(RelationshipPattern::Regular(RelationshipDetails {
                    variable: None,
                    direction: Direction::Right,
                    properties: None,
                    rel_type: Some("KNOWS".to_string()),
                    length: None,
                    where_clause: None,
                    quantifier: None,
                    is_optional: false,
                })),
                PatternElement::Node(NodePattern {
                    variable: Some("b".to_string()),
                    label: Some("Person".to_string()),
                        label_expression: None,
                    properties: None,
                }),
                PatternElement::Relationship(RelationshipPattern::Regular(RelationshipDetails {
                    variable: None,
                    direction: Direction::Right,
                    properties: None,
                    rel_type: Some("WORKS_AT".to_string()),
                    length: None,
                    where_clause: None,
                    quantifier: None,
                    is_optional: false,
                })),
                PatternElement::Node(NodePattern {
                    variable: Some("c".to_string()),
                    label: Some("Company".to_string()),
                        label_expression: None,
                    properties: None,
                }),
            ],
            min: Some(1),
            max: Some(2),
            where_clause: None,
            path_variable: None,
        };

        let query = Query {
            match_clauses: vec![MatchClause {
                elements: vec![MatchElement {
                    path_function: None,
                    path_var: None,
                    pattern: vec![PatternElement::QuantifiedPathPattern(qpp)],
                }],
                is_optional: false,
            }],
            merge_clauses: vec![],
            create_clauses: vec![],
            with_clauses: vec![],
            where_clauses: vec![],
            return_clauses: vec![],
            unwind_clauses: vec![],
            call_clauses: vec![],
            delete_clauses: vec![],
            remove_clauses: vec![],
            set_clauses: vec![],
            foreach_clauses: vec![],
            load_csv_clauses: vec![],
            union_queries: vec![],
        };

        let elements = extract_query_elements(&query);
        let errors = validate_query_elements(&elements, &schema);

        // Complex multi-hop QPP should be valid
        if !errors.is_empty() {
            println!("Unexpected errors: {:?}", errors);
        }
        assert!(errors.is_empty(), "Complex multi-hop QPP should be valid");

        // Verify all labels and relationships extracted
        assert!(elements.node_labels.contains("Person"));
        assert!(elements.node_labels.contains("Company"));
        assert!(elements.relationship_types.contains("KNOWS"));
        assert!(elements.relationship_types.contains("WORKS_AT"));
    }

    // === shortestPath() Validation Tests ===

    #[test]
    fn test_shortest_path_validation() {
        // Test that shortestPath patterns are validated correctly
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();

        let knows_rel = DbSchemaRelationshipPattern::new("Person", "Person", "KNOWS");
        schema.add_relationship_pattern(knows_rel).unwrap();

        // Parse a query with shortestPath
        let query_str = "MATCH p = shortestPath((a:Person)-[:KNOWS*]-(b:Person)) RETURN p";
        let result = crate::parse_query(query_str);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let query = result.unwrap();
        let elements = extract_query_elements(&query);
        let errors = validate_query_elements(&elements, &schema);

        // Should have no errors - valid shortestPath
        assert!(errors.is_empty(), "Valid shortestPath should not produce errors: {:?}", errors);

        // Verify extraction worked
        assert!(elements.node_labels.contains("Person"));
        assert!(elements.relationship_types.contains("KNOWS"));
    }

    #[test]
    fn test_shortest_path_invalid_relationship() {
        // Test that invalid relationships in shortestPath are caught
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();

        let knows_rel = DbSchemaRelationshipPattern::new("Person", "Person", "KNOWS");
        schema.add_relationship_pattern(knows_rel).unwrap();

        // Parse a query with invalid relationship type
        let query_str = "MATCH p = shortestPath((a:Person)-[:INVALID*]-(b:Person)) RETURN p";
        let result = crate::parse_query(query_str);
        assert!(result.is_ok());

        let query = result.unwrap();
        let elements = extract_query_elements(&query);
        let errors = validate_query_elements(&elements, &schema);

        // Should have error for invalid relationship type
        assert!(!errors.is_empty(), "Invalid relationship in shortestPath should produce errors");
        assert!(
            errors.iter().any(|e| matches!(e, CypherGuardValidationError::InvalidRelationshipType(_))),
            "Should have InvalidRelationshipType error"
        );
    }

    #[test]
    fn test_all_shortest_paths_validation() {
        // Test allShortestPaths validation
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();

        let knows_rel = DbSchemaRelationshipPattern::new("Person", "Person", "KNOWS");
        schema.add_relationship_pattern(knows_rel).unwrap();

        let query_str = "MATCH p = allShortestPaths((a:Person)-[:KNOWS*]-(b:Person)) RETURN p";
        let result = crate::parse_query(query_str);
        assert!(result.is_ok());

        let query = result.unwrap();
        let elements = extract_query_elements(&query);
        let errors = validate_query_elements(&elements, &schema);

        // Should have no errors
        assert!(errors.is_empty(), "Valid allShortestPaths should not produce errors: {:?}", errors);
    }

    // === Path Function Validation Tests ===

    #[test]
    fn test_path_function_length_validation() {
        // Test that length() function works with path variables
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();

        let knows_rel = DbSchemaRelationshipPattern::new("Person", "Person", "KNOWS");
        schema.add_relationship_pattern(knows_rel).unwrap();

        let query_str = "MATCH p = (a:Person)-[:KNOWS*]-(b:Person) WHERE length(p) < 5 RETURN p";
        let result = crate::parse_query(query_str);
        assert!(result.is_ok(), "Failed to parse: {:?}", result.err());

        let query = result.unwrap();
        let elements = extract_query_elements(&query);
        let errors = validate_query_elements(&elements, &schema);

        // Should have no errors
        assert!(errors.is_empty(), "length() function should not produce validation errors: {:?}", errors);
    }

    #[test]
    fn test_path_function_nodes_validation() {
        // Test that nodes() function works
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();

        let knows_rel = DbSchemaRelationshipPattern::new("Person", "Person", "KNOWS");
        schema.add_relationship_pattern(knows_rel).unwrap();

        let query_str = "MATCH p = (a:Person)-[:KNOWS*]-(b:Person) RETURN nodes(p)";
        let result = crate::parse_query(query_str);
        assert!(result.is_ok());

        let query = result.unwrap();
        let elements = extract_query_elements(&query);
        let errors = validate_query_elements(&elements, &schema);

        // Should have no errors
        assert!(errors.is_empty(), "nodes() function should not produce validation errors: {:?}", errors);
    }

    #[test]
    fn test_path_function_relationships_validation() {
        // Test that relationships() function works
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();

        let knows_rel = DbSchemaRelationshipPattern::new("Person", "Person", "KNOWS");
        schema.add_relationship_pattern(knows_rel).unwrap();

        let query_str = "MATCH p = (a:Person)-[:KNOWS*]-(b:Person) RETURN relationships(p)";
        let result = crate::parse_query(query_str);
        assert!(result.is_ok());

        let query = result.unwrap();
        let elements = extract_query_elements(&query);
        let errors = validate_query_elements(&elements, &schema);

        // Should have no errors
        assert!(errors.is_empty(), "relationships() function should not produce validation errors: {:?}", errors);
    }

    #[test]
    fn test_path_functions_combined_validation() {
        // Test combined path functions
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();

        let knows_rel = DbSchemaRelationshipPattern::new("Person", "Person", "KNOWS");
        schema.add_relationship_pattern(knows_rel).unwrap();

        let query_str = "MATCH p = (a:Person)-[:KNOWS*]-(b:Person) WHERE length(p) <= 3 RETURN nodes(p), relationships(p), length(p)";
        let result = crate::parse_query(query_str);
        assert!(result.is_ok());

        let query = result.unwrap();
        let elements = extract_query_elements(&query);
        let errors = validate_query_elements(&elements, &schema);

        // Should have no errors
        assert!(errors.is_empty(), "Combined path functions should not produce validation errors: {:?}", errors);
    }

    // === Label Expression Validation Tests (Neo4j 5.x) ===

    #[test]
    fn test_label_expression_or_validation() {
        // Test OR label expression - both labels should be validated
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();
        schema.add_label("Company").unwrap();

        let query_str = "MATCH (n:Person|Company) RETURN n";
        let result = crate::parse_query(query_str);
        assert!(result.is_ok());

        let query = result.unwrap();
        let elements = extract_query_elements(&query);
        let errors = validate_query_elements(&elements, &schema);

        // Should have no errors - both labels exist
        assert!(errors.is_empty(), "OR label expression with valid labels should have no errors: {:?}", errors);

        // Verify both labels were extracted
        assert!(elements.node_labels.contains("Person"));
        assert!(elements.node_labels.contains("Company"));
    }

    #[test]
    fn test_label_expression_and_validation() {
        // Test AND label expression - both labels should be validated
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();
        schema.add_label("Manager").unwrap();

        let query_str = "MATCH (n:Person&Manager) RETURN n";
        let result = crate::parse_query(query_str);
        assert!(result.is_ok());

        let query = result.unwrap();
        let elements = extract_query_elements(&query);
        let errors = validate_query_elements(&elements, &schema);

        // Should have no errors - both labels exist
        assert!(errors.is_empty(), "AND label expression with valid labels should have no errors: {:?}", errors);

        // Verify both labels were extracted
        assert!(elements.node_labels.contains("Person"));
        assert!(elements.node_labels.contains("Manager"));
    }

    #[test]
    fn test_label_expression_not_validation() {
        // Test NOT label expression - negated label should still be validated
        let mut schema = DbSchema::new();
        schema.add_label("Deleted").unwrap();

        let query_str = "MATCH (n:!Deleted) RETURN n";
        let result = crate::parse_query(query_str);
        assert!(result.is_ok());

        let query = result.unwrap();
        let elements = extract_query_elements(&query);
        let errors = validate_query_elements(&elements, &schema);

        // Should have no errors - label exists
        assert!(errors.is_empty(), "NOT label expression with valid label should have no errors: {:?}", errors);

        // Verify label was extracted
        assert!(elements.node_labels.contains("Deleted"));
    }

    #[test]
    fn test_label_expression_invalid_label() {
        // Test that invalid labels in expressions are caught
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();
        // Note: InvalidLabel is not added to schema

        let query_str = "MATCH (n:Person|InvalidLabel) RETURN n";
        let result = crate::parse_query(query_str);
        assert!(result.is_ok());

        let query = result.unwrap();
        let elements = extract_query_elements(&query);
        let errors = validate_query_elements(&elements, &schema);

        // Should have an error for InvalidLabel
        assert!(!errors.is_empty(), "Invalid label in OR expression should produce error");
        let has_invalid_label_error = errors.iter().any(|e| {
            matches!(e, CypherGuardValidationError::InvalidNodeLabel(label) if label == "InvalidLabel")
        });
        assert!(has_invalid_label_error, "Should have InvalidNodeLabel error");
    }

    #[test]
    fn test_label_expression_with_properties() {
        // Test that label expression with properties validates both label and properties
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();
        schema.add_label("Company").unwrap();

        let name_prop = DbSchemaProperty::new("name", PropertyType::STRING);
        schema.add_node_property("Person", &name_prop).unwrap();
        schema.add_node_property("Company", &name_prop).unwrap();

        let query_str = "MATCH (n:Person|Company {name: 'Alice'}) RETURN n";
        let result = crate::parse_query(query_str);
        assert!(result.is_ok());

        let query = result.unwrap();
        let elements = extract_query_elements(&query);
        let errors = validate_query_elements(&elements, &schema);

        // Should have no errors - both labels exist and have the name property
        assert!(errors.is_empty(), "Label expression with valid properties should have no errors: {:?}", errors);

        // Verify properties were extracted for both labels
        assert!(elements.node_properties.get("Person").unwrap().contains("name"));
        assert!(elements.node_properties.get("Company").unwrap().contains("name"));
    }

    #[test]
    fn test_label_expression_complex_pattern() {
        // Test complex label expression in a full pattern
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();
        schema.add_label("Company").unwrap();
        schema.add_label("Organization").unwrap();

        let works_for_rel = DbSchemaRelationshipPattern::new("Person", "Company", "WORKS_FOR");
        schema.add_relationship_pattern(works_for_rel).unwrap();

        let query_str = "MATCH (a:Person)-[:WORKS_FOR]->(b:Company|Organization) RETURN a, b";
        let result = crate::parse_query(query_str);
        assert!(result.is_ok());

        let query = result.unwrap();
        let elements = extract_query_elements(&query);
        let errors = validate_query_elements(&elements, &schema);

        // Should have no errors
        assert!(errors.is_empty(), "Complex pattern with label expressions should have no errors: {:?}", errors);

        // Verify all labels were extracted
        assert!(elements.node_labels.contains("Person"));
        assert!(elements.node_labels.contains("Company"));
        assert!(elements.node_labels.contains("Organization"));
        assert!(elements.relationship_types.contains("WORKS_FOR"));
    }
}
