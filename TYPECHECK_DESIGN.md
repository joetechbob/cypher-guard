# Cypher-Guard Type Checking Implementation

## Overview

Add opt-in type checking to cypher-guard to detect type mismatches in WHERE clause predicates that cause silent failures in Neo4j.

**Primary Use Case:** Detect `String` properties compared to `date()` functions, which returns 0 results instead of an error.

---

## User Requirements

1. **Default:** Type checking OFF (backward compatible)
2. **Opt-in:** `type_checking="off"|"warnings"|"strict"`
3. **Scope:** All type checks (Date/String, Numeric, Collections, Boolean)
4. **Strategy:** Blocklist (only flag KNOWN problematic patterns)

---

## API Design

### Python API

```python
from cypher_guard import validate_cypher

# Default: no type checking (backward compatible)
result = validate_cypher(query, schema)

# With type checking
result = validate_cypher(query, schema, type_checking="warnings")
result = validate_cypher(query, schema, type_checking="strict")
```

### Result Format

```python
{
    "valid": bool,
    "errors": List[str],           # Schema errors
    "warnings": List[str],          # Schema warnings
    "type_warnings": List[str],     # NEW: Type mismatches (warnings mode)
    "type_errors": List[str],       # NEW: Type mismatches (strict mode)
}
```

---

## Type Checking Rules (Blocklist Approach)

### ERROR Level (Silent Failures)
```rust
(String, Date) | (Date, String)           // ps.valid_from <= date('2025-01-01')
(String, DateTime) | (DateTime, String)   // timestamp >= '2025-01-01T00:00:00'
(String, Boolean) | (Boolean, String)     // active = 'true'
```

### WARNING Level (Likely Unintentional)
```rust
(String, Integer) | (Integer, String)     // employee_id > 1000
(String, Float) | (Float, String)         // budget = '50000.00'
```

### ALLOWED (Always)
```rust
(Integer, Float) | (Float, Integer)       // Neo4j handles coercion
(Date, DateTime) | (DateTime, Date)       // Works with minor precision difference
(Unknown, *)                              // Don't block unknown types
```

---

## Implementation Phases

### Phase 1: Core Infrastructure

**1.1 Type System (Rust)**

File: `rust/src/types.rs` (NEW)

```rust
/// Type checking severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeCheckLevel {
    Off,
    Warnings,
    Strict,
}

/// Neo4j property types for type checking
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Neo4jType {
    String,
    Integer,
    Float,
    Boolean,
    Date,
    DateTime,
    LocalTime,
    Time,
    Duration,
    Point,
    List(Box<Neo4jType>),
    Unknown,  // For types we don't recognize
}

/// Type mismatch severity
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeMismatchSeverity {
    Error,    // Silent failure or wrong results
    Warning,  // Likely unintentional
}

/// Type warning/error message
#[derive(Debug, Clone)]
pub struct TypeIssue {
    pub severity: TypeMismatchSeverity,
    pub message: String,
    pub suggestion: Option<String>,
}
```

**1.2 Validation Options**

File: `rust/src/validation.rs` (EXTEND)

```rust
/// Validation configuration options
#[derive(Debug, Clone)]
pub struct ValidationOptions {
    pub type_checking: TypeCheckLevel,
}

impl Default for ValidationOptions {
    fn default() -> Self {
        Self {
            type_checking: TypeCheckLevel::Off,  // Backward compatible
        }
    }
}

/// Extended validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub type_warnings: Vec<String>,  // NEW
    pub type_errors: Vec<String>,    // NEW
}
```

**1.3 Variable Context Tracking**

File: `rust/src/context.rs` (NEW)

```rust
use std::collections::HashMap;

/// Tracks variable bindings during AST traversal
#[derive(Debug, Clone, Default)]
pub struct ValidationContext {
    /// Maps variable names to their node labels
    /// Example: {"ps": "ProjectStaffing", "e": "Employee"}
    pub variable_labels: HashMap<String, String>,
    
    /// Current validation options
    pub options: ValidationOptions,
    
    /// Accumulated type issues
    pub type_issues: Vec<TypeIssue>,
}

impl ValidationContext {
    pub fn new(options: ValidationOptions) -> Self {
        Self {
            variable_labels: HashMap::new(),
            options,
            type_issues: Vec::new(),
        }
    }
    
    pub fn bind_variable(&mut self, var: String, label: String) {
        self.variable_labels.insert(var, label);
    }
    
    pub fn get_label(&self, var: &str) -> Option<&String> {
        self.variable_labels.get(var)
    }
    
    pub fn add_type_issue(&mut self, issue: TypeIssue) {
        self.type_issues.push(issue);
    }
}
```

---

### Phase 2: Type Checking Logic

**2.1 Type Compatibility Check**

File: `rust/src/type_checker.rs` (NEW)

```rust
use crate::types::*;

/// Check if two types have a known problematic comparison
/// Returns None if compatible, Some(severity) if incompatible
pub fn check_type_compatibility(
    lhs: &Neo4jType,
    rhs: &Neo4jType,
) -> Option<TypeMismatchSeverity> {
    // If either type is Unknown, allow it (conservative)
    if matches!(lhs, Neo4jType::Unknown) || matches!(rhs, Neo4jType::Unknown) {
        return None;
    }
    
    match (lhs, rhs) {
        // ERROR: Silent failures in Neo4j
        (Neo4jType::String, Neo4jType::Date) | (Neo4jType::Date, Neo4jType::String) => {
            Some(TypeMismatchSeverity::Error)
        }
        (Neo4jType::String, Neo4jType::DateTime) | (Neo4jType::DateTime, Neo4jType::String) => {
            Some(TypeMismatchSeverity::Error)
        }
        (Neo4jType::String, Neo4jType::Boolean) | (Neo4jType::Boolean, Neo4jType::String) => {
            Some(TypeMismatchSeverity::Error)
        }
        
        // WARNING: Likely unintentional
        (Neo4jType::String, Neo4jType::Integer) | (Neo4jType::Integer, Neo4jType::String) => {
            Some(TypeMismatchSeverity::Warning)
        }
        (Neo4jType::String, Neo4jType::Float) | (Neo4jType::Float, Neo4jType::String) => {
            Some(TypeMismatchSeverity::Warning)
        }
        
        // ALLOW: All other combinations (including Integer↔Float, Date↔DateTime)
        _ => None,
    }
}

/// Parse Neo4j type string from schema
pub fn parse_neo4j_type(type_str: &str) -> Neo4jType {
    match type_str.to_lowercase().trim() {
        "string" => Neo4jType::String,
        "integer" | "int" | "long" => Neo4jType::Integer,
        "float" | "double" => Neo4jType::Float,
        "boolean" | "bool" => Neo4jType::Boolean,
        "date" => Neo4jType::Date,
        "datetime" | "zoneddatetime" => Neo4jType::DateTime,
        "localtime" => Neo4jType::LocalTime,
        "time" => Neo4jType::Time,
        "duration" => Neo4jType::Duration,
        "point" => Neo4jType::Point,
        _ => Neo4jType::Unknown,  // Conservative: allow unknown types
    }
}

/// Build type mismatch message
pub fn build_type_issue_message(
    var: &str,
    prop: &str,
    prop_type: &Neo4jType,
    compared_to: &Neo4jType,
    operator: &str,
) -> TypeIssue {
    let message = format!(
        "Type mismatch: {}.{} is {:?}, compared with {:?} using {}",
        var, prop, prop_type, compared_to, operator
    );
    
    let suggestion = match (prop_type, compared_to) {
        (Neo4jType::String, Neo4jType::Date) => {
            Some(format!(
                "Convert string to date: WHERE date({}.{}) {} date(...)",
                var, prop, operator
            ))
        }
        (Neo4jType::Date, Neo4jType::String) => {
            Some(format!(
                "Use date() function: WHERE {}.{} {} date('YYYY-MM-DD')",
                var, prop, operator
            ))
        }
        _ => None,
    };
    
    TypeIssue {
        severity: check_type_compatibility(prop_type, compared_to).unwrap(),
        message,
        suggestion,
    }
}
```

**2.2 WHERE Clause Analysis**

File: `rust/src/where_analyzer.rs` (NEW)

```rust
use crate::context::ValidationContext;
use crate::schema::DbSchema;
use crate::type_checker::*;

/// Analyze WHERE clause for type mismatches
pub fn analyze_where_clause(
    where_expr: &WhereClause,  // AST node
    schema: &DbSchema,
    context: &mut ValidationContext,
) {
    // Skip if type checking is off
    if context.options.type_checking == TypeCheckLevel::Off {
        return;
    }
    
    // Traverse WHERE clause AST and check comparisons
    visit_comparisons(where_expr, schema, context);
}

fn visit_comparisons(
    expr: &Expression,
    schema: &DbSchema,
    context: &mut ValidationContext,
) {
    match expr {
        Expression::Comparison { lhs, operator, rhs } => {
            check_comparison(lhs, operator, rhs, schema, context);
        }
        Expression::And(left, right) | Expression::Or(left, right) => {
            visit_comparisons(left, schema, context);
            visit_comparisons(right, schema, context);
        }
        _ => {}
    }
}

fn check_comparison(
    lhs: &Expression,
    operator: &str,
    rhs: &Expression,
    schema: &DbSchema,
    context: &mut ValidationContext,
) {
    // Extract types from both sides
    let lhs_type = infer_expression_type(lhs, schema, context);
    let rhs_type = infer_expression_type(rhs, schema, context);
    
    // Check compatibility
    if let (Some(lhs_t), Some(rhs_t)) = (lhs_type, rhs_type) {
        if let Some(severity) = check_type_compatibility(&lhs_t, &rhs_t) {
            // Build helpful message
            let issue = build_type_issue_message(
                &extract_var_name(lhs),
                &extract_prop_name(lhs),
                &lhs_t,
                &rhs_t,
                operator,
            );
            
            context.add_type_issue(issue);
        }
    }
}

/// Infer type of an expression
fn infer_expression_type(
    expr: &Expression,
    schema: &DbSchema,
    context: &ValidationContext,
) -> Option<Neo4jType> {
    match expr {
        // Property access: var.property
        Expression::Property(var, prop) => {
            // Look up variable's label
            let label = context.get_label(var)?;
            
            // Look up property type in schema
            schema.get_property_type(label, prop)
                .map(|type_str| parse_neo4j_type(type_str))
        }
        
        // Function call: date('2025-01-01')
        Expression::FunctionCall { name, .. } => {
            match name.to_lowercase().as_str() {
                "date" => Some(Neo4jType::Date),
                "datetime" => Some(Neo4jType::DateTime),
                "tointeger" => Some(Neo4jType::Integer),
                "tofloat" => Some(Neo4jType::Float),
                _ => Some(Neo4jType::Unknown),
            }
        }
        
        // Literal: '2025-01-01', 1000, true
        Expression::StringLiteral(_) => Some(Neo4jType::String),
        Expression::IntegerLiteral(_) => Some(Neo4jType::Integer),
        Expression::FloatLiteral(_) => Some(Neo4jType::Float),
        Expression::BooleanLiteral(_) => Some(Neo4jType::Boolean),
        
        _ => Some(Neo4jType::Unknown),
    }
}
```

---

### Phase 3: Python Bindings

**3.1 Expose Type Checking Options**

File: `python/cypher_guard/__init__.py` (EXTEND)

```python
from .cypher_guard import (
    validate_cypher as _validate_cypher_rust,
    # ... existing exports
)

def validate_cypher(
    query: str,
    schema,
    type_checking: str = "off"
) -> dict:
    """
    Validate Cypher query against schema with optional type checking.
    
    Args:
        query: Cypher query string
        schema: DbSchema object
        type_checking: Type checking level ("off", "warnings", "strict")
        
    Returns:
        ValidationResult dictionary with:
        - valid: bool
        - errors: List[str]
        - warnings: List[str]
        - type_warnings: List[str] (if type_checking != "off")
        - type_errors: List[str] (if type_checking == "strict")
    """
    # Convert string to enum
    type_check_level = {
        "off": TypeCheckLevel.Off,
        "warnings": TypeCheckLevel.Warnings,
        "strict": TypeCheckLevel.Strict,
    }.get(type_checking.lower(), TypeCheckLevel.Off)
    
    # Call Rust validation with options
    return _validate_cypher_rust(query, schema, type_check_level)
```

**3.2 PyO3 Bindings**

File: `rust/src/lib.rs` (EXTEND)

```rust
use pyo3::prelude::*;

#[pyfunction]
fn validate_cypher(
    query: String,
    schema: &DbSchema,
    type_checking: TypeCheckLevel,
) -> PyResult<HashMap<String, PyObject>> {
    let options = ValidationOptions { type_checking };
    
    let result = validate_cypher_with_options(&query, schema, options)?;
    
    // Convert to Python dict
    Python::with_gil(|py| {
        let dict = PyDict::new(py);
        dict.set_item("valid", result.is_valid)?;
        dict.set_item("errors", result.errors)?;
        dict.set_item("warnings", result.warnings)?;
        dict.set_item("type_warnings", result.type_warnings)?;
        dict.set_item("type_errors", result.type_errors)?;
        Ok(dict.to_object(py))
    })
}

#[pymodule]
fn cypher_guard(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(validate_cypher, m)?)?;
    // ... existing exports
    Ok(())
}
```

---

### Phase 4: Testing

**4.1 Rust Unit Tests**

File: `rust/src/type_checker_tests.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_date_mismatch() {
        let result = check_type_compatibility(
            &Neo4jType::String,
            &Neo4jType::Date,
        );
        assert_eq!(result, Some(TypeMismatchSeverity::Error));
    }

    #[test]
    fn test_integer_float_allowed() {
        let result = check_type_compatibility(
            &Neo4jType::Integer,
            &Neo4jType::Float,
        );
        assert_eq!(result, None);  // Allowed
    }

    #[test]
    fn test_unknown_type_allowed() {
        let result = check_type_compatibility(
            &Neo4jType::Unknown,
            &Neo4jType::Date,
        );
        assert_eq!(result, None);  // Conservative: allow unknown
    }
}
```

**4.2 Integration Tests**

File: `tests/test_type_checking.py`

```python
def test_string_date_mismatch_warnings():
    query = """
    MATCH (ps:ProjectStaffing)
    WHERE ps.valid_from <= date('2025-04-08')
    RETURN ps
    """
    
    result = validate_cypher(query, schema, type_checking="warnings")
    
    assert result["valid"] == True  # Still valid in warnings mode
    assert len(result["type_warnings"]) > 0
    assert "String" in result["type_warnings"][0]
    assert "Date" in result["type_warnings"][0]

def test_string_date_mismatch_strict():
    query = """
    MATCH (ps:ProjectStaffing)
    WHERE ps.valid_from <= date('2025-04-08')
    RETURN ps
    """
    
    result = validate_cypher(query, schema, type_checking="strict")
    
    assert result["valid"] == False  # Invalid in strict mode
    assert len(result["type_errors"]) > 0

def test_type_checking_off_by_default():
    query = """
    MATCH (ps:ProjectStaffing)
    WHERE ps.valid_from <= date('2025-04-08')
    RETURN ps
    """
    
    result = validate_cypher(query, schema)  # No type_checking param
    
    assert result["valid"] == True
    assert len(result.get("type_warnings", [])) == 0  # No type checking
```

---

### Phase 5: Integration

**5.1 Update knowledge_api_tools.py**

Remove regex-based type checking, use cypher-guard's new API:

```python
def _validate_cypher_query(cypher_query: str) -> Dict[str, Any]:
    """Validate with type checking enabled."""
    
    # ... existing schema loading ...
    
    try:
        from cypher_guard import validate_cypher, DbSchema
        
        schema_dict = json.loads(guard_schema_json)
        guard_schema = DbSchema.from_dict(schema_dict)
        
        # Enable type checking!
        validation_result = validate_cypher(
            cypher_query, 
            guard_schema,
            type_checking="warnings"  # Opt-in to type checking
        )
        
        # Handle type warnings
        if validation_result.get("type_warnings"):
            for warning in validation_result["type_warnings"]:
                print(f"⚠️  Type warning: {warning}")
        
        # Type errors in strict mode
        if validation_result.get("type_errors"):
            return {
                "valid": False,
                "errors": validation_result["type_errors"],
                "suggestions": []
            }
        
        # ... rest of validation ...
```

---

## Timeline Estimate

| Phase | Task | Estimate |
|-------|------|----------|
| 1 | Core Infrastructure | 2-3 hours |
| 2 | Type Checking Logic | 4-6 hours |
| 3 | Python Bindings | 1-2 hours |
| 4 | Testing | 2-3 hours |
| 5 | Integration | 1 hour |
| **Total** | | **10-15 hours** |

---

## Success Criteria

✅ Type checking is OFF by default (backward compatible)
✅ Can enable with `type_checking="warnings"` or `"strict"`
✅ Detects String vs Date mismatches
✅ Uses blocklist approach (conservative)
✅ Unknown types are allowed
✅ All existing tests pass
✅ New type checking tests pass
✅ Integrated with knowledge_api_tools.py
