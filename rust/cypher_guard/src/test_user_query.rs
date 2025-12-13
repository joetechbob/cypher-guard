#[cfg(test)]
mod test_user_exact_query {
    use crate::parse_query;
    use crate::schema::{DbSchema, DbSchemaProperty, DbSchemaRelationshipPattern, PropertyType};
    use crate::validation::{extract_query_elements, validate_query_elements_with_options, ValidationOptions};
    use crate::types::TypeCheckLevel;

    #[test]
    fn test_exact_user_query_string_date_comparison() {
        // Create schema with STRING properties for valid_from/valid_to (the issue!)
        let mut schema = DbSchema::new();
        
        // Add all labels used in query
        schema.add_label("ProjectStaffing").unwrap();
        schema.add_label("Employee").unwrap();
        schema.add_label("Project").unwrap();
        
        // Add relationships
        schema.add_relationship_pattern(DbSchemaRelationshipPattern::new(
            "ProjectStaffing", "Employee", "ASSIGNS"
        )).unwrap();
        schema.add_relationship_pattern(DbSchemaRelationshipPattern::new(
            "ProjectStaffing", "Project", "TO_PROJECT"
        )).unwrap();
        
        // Add the problematic STRING properties (should be Date!)
        let valid_from_prop = DbSchemaProperty::new("valid_from", PropertyType::STRING);
        let valid_to_prop = DbSchemaProperty::new("valid_to", PropertyType::STRING);
        
        schema.add_node_property("ProjectStaffing", &valid_from_prop).unwrap();
        schema.add_node_property("ProjectStaffing", &valid_to_prop).unwrap();
        
        // Add email property for Employee
        let email_prop = DbSchemaProperty::new("email", PropertyType::STRING);
        schema.add_node_property("Employee", &email_prop).unwrap();

        // THE EXACT USER QUERY - NOW WORKS!
        let query = r#"
MATCH (ps:ProjectStaffing)-[:ASSIGNS]->(e:Employee {email: 'jake.morrison@agassizconsulting.com'})
MATCH (ps)-[:TO_PROJECT]->(p:Project)
WHERE ps.valid_from <= date('2025-04-08') 
  AND ps.valid_to >= date('2025-04-04')
RETURN ps, p
        "#;

        // Parse the query - should work now with function call support!
        let ast = parse_query(query).expect("❌ Parse failed - function calls not supported in WHERE!");
        let elements = extract_query_elements(&ast);
        
        println!("\n✅ Query parsed successfully with date() functions in WHERE clause!");

        // Validate with type checking in STRICT mode
        let options = ValidationOptions {
            type_checking: TypeCheckLevel::Strict,
        };
        
        let (errors, type_issues) = validate_query_elements_with_options(&elements, &schema, &options);

        // Should have NO schema errors (query structure is valid)
        assert!(errors.is_empty(), "Should have no schema validation errors, got: {:?}", errors);

        // Should have TYPE ISSUES (String vs Date comparison)
        assert!(!type_issues.is_empty(), "❌ FAILED: Should detect String vs Date comparison!");
        
        println!("\n🎯 TYPE CHECKING RESULTS:");
        println!("  Schema Errors: {}", errors.len());
        println!("  Type Issues: {}", type_issues.len());
        
        for issue in &type_issues {
            println!("\n  🚨 {:?}: {}", issue.severity, issue.message);
            if let Some(suggestion) = &issue.suggestion {
                println!("     💡 Suggestion: {}", suggestion);
            }
        }
        
        // Verify we detected BOTH comparisons
        assert_eq!(type_issues.len(), 2, "Should detect both valid_from and valid_to comparisons");
        
        // Verify the error messages mention the properties
        let messages: Vec<String> = type_issues.iter().map(|i| i.message.clone()).collect();
        assert!(messages.iter().any(|m| m.contains("valid_from")), "Should detect valid_from comparison");
        assert!(messages.iter().any(|m| m.contains("valid_to")), "Should detect valid_to comparison");
        
        println!("\n✅ SUCCESS: Detected STRING properties compared to date() functions!");
    }
}
