# Cypher-Guard Type Checking Implementation Roadmap

## Status: Ready for Implementation

This document provides step-by-step instructions for implementing type checking in cypher-guard.

---

## Current State Analysis

### ✅ Existing Infrastructure (Great News!)

The codebase already has infrastructure we can build on:

**`validation.rs` - QueryElements:**
```rust
pub struct QueryElements {
    pub variable_node_bindings: HashMap<String, String>,      // ✅ Already exists!
    pub variable_relationship_bindings: HashMap<String, String>, // ✅ Already exists!
    pub property_comparisons: Vec<PropertyComparison>,         // ✅ Already exists!
    // ... other fields
}

pub struct PropertyComparison {
    pub variable: String,
    pub property: String,
    pub value: String,
    pub value_type: PropertyValueType,  // ✅ Already has type info!
}
```

**What This Means:**
- Variable→label tracking: ✅ DONE
- Property comparison tracking: ✅ DONE
- We just need to add:
  1. Type checking levels (off/warnings/strict)
  2. Enhanced type compatibility logic
  3. Python bindings for new API

---

## Implementation Steps

### Step 1: Add Type System (30 minutes)

**File: `rust/cypher_guard/src/types.rs` (NEW)**

Create this new file:

```rust
//! Type system for cypher-guard type checking

use std::fmt;

/// Type checking severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeCheckLevel {
    Off,
    Warnings,
    Strict,
}

impl Default for TypeCheckLevel {
    fn default() -> Self {
        Self::Off  // Backward compatible
    }
}

impl fmt::Display for TypeCheckLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeCheckLevel::Off => write!(f, "off"),
            TypeCheckLevel::Warnings => write!(f, "warnings"),
            TypeCheckLevel::Strict => write!(f, "strict"),
        }
    }
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
    Unknown,  // For types we don't recognize (conservative)
}

impl fmt::Display for Neo4jType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Neo4jType::String => write!(f, "String"),
            Neo4jType::Integer => write!(f, "Integer"),
            Neo4jType::Float => write!(f, "Float"),
            Neo4jType::Boolean => write!(f, "Boolean"),
            Neo4jType::Date => write!(f, "Date"),
            Neo4jType::DateTime => write!(f, "DateTime"),
            Neo4jType::LocalTime => write!(f, "LocalTime"),
            Neo4jType::Time => write!(f, "Time"),
            Neo4jType::Duration => write!(f, "Duration"),
            Neo4jType::Point => write!(f, "Point"),
            Neo4jType::Unknown => write!(f, "Unknown"),
        }
    }
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

/// Parse Neo4j type string from schema
pub fn parse_neo4j_type(type_str: &str) -> Neo4jType {
    match type_str.to_uppercase().trim() {
        "STRING" => Neo4jType::String,
        "INTEGER" | "INT" | "LONG" => Neo4jType::Integer,
        "FLOAT" | "DOUBLE" => Neo4jType::Float,
        "BOOLEAN" | "BOOL" => Neo4jType::Boolean,
        "DATE" => Neo4jType::Date,
        "DATETIME" | "ZONEDDATETIME" => Neo4jType::DateTime,
        "LOCALTIME" => Neo4jType::LocalTime,
        "TIME" => Neo4jType::Time,
        "DURATION" => Neo4jType::Duration,
        "POINT" => Neo4jType::Point,
        _ => Neo4jType::Unknown,  // Conservative: allow unknown types
    }
}

/// Check if two types have a known problematic comparison (BLOCKLIST approach)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_date_mismatch() {
        let result = check_type_compatibility(&Neo4jType::String, &Neo4jType::Date);
        assert_eq!(result, Some(TypeMismatchSeverity::Error));
    }

    #[test]
    fn test_integer_float_allowed() {
        let result = check_type_compatibility(&Neo4jType::Integer, &Neo4jType::Float);
        assert_eq!(result, None);  // Allowed
    }

    #[test]
    fn test_unknown_type_allowed() {
        let result = check_type_compatibility(&Neo4jType::Unknown, &Neo4jType::Date);
        assert_eq!(result, None);  // Conservative: allow unknown
    }
}
```

**Add to `rust/cypher_guard/src/lib.rs`:**
```rust
pub mod types;  // Add this line
```

---

### Step 2: Extend validation.rs (1-2 hours)

**File: `rust/cypher_guard/src/validation.rs` (MODIFY)**

Add at the top:
```rust
use crate::types::{TypeCheckLevel, Neo4jType, parse_neo4j_type, check_type_compatibility, TypeIssue, TypeMismatchSeverity};
```

Add ValidationOptions struct after QueryElements:
```rust
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
```

Modify `validate_query_elements` signature:
```rust
pub fn validate_query_elements(
    elements: &QueryElements,
    schema: &DbSchema,
    options: &ValidationOptions,  // NEW parameter
) -> (Vec<CypherGuardValidationError>, Vec<TypeIssue>) {  // Return tuple
    // ... existing validation code ...
    
    let mut errors = Vec::new();
    let mut type_issues = Vec::new();  // NEW
    
    // ... existing validation ...
    
    // NEW: Type checking logic (only if enabled)
    if options.type_checking != TypeCheckLevel::Off {
        for comparison in &elements.property_comparisons {
            if let Some(issue) = check_property_comparison_types(comparison, elements, schema) {
                type_issues.push(issue);
            }
        }
    }
    
    (errors, type_issues)
}
```

Add new function before the tests:
```rust
/// Check a property comparison for type mismatches
fn check_property_comparison_types(
    comparison: &PropertyComparison,
    elements: &QueryElements,
    schema: &DbSchema,
) -> Option<TypeIssue> {
    // Get the node label for this variable
    let label = elements.variable_node_bindings.get(&comparison.variable)?;
    
    // Get the property type from schema
    let properties = schema.node_props.get(label)?;
    let prop_def = properties.iter().find(|p| p.name == comparison.property)?;
    
    // Parse the property type
    let prop_type = parse_neo4jtype(&prop_def.neo4j_type.to_string());
    
    // Infer the comparison value type
    let value_type = match comparison.value_type {
        PropertyValueType::String => Neo4jType::String,
        PropertyValueType::Number => Neo4jType::Integer,  // Simplified
        PropertyValueType::Boolean => Neo4jType::Boolean,
        PropertyValueType::Null | PropertyValueType::Unknown => return None,  // Skip
    };
    
    // Check compatibility (blocklist approach)
    if let Some(severity) = check_type_compatibility(&prop_type, &value_type) {
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
        
        return Some(TypeIssue {
            severity,
            message,
            suggestion,
        });
    }
    
    None
}
```

---

### Step 3: Update Python Bindings (1 hour)

**File: `rust/python_bindings/src/lib.rs` (MODIFY)**

Update the validate_cypher function to accept type_checking parameter:

```rust
#[pyfunction]
#[pyo3(signature = (cypher_query, schema, type_checking="off"))]
fn validate_cypher(
    cypher_query: String,
    schema: &PyDbSchema,
    type_checking: &str,
) -> PyResult<HashMap<String, PyObject>> {
    // Parse type checking level
    let type_check_level = match type_checking.to_lowercase().as_str() {
        "off" => TypeCheckLevel::Off,
        "warnings" => TypeCheckLevel::Warnings,
        "strict" => TypeCheckLevel::Strict,
        _ => TypeCheckLevel::Off,
    };
    
    let options = ValidationOptions {
        type_checking: type_check_level,
    };
    
    // ... existing parsing code ...
    
    // Call validation with options
    let (errors, type_issues) = validate_query_elements(&elements, &schema.0, &options);
    
    // Build result dictionary
    Python::with_gil(|py| {
        let dict = PyDict::new(py);
        
        // Separate type issues by severity
        let mut type_warnings = Vec::new();
        let mut type_errors = Vec::new();
        
        for issue in type_issues {
            let msg = if let Some(suggestion) = &issue.suggestion {
                format!("{}\nSuggestion: {}", issue.message, suggestion)
            } else {
                issue.message.clone()
            };
            
            match issue.severity {
                TypeMismatchSeverity::Warning => type_warnings.push(msg),
                TypeMismatchSeverity::Error => type_errors.push(msg),
            }
        }
        
        // Set validity based on mode
        let is_valid = if type_check_level == TypeCheckLevel::Strict {
            errors.is_empty() && type_errors.is_empty()
        } else {
            errors.is_empty()
        };
        
        dict.set_item("valid", is_valid)?;
        dict.set_item("errors", error_messages)?;
        dict.set_item("type_warnings", type_warnings)?;
        dict.set_item("type_errors", type_errors)?;
        
        Ok(dict.to_object(py))
    })
}
```

---

### Step 4: Testing (2-3 hours)

**File: `rust/python_bindings/tests/unit/test_type_checking.py` (NEW)**

```python
import pytest
from cypher_guard import validate_cypher, DbSchema

def test_type_checking_off_by_default():
    """Type checking should be OFF by default (backward compatible)"""
    schema_json = '''
    {
        "node_props": {
            "ProjectStaffing": [
                {"name": "valid_from", "neo4j_type": "STRING"}
            ]
        },
        "rel_props": {},
        "relationships": [],
        "metadata": {}
    }
    '''
    
    schema = DbSchema.from_json_string(schema_json)
    
    query = """
    MATCH (ps:ProjectStaffing)
    WHERE ps.valid_from <= date('2025-04-08')
    RETURN ps
    """
    
    result = validate_cypher(query, schema)  # No type_checking parameter
    
    assert result["valid"] == True
    assert len(result.get("type_warnings", [])) == 0
    assert len(result.get("type_errors", [])) == 0


def test_type_checking_warnings_mode():
    """Type checking in warnings mode should report issues but not block"""
    schema_json = '''
    {
        "node_props": {
            "ProjectStaffing": [
                {"name": "valid_from", "neo4j_type": "STRING"}
            ]
        },
        "rel_props": {},
        "relationships": [],
        "metadata": {}
    }
    '''
    
    schema = DbSchema.from_json_string(schema_json)
    
    query = """
    MATCH (ps:ProjectStaffing)
    WHERE ps.valid_from <= date('2025-04-08')
    RETURN ps
    """
    
    result = validate_cypher(query, schema, type_checking="warnings")
    
    assert result["valid"] == True  # Still valid
    assert len(result["type_warnings"]) > 0  # But has warnings
    assert "String" in result["type_warnings"][0]
    assert "Date" in result["type_warnings"][0]


def test_type_checking_strict_mode():
    """Type checking in strict mode should block invalid queries"""
    schema_json = '''
    {
        "node_props": {
            "ProjectStaffing": [
                {"name": "valid_from", "neo4j_type": "STRING"}
            ]
        },
        "rel_props": {},
        "relationships": [],
        "metadata": {}
    }
    '''
    
    schema = DbSchema.from_json_string(schema_json)
    
    query = """
    MATCH (ps:ProjectStaffing)
    WHERE ps.valid_from <= date('2025-04-08')
    RETURN ps
    """
    
    result = validate_cypher(query, schema, type_checking="strict")
    
    assert result["valid"] == False  # Invalid in strict mode
    assert len(result["type_errors"]) > 0


def test_integer_float_allowed():
    """Integer and Float comparisons should be allowed"""
    schema_json = '''
    {
        "node_props": {
            "Product": [
                {"name": "price", "neo4j_type": "INTEGER"}
            ]
        },
        "rel_props": {},
        "relationships": [],
        "metadata": {}
    }
    '''
    
    schema = DbSchema.from_json_string(schema_json)
    
    query = """
    MATCH (p:Product)
    WHERE p.price > 25.5
    RETURN p
    """
    
    result = validate_cypher(query, schema, type_checking="strict")
    
    # Integer vs Float should be allowed (no type error)
    assert len(result.get("type_errors", [])) == 0
```

---

### Step 5: Rebuild and Test (30 minutes)

```bash
cd packages/cypher-guard

# Rebuild Rust with Python bindings
maturin develop --release

# Run tests
pytest rust/python_bindings/tests/unit/test_type_checking.py -v

# Run all tests to ensure no regressions
pytest rust/python_bindings/tests/ -v
```

---

### Step 6: Update knowledge_api_tools.py (15 minutes)

**File: `knowledge_api_tools.py` (MODIFY)**

Remove the `_detect_type_mismatches()` function and update `_validate_cypher_query()`:

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
            logger.warning("Type warnings detected:")
            for warning in validation_result["type_warnings"]:
                logger.warning(f"  ⚠️  {warning}")
        
        # Type errors block in strict mode (but we use warnings mode)
        if validation_result.get("type_errors"):
            return {
                "valid": False,
                "errors": validation_result["type_errors"],
                "suggestions": []
            }
        
        # ... rest of validation ...
```

---

## Timeline

| Step | Task | Estimate |
|------|------|----------|
| 1 | Add Type System | 30 min |
| 2 | Extend validation.rs | 1-2 hours |
| 3 | Update Python Bindings | 1 hour |
| 4 | Testing | 2-3 hours |
| 5 | Rebuild and Test | 30 min |
| 6 | Update knowledge_api_tools | 15 min |
| **Total** | | **5-7 hours** |

Note: Original estimate was 10-15 hours, but leveraging existing infrastructure reduces this significantly!

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

---

## Commit Message Template

```
feat: Add opt-in type checking to cypher-guard

- Add TypeCheckLevel enum (off, warnings, strict)
- Implement blocklist-based type compatibility checking
- Detect String vs Date/DateTime/Boolean mismatches
- Detect String vs Integer/Float mismatches (warnings)
- Allow Integer↔Float, Date↔DateTime, and unknown types
- Default: type_checking="off" (backward compatible)
- Python API: validate_cypher(query, schema, type_checking="warnings")
- Add comprehensive test suite

Fixes issue where String properties compared to date() functions
cause silent failures in Neo4j (returns 0 results instead of error).

Conservative blocklist approach: only flags known problematic patterns,
allows all other type combinations including unknowns.
```
