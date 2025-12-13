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
    
    #[test]
    fn test_parse_neo4j_type() {
        assert_eq!(parse_neo4j_type("STRING"), Neo4jType::String);
        assert_eq!(parse_neo4j_type("integer"), Neo4jType::Integer);
        assert_eq!(parse_neo4j_type("DATE"), Neo4jType::Date);
        assert_eq!(parse_neo4j_type("unknown_type"), Neo4jType::Unknown);
    }
}
