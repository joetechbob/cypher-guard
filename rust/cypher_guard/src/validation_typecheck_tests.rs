//! Comprehensive type checking integration tests

#[cfg(test)]
mod type_checking_tests {
    use crate::parser::ast::*;
    use crate::schema::{DbSchema, DbSchemaProperty, PropertyType};
    use crate::types::{TypeCheckLevel, TypeMismatchSeverity};
    use crate::validation::{extract_query_elements, validate_query_elements_with_options, ValidationOptions, PropertyComparison, PropertyValueType, QueryElements};

    fn create_project_staffing_schema() -> DbSchema {
        let mut schema = DbSchema::new();
        
        // ProjectStaffing node with STRING valid_from (the problematic case!)
        schema.add_label("ProjectStaffing").unwrap();
        let valid_from = DbSchemaProperty::new("valid_from", PropertyType::STRING);
        let valid_to = DbSchemaProperty::new("valid_to", PropertyType::STRING);
        schema.add_node_property("ProjectStaffing", &valid_from).unwrap();
        schema.add_node_property("ProjectStaffing", &valid_to).unwrap();
        
        // Employee node with various types
        schema.add_label("Employee").unwrap();
        let email = DbSchemaProperty::new("email", PropertyType::STRING);
        let age = DbSchemaProperty::new("age", PropertyType::INTEGER);
        let salary = DbSchemaProperty::new("salary", PropertyType::FLOAT);
        let is_active = DbSchemaProperty::new("is_active", PropertyType::BOOLEAN);
        schema.add_node_property("Employee", &email).unwrap();
        schema.add_node_property("Employee", &age).unwrap();
        schema.add_node_property("Employee", &salary).unwrap();
        schema.add_node_property("Employee", &is_active).unwrap();
        
        // Event node with temporal properties (using STRING in schema, type checking catches mismatches)
        schema.add_label("Event").unwrap();
        let event_date = DbSchemaProperty::new("event_date", PropertyType::STRING); // DATE stored as STRING
        let timestamp = DbSchemaProperty::new("timestamp", PropertyType::STRING); // DATETIME stored as STRING
        schema.add_node_property("Event", &event_date).unwrap();
        schema.add_node_property("Event", &timestamp).unwrap();
        
        schema
    }

    #[test]
    fn test_string_vs_date_detected_strict() {
        let schema = create_project_staffing_schema();
        
        // Manually create QueryElements with a String vs Date comparison
        let mut elements = QueryElements::new();
        elements.add_node_label("ProjectStaffing".to_string());
        elements.add_defined_variable("ps".to_string());
        elements.add_variable_node_binding("ps".to_string(), "ProjectStaffing".to_string());
        
        // Add comparison: ps.valid_from <= date('2025-04-08')
        elements.add_property_comparison(PropertyComparison {
            variable: "ps".to_string(),
            property: "valid_from".to_string(),
            value: "date('2025-04-08')".to_string(),
            value_type: PropertyValueType::Date,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty(), "Should have no schema validation errors, got: {:?}", errors);
        assert_eq!(type_issues.len(), 1, "Should detect String vs Date mismatch");
        assert_eq!(type_issues[0].severity, TypeMismatchSeverity::Error);
        assert!(type_issues[0].message.contains("valid_from"));
        assert!(type_issues[0].message.contains("String"));
        assert!(type_issues[0].message.contains("Date"));
    }

    #[test]
    fn test_string_vs_date_detected_warnings() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("ProjectStaffing".to_string());
        elements.add_defined_variable("ps".to_string());
        elements.add_variable_node_binding("ps".to_string(), "ProjectStaffing".to_string());
        
        elements.add_property_comparison(PropertyComparison {
            variable: "ps".to_string(),
            property: "valid_from".to_string(),
            value: "date('2025-04-08')".to_string(),
            value_type: PropertyValueType::Date,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Warnings,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert_eq!(type_issues.len(), 1, "Warnings mode should also detect mismatches");
        assert!(type_issues[0].message.contains("String"));
        assert!(type_issues[0].message.contains("Date"));
    }

    #[test]
    fn test_multiple_date_comparisons() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("ProjectStaffing".to_string());
        elements.add_defined_variable("ps".to_string());
        elements.add_variable_node_binding("ps".to_string(), "ProjectStaffing".to_string());
        
        // ps.valid_from <= date('2025-04-08')
        elements.add_property_comparison(PropertyComparison {
            variable: "ps".to_string(),
            property: "valid_from".to_string(),
            value: "date('2025-04-08')".to_string(),
            value_type: PropertyValueType::Date,
        });
        
        // ps.valid_to >= date('2025-04-04')
        elements.add_property_comparison(PropertyComparison {
            variable: "ps".to_string(),
            property: "valid_to".to_string(),
            value: "date('2025-04-04')".to_string(),
            value_type: PropertyValueType::Date,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert_eq!(type_issues.len(), 2, "Should detect BOTH String vs Date comparisons");
    }

    #[test]
    fn test_type_checking_off_by_default() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("ProjectStaffing".to_string());
        elements.add_defined_variable("ps".to_string());
        elements.add_variable_node_binding("ps".to_string(), "ProjectStaffing".to_string());
        
        elements.add_property_comparison(PropertyComparison {
            variable: "ps".to_string(),
            property: "valid_from".to_string(),
            value: "date('2025-04-08')".to_string(),
            value_type: PropertyValueType::Date,
        });
        
        // Default options (OFF)
        let options = ValidationOptions::default();
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert!(type_issues.is_empty(), "Type checking is OFF, should not report issues");
    }

    #[test]
    fn test_string_vs_datetime() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("ProjectStaffing".to_string());
        elements.add_defined_variable("ps".to_string());
        elements.add_variable_node_binding("ps".to_string(), "ProjectStaffing".to_string());
        
        elements.add_property_comparison(PropertyComparison {
            variable: "ps".to_string(),
            property: "valid_from".to_string(),
            value: "datetime('2025-04-08T10:00:00')".to_string(),
            value_type: PropertyValueType::DateTime,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert_eq!(type_issues.len(), 1, "Should detect String vs DateTime mismatch");
        assert!(type_issues[0].message.contains("DateTime"));
    }

    #[test]
    fn test_string_vs_string_allowed() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("Employee".to_string());
        elements.add_defined_variable("e".to_string());
        elements.add_variable_node_binding("e".to_string(), "Employee".to_string());
        
        elements.add_property_comparison(PropertyComparison {
            variable: "e".to_string(),
            property: "email".to_string(),
            value: "test@example.com".to_string(),
            value_type: PropertyValueType::String,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert!(type_issues.is_empty(), "String vs String should be allowed");
    }

    #[test]
    fn test_integer_vs_integer_allowed() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("Employee".to_string());
        elements.add_defined_variable("e".to_string());
        elements.add_variable_node_binding("e".to_string(), "Employee".to_string());
        
        elements.add_property_comparison(PropertyComparison {
            variable: "e".to_string(),
            property: "age".to_string(),
            value: "25".to_string(),
            value_type: PropertyValueType::Number,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert!(type_issues.is_empty(), "Integer vs Integer should be allowed");
    }

    #[test]
    fn test_suggestion_provided_for_string_date() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("ProjectStaffing".to_string());
        elements.add_defined_variable("ps".to_string());
        elements.add_variable_node_binding("ps".to_string(), "ProjectStaffing".to_string());
        
        elements.add_property_comparison(PropertyComparison {
            variable: "ps".to_string(),
            property: "valid_from".to_string(),
            value: "date('2025-04-08')".to_string(),
            value_type: PropertyValueType::Date,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Warnings,
        };
        let (_errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert_eq!(type_issues.len(), 1);
        assert!(type_issues[0].suggestion.is_some(), "Should provide a suggestion");
        let suggestion = type_issues[0].suggestion.as_ref().unwrap();
        assert!(suggestion.contains("date("), "Suggestion should mention date() conversion: {}", suggestion);
    }

    #[test]
    fn test_backward_compatibility_wrapper() {
        use crate::validation::validate_query_elements;
        
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("ProjectStaffing".to_string());
        elements.add_defined_variable("ps".to_string());
        elements.add_variable_node_binding("ps".to_string(), "ProjectStaffing".to_string());
        
        elements.add_property_comparison(PropertyComparison {
            variable: "ps".to_string(),
            property: "valid_from".to_string(),
            value: "date('2025-04-08')".to_string(),
            value_type: PropertyValueType::Date,
        });
        
        // Old API (no type checking)
        let errors = validate_query_elements(&elements, &schema);
        
        assert!(errors.is_empty(), "Backward compatible API should work without type checking");
    }

    // ========== INTEGER TYPE MISMATCH TESTS ==========
    
    #[test]
    fn test_integer_vs_string_mismatch() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("Employee".to_string());
        elements.add_defined_variable("e".to_string());
        elements.add_variable_node_binding("e".to_string(), "Employee".to_string());
        
        // age (INTEGER) compared with string
        elements.add_property_comparison(PropertyComparison {
            variable: "e".to_string(),
            property: "age".to_string(),
            value: "not_a_number".to_string(),
            value_type: PropertyValueType::String,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert_eq!(type_issues.len(), 1, "Should detect Integer vs String mismatch");
        assert!(type_issues[0].message.contains("Integer"));
        assert!(type_issues[0].message.contains("String"));
    }

    #[test]
    fn test_integer_vs_boolean_mismatch() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("Employee".to_string());
        elements.add_defined_variable("e".to_string());
        elements.add_variable_node_binding("e".to_string(), "Employee".to_string());
        
        // age (INTEGER) compared with boolean
        elements.add_property_comparison(PropertyComparison {
            variable: "e".to_string(),
            property: "age".to_string(),
            value: "true".to_string(),
            value_type: PropertyValueType::Boolean,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert_eq!(type_issues.len(), 1, "Should detect Integer vs Boolean mismatch");
        assert!(type_issues[0].message.contains("Integer"));
        assert!(type_issues[0].message.contains("Boolean"));
    }

    #[test]
    fn test_integer_vs_date_mismatch() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("Employee".to_string());
        elements.add_defined_variable("e".to_string());
        elements.add_variable_node_binding("e".to_string(), "Employee".to_string());
        
        // age (INTEGER) compared with date
        elements.add_property_comparison(PropertyComparison {
            variable: "e".to_string(),
            property: "age".to_string(),
            value: "date('2025-04-08')".to_string(),
            value_type: PropertyValueType::Date,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert_eq!(type_issues.len(), 1, "Should detect Integer vs Date mismatch");
        assert!(type_issues[0].message.contains("Integer"));
        assert!(type_issues[0].message.contains("Date"));
    }

    #[test]
    fn test_integer_vs_datetime_mismatch() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("Employee".to_string());
        elements.add_defined_variable("e".to_string());
        elements.add_variable_node_binding("e".to_string(), "Employee".to_string());
        
        // age (INTEGER) compared with datetime
        elements.add_property_comparison(PropertyComparison {
            variable: "e".to_string(),
            property: "age".to_string(),
            value: "datetime('2025-04-08T10:00:00')".to_string(),
            value_type: PropertyValueType::DateTime,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert_eq!(type_issues.len(), 1, "Should detect Integer vs DateTime mismatch");
        assert!(type_issues[0].message.contains("Integer"));
        assert!(type_issues[0].message.contains("DateTime"));
    }

    // ========== BOOLEAN TYPE MISMATCH TESTS ==========
    
    #[test]
    fn test_boolean_vs_string_mismatch() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("Employee".to_string());
        elements.add_defined_variable("e".to_string());
        elements.add_variable_node_binding("e".to_string(), "Employee".to_string());
        
        // is_active (BOOLEAN) compared with string
        elements.add_property_comparison(PropertyComparison {
            variable: "e".to_string(),
            property: "is_active".to_string(),
            value: "yes".to_string(),
            value_type: PropertyValueType::String,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert_eq!(type_issues.len(), 1, "Should detect Boolean vs String mismatch");
        assert!(type_issues[0].message.contains("Boolean"));
        assert!(type_issues[0].message.contains("String"));
    }

    #[test]
    fn test_boolean_vs_integer_mismatch() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("Employee".to_string());
        elements.add_defined_variable("e".to_string());
        elements.add_variable_node_binding("e".to_string(), "Employee".to_string());
        
        // is_active (BOOLEAN) compared with integer
        elements.add_property_comparison(PropertyComparison {
            variable: "e".to_string(),
            property: "is_active".to_string(),
            value: "1".to_string(),
            value_type: PropertyValueType::Number,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert_eq!(type_issues.len(), 1, "Should detect Boolean vs Integer mismatch");
        assert!(type_issues[0].message.contains("Boolean"));
        assert!(type_issues[0].message.contains("Integer"));
    }

    #[test]
    fn test_boolean_vs_boolean_allowed() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("Employee".to_string());
        elements.add_defined_variable("e".to_string());
        elements.add_variable_node_binding("e".to_string(), "Employee".to_string());
        
        // is_active (BOOLEAN) compared with boolean
        elements.add_property_comparison(PropertyComparison {
            variable: "e".to_string(),
            property: "is_active".to_string(),
            value: "true".to_string(),
            value_type: PropertyValueType::Boolean,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert!(type_issues.is_empty(), "Boolean vs Boolean should be allowed");
    }

    // ========== FLOAT TYPE MISMATCH TESTS ==========
    
    #[test]
    fn test_float_vs_string_mismatch() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("Employee".to_string());
        elements.add_defined_variable("e".to_string());
        elements.add_variable_node_binding("e".to_string(), "Employee".to_string());
        
        // salary (FLOAT) compared with string
        elements.add_property_comparison(PropertyComparison {
            variable: "e".to_string(),
            property: "salary".to_string(),
            value: "high".to_string(),
            value_type: PropertyValueType::String,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert_eq!(type_issues.len(), 1, "Should detect Float vs String mismatch");
        assert!(type_issues[0].message.contains("Float"));
        assert!(type_issues[0].message.contains("String"));
    }

    #[test]
    fn test_float_vs_integer_allowed() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("Employee".to_string());
        elements.add_defined_variable("e".to_string());
        elements.add_variable_node_binding("e".to_string(), "Employee".to_string());
        
        // salary (FLOAT) compared with integer (should be compatible)
        elements.add_property_comparison(PropertyComparison {
            variable: "e".to_string(),
            property: "salary".to_string(),
            value: "50000".to_string(),
            value_type: PropertyValueType::Number,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert!(type_issues.is_empty(), "Float vs Integer should be allowed (numeric compatibility)");
    }

    #[test]
    fn test_float_vs_boolean_mismatch() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("Employee".to_string());
        elements.add_defined_variable("e".to_string());
        elements.add_variable_node_binding("e".to_string(), "Employee".to_string());
        
        // salary (FLOAT) compared with boolean
        elements.add_property_comparison(PropertyComparison {
            variable: "e".to_string(),
            property: "salary".to_string(),
            value: "false".to_string(),
            value_type: PropertyValueType::Boolean,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert_eq!(type_issues.len(), 1, "Should detect Float vs Boolean mismatch");
        assert!(type_issues[0].message.contains("Float"));
        assert!(type_issues[0].message.contains("Boolean"));
    }

    // ========== DATE/DATETIME vs STRING MISMATCH TESTS ==========
    // NOTE: In real schemas, dates are stored as STRING, so these tests validate
    // detection of STRING vs Date/DateTime mismatches (the original problem!)
    
    #[test]
    fn test_string_field_vs_date_function_mismatch() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("Event".to_string());
        elements.add_defined_variable("ev".to_string());
        elements.add_variable_node_binding("ev".to_string(), "Event".to_string());
        
        // event_date (STRING in schema) compared with date() function
        elements.add_property_comparison(PropertyComparison {
            variable: "ev".to_string(),
            property: "event_date".to_string(),
            value: "date('2025-04-08')".to_string(),
            value_type: PropertyValueType::Date,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert_eq!(type_issues.len(), 1, "Should detect String vs Date mismatch");
        assert!(type_issues[0].message.contains("String"));
        assert!(type_issues[0].message.contains("Date"));
    }

    #[test]
    fn test_string_field_vs_datetime_function_mismatch() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("Event".to_string());
        elements.add_defined_variable("ev".to_string());
        elements.add_variable_node_binding("ev".to_string(), "Event".to_string());
        
        // timestamp (STRING in schema) compared with datetime() function
        elements.add_property_comparison(PropertyComparison {
            variable: "ev".to_string(),
            property: "timestamp".to_string(),
            value: "datetime('2025-04-08T10:00:00')".to_string(),
            value_type: PropertyValueType::DateTime,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert_eq!(type_issues.len(), 1, "Should detect String vs DateTime mismatch");
        assert!(type_issues[0].message.contains("String"));
        assert!(type_issues[0].message.contains("DateTime"));
    }

    #[test]
    fn test_string_field_vs_string_literal_allowed() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("Event".to_string());
        elements.add_defined_variable("ev".to_string());
        elements.add_variable_node_binding("ev".to_string(), "Event".to_string());
        
        // event_date (STRING) compared with string literal - this is allowed
        elements.add_property_comparison(PropertyComparison {
            variable: "ev".to_string(),
            property: "event_date".to_string(),
            value: "2025-04-08".to_string(),
            value_type: PropertyValueType::String,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert!(type_issues.is_empty(), "String vs String should be allowed");
    }

    // ========== EDGE CASES AND SPECIAL SCENARIOS ==========
    
    #[test]
    fn test_null_always_allowed() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("Employee".to_string());
        elements.add_defined_variable("e".to_string());
        elements.add_variable_node_binding("e".to_string(), "Employee".to_string());
        
        // any property compared with null should be allowed
        elements.add_property_comparison(PropertyComparison {
            variable: "e".to_string(),
            property: "age".to_string(),
            value: "null".to_string(),
            value_type: PropertyValueType::Null,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert!(type_issues.is_empty(), "Null should always be allowed");
    }

    #[test]
    fn test_unknown_type_skipped() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("Employee".to_string());
        elements.add_defined_variable("e".to_string());
        elements.add_variable_node_binding("e".to_string(), "Employee".to_string());
        
        // Unknown value type (e.g., variable) should be skipped
        elements.add_property_comparison(PropertyComparison {
            variable: "e".to_string(),
            property: "age".to_string(),
            value: "someVariable".to_string(),
            value_type: PropertyValueType::Unknown,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert!(type_issues.is_empty(), "Unknown type should be skipped");
    }

    #[test]
    fn test_multiple_mismatches_all_detected() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("Employee".to_string());
        elements.add_defined_variable("e".to_string());
        elements.add_variable_node_binding("e".to_string(), "Employee".to_string());
        
        // Multiple mismatches
        elements.add_property_comparison(PropertyComparison {
            variable: "e".to_string(),
            property: "age".to_string(),
            value: "not_a_number".to_string(),
            value_type: PropertyValueType::String,
        });
        
        elements.add_property_comparison(PropertyComparison {
            variable: "e".to_string(),
            property: "is_active".to_string(),
            value: "1".to_string(),
            value_type: PropertyValueType::Number,
        });
        
        elements.add_property_comparison(PropertyComparison {
            variable: "e".to_string(),
            property: "salary".to_string(),
            value: "true".to_string(),
            value_type: PropertyValueType::Boolean,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert!(errors.is_empty());
        assert_eq!(type_issues.len(), 3, "Should detect all 3 type mismatches");
    }

    #[test]
    fn test_severity_strict_is_error() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("ProjectStaffing".to_string());
        elements.add_defined_variable("ps".to_string());
        elements.add_variable_node_binding("ps".to_string(), "ProjectStaffing".to_string());
        
        elements.add_property_comparison(PropertyComparison {
            variable: "ps".to_string(),
            property: "valid_from".to_string(),
            value: "date('2025-04-08')".to_string(),
            value_type: PropertyValueType::Date,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (_errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert_eq!(type_issues.len(), 1);
        assert_eq!(type_issues[0].severity, TypeMismatchSeverity::Error, "Strict mode should produce Error severity");
    }

    #[test]
    fn test_severity_warnings_is_warning() {
        let schema = create_project_staffing_schema();
        
        let mut elements = QueryElements::new();
        elements.add_node_label("ProjectStaffing".to_string());
        elements.add_defined_variable("ps".to_string());
        elements.add_variable_node_binding("ps".to_string(), "ProjectStaffing".to_string());
        
        elements.add_property_comparison(PropertyComparison {
            variable: "ps".to_string(),
            property: "valid_from".to_string(),
            value: "date('2025-04-08')".to_string(),
            value_type: PropertyValueType::Date,
        });
        
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Warnings,
        };
        let (_errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);
        
        assert_eq!(type_issues.len(), 1);
        assert_eq!(type_issues[0].severity, TypeMismatchSeverity::Warning, "Warnings mode should produce Warning severity");
    }
}
