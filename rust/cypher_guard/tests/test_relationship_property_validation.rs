#[cfg(test)]
mod relationship_property_validation_tests {
    use cypher_guard::{DbSchema, DbSchemaProperty, PropertyType};
    use cypher_guard::types::TypeCheckLevel;
    use cypher_guard::validation::{
        validate_query_elements_with_options, PropertyComparison,
        PropertyValueType, QueryElements, ValidationOptions,
    };

    /// Tests for relationship property type checking
    ///
    /// These tests verify that the type checker correctly validates properties
    /// on relationships, not just nodes.

    #[test]
    fn test_relationship_property_type_mismatch() {
        // Create schema with relationship property
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();
        schema.add_label("Company").unwrap();

        let salary_prop = DbSchemaProperty::new("salary", PropertyType::FLOAT);
        schema
            .add_relationship_property("WORKS_FOR", &salary_prop)
            .unwrap();

        // Manually create QueryElements with relationship property comparison
        let mut elements = QueryElements::new();
        elements.add_node_label("Person".to_string());
        elements.add_node_label("Company".to_string());
        elements.add_relationship_type("WORKS_FOR".to_string());
        elements.add_defined_variable("p".to_string());
        elements.add_defined_variable("c".to_string());
        elements.add_defined_variable("r".to_string());
        elements.add_variable_node_binding("p".to_string(), "Person".to_string());
        elements.add_variable_node_binding("c".to_string(), "Company".to_string());
        elements.add_variable_relationship_binding("r".to_string(), "WORKS_FOR".to_string());

        // Add comparison: r.salary = 'high' (String) but salary is FLOAT
        elements.add_property_comparison(PropertyComparison {
            variable: "r".to_string(),
            property: "salary".to_string(),
            value: "'high'".to_string(),
            value_type: PropertyValueType::String,
        });

        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);

        // Should detect type mismatch: salary is FLOAT but compared with String
        assert!(
            errors.is_empty(),
            "Should have no schema validation errors"
        );
        assert_eq!(
            type_issues.len(),
            1,
            "Expected 1 type issue for relationship property type mismatch, got {}",
            type_issues.len()
        );
        assert!(type_issues[0].message.contains("salary"));
    }

    #[test]
    fn test_relationship_property_correct_type() {
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();
        schema.add_label("Company").unwrap();

        let since_prop = DbSchemaProperty::new("since", PropertyType::INTEGER);
        schema
            .add_relationship_property("WORKS_FOR", &since_prop)
            .unwrap();

        let mut elements = QueryElements::new();
        elements.add_node_label("Person".to_string());
        elements.add_node_label("Company".to_string());
        elements.add_relationship_type("WORKS_FOR".to_string());
        elements.add_defined_variable("p".to_string());
        elements.add_defined_variable("c".to_string());
        elements.add_defined_variable("r".to_string());
        elements.add_variable_node_binding("p".to_string(), "Person".to_string());
        elements.add_variable_node_binding("c".to_string(), "Company".to_string());
        elements.add_variable_relationship_binding("r".to_string(), "WORKS_FOR".to_string());

        // Add comparison: r.since > 2020 (INTEGER vs INTEGER - correct)
        elements.add_property_comparison(PropertyComparison {
            variable: "r".to_string(),
            property: "since".to_string(),
            value: "2020".to_string(),
            value_type: PropertyValueType::Number,
        });

        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);

        // Should not detect any issues: since is INTEGER, compared with INTEGER
        assert!(errors.is_empty());
        assert_eq!(
            type_issues.len(),
            0,
            "Expected no type issues for correct relationship property type"
        );
    }

    #[test]
    fn test_relationship_property_string_comparison() {
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();

        let rel_type_prop = DbSchemaProperty::new("relationship_type", PropertyType::STRING);
        schema
            .add_relationship_property("KNOWS", &rel_type_prop)
            .unwrap();

        let mut elements = QueryElements::new();
        elements.add_node_label("Person".to_string());
        elements.add_relationship_type("KNOWS".to_string());
        elements.add_defined_variable("p1".to_string());
        elements.add_defined_variable("p2".to_string());
        elements.add_defined_variable("r".to_string());
        elements.add_variable_node_binding("p1".to_string(), "Person".to_string());
        elements.add_variable_node_binding("p2".to_string(), "Person".to_string());
        elements.add_variable_relationship_binding("r".to_string(), "KNOWS".to_string());

        // Add comparison: r.relationship_type = 'friend' (STRING vs STRING - correct)
        elements.add_property_comparison(PropertyComparison {
            variable: "r".to_string(),
            property: "relationship_type".to_string(),
            value: "'friend'".to_string(),
            value_type: PropertyValueType::String,
        });

        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);

        // Should not detect any issues: relationship_type is STRING, compared with STRING
        assert!(errors.is_empty());
        assert_eq!(
            type_issues.len(),
            0,
            "Expected no type issues for STRING relationship property"
        );
    }

    #[test]
    fn test_relationship_property_boolean_mismatch() {
        let mut schema = DbSchema::new();
        schema.add_label("User").unwrap();
        schema.add_label("Resource").unwrap();

        let is_active_prop = DbSchemaProperty::new("is_active", PropertyType::BOOLEAN);
        schema
            .add_relationship_property("HAS_ACCESS", &is_active_prop)
            .unwrap();

        let mut elements = QueryElements::new();
        elements.add_node_label("User".to_string());
        elements.add_node_label("Resource".to_string());
        elements.add_relationship_type("HAS_ACCESS".to_string());
        elements.add_defined_variable("u".to_string());
        elements.add_defined_variable("res".to_string());
        elements.add_defined_variable("r".to_string());
        elements.add_variable_node_binding("u".to_string(), "User".to_string());
        elements.add_variable_node_binding("res".to_string(), "Resource".to_string());
        elements.add_variable_relationship_binding("r".to_string(), "HAS_ACCESS".to_string());

        // Add comparison: r.is_active = 1 (INTEGER) but is_active is BOOLEAN
        elements.add_property_comparison(PropertyComparison {
            variable: "r".to_string(),
            property: "is_active".to_string(),
            value: "1".to_string(),
            value_type: PropertyValueType::Number,
        });

        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);

        // Should detect type mismatch: is_active is BOOLEAN but compared with INTEGER
        assert!(errors.is_empty());
        assert_eq!(
            type_issues.len(),
            1,
            "Expected 1 type issue for BOOLEAN vs INTEGER"
        );
    }

    #[test]
    fn test_mixed_node_and_relationship_properties() {
        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();
        schema.add_label("Company").unwrap();

        let age_prop = DbSchemaProperty::new("age", PropertyType::INTEGER);
        schema.add_node_property("Person", &age_prop).unwrap();

        let salary_prop = DbSchemaProperty::new("salary", PropertyType::FLOAT);
        schema
            .add_relationship_property("WORKS_FOR", &salary_prop)
            .unwrap();

        let mut elements = QueryElements::new();
        elements.add_node_label("Person".to_string());
        elements.add_node_label("Company".to_string());
        elements.add_relationship_type("WORKS_FOR".to_string());
        elements.add_defined_variable("p".to_string());
        elements.add_defined_variable("c".to_string());
        elements.add_defined_variable("r".to_string());
        elements.add_variable_node_binding("p".to_string(), "Person".to_string());
        elements.add_variable_node_binding("c".to_string(), "Company".to_string());
        elements.add_variable_relationship_binding("r".to_string(), "WORKS_FOR".to_string());

        // Add comparisons: p.age = 30 (INTEGER) AND r.salary > 50000.0 (FLOAT)
        elements.add_property_comparison(PropertyComparison {
            variable: "p".to_string(),
            property: "age".to_string(),
            value: "30".to_string(),
            value_type: PropertyValueType::Number,
        });
        elements.add_property_comparison(PropertyComparison {
            variable: "r".to_string(),
            property: "salary".to_string(),
            value: "50000.0".to_string(),
            value_type: PropertyValueType::Number,
        });

        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);

        // Should not detect any issues: both properties have correct types
        assert!(errors.is_empty());
        assert_eq!(
            type_issues.len(),
            0,
            "Expected no type issues for correct mixed properties"
        );
    }

    #[test]
    fn test_relationship_property_type_mismatch_from_parsed_query() {
        // This is an INTEGRATION test that parses a real Cypher query
        // Unlike the other tests, this doesn't manually create QueryElements
        use cypher_guard::parse_query;
        use cypher_guard::validation::extract_query_elements;

        let mut schema = DbSchema::new();
        schema.add_label("Person").unwrap();
        schema.add_label("Company").unwrap();

        let salary_prop = DbSchemaProperty::new("salary", PropertyType::FLOAT);
        schema
            .add_relationship_property("WORKS_FOR", &salary_prop)
            .unwrap();

        let query =
            "MATCH (p:Person)-[r:WORKS_FOR]->(c:Company) WHERE r.salary = 'high' RETURN p, c";

        // Parse the query
        let ast = parse_query(query).expect("Should parse successfully");

        // Extract query elements (this is what the Python binding does)
        let elements = extract_query_elements(&ast);

        println!("DEBUG: Extracted elements:");
        println!("  Variable relationship bindings: {:?}", elements.variable_relationship_bindings);
        println!("  Property comparisons: {:?}", elements.property_comparisons);

        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        let (errors, type_issues) =
            validate_query_elements_with_options(&elements, &schema, &options);

        // Should detect type mismatch: salary is FLOAT but compared with String
        assert!(errors.is_empty(), "Should have no schema validation errors");
        assert_eq!(
            type_issues.len(),
            1,
            "Expected 1 type issue for relationship property type mismatch from parsed query. Got {} issues",
            type_issues.len()
        );
        if !type_issues.is_empty() {
            assert!(type_issues[0].message.contains("salary"));
        }
    }
}
