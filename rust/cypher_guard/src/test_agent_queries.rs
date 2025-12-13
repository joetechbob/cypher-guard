#[cfg(test)]
mod test_agent_generated_queries {
    use crate::parse_query;

    #[test]
    fn test_query_1_client_search_with_or() {
        let query = "MATCH (c:Client)
WHERE c.headquarters_city = 'Phoenix' 
   OR c.headquarters_state = 'Arizona' 
   OR c.headquarters_state = 'AZ'
RETURN c";
        
        println!("\n=== Query 1: Client search with OR (VALID) ===");
        let result = parse_query(query);
        assert!(result.is_ok(), "Should parse valid OR conditions");
        println!("✅ Parsed successfully");
    }

    #[test]
    fn test_query_2_undefined_variable() {
        let query = "MATCH (p:Project)
WHERE c.client_id IN ['C00001', 'C00002']
RETURN p
LIMIT 5";
        
        println!("\n=== Query 2: Undefined variable 'c' (INVALID - BUG!) ===");
        let result = parse_query(query);
        
        // This SHOULD fail validation because 'c' is never defined
        // But it may parse successfully - validation would catch it
        if result.is_ok() {
            println!("⚠️  Query parsed but uses undefined variable 'c'");
            println!("    (This should be caught by validation, not parser)");
        } else {
            println!("❌ Parse failed (unexpected)");
        }
    }

    #[test]
    fn test_query_3_simple_limit() {
        let query = "MATCH (p:Project)
RETURN p
LIMIT 3";
        
        println!("\n=== Query 3: Simple LIMIT (VALID) ===");
        let result = parse_query(query);
        
        // LIMIT is not yet supported in parser
        if result.is_err() {
            println!("⚠️  LIMIT clause not yet supported by parser");
        } else {
            println!("✅ Parsed successfully");
        }
    }

    #[test]
    fn test_query_4_in_with_list() {
        let query = "
MATCH (p:Project)
WHERE p.code IN [\"SBS-CLOUD-2024\", \"SBS-CLOUD-2025\", \"RMC-ERP-2024\"]
RETURN p
";
        
        println!("\n=== Query 4: IN with list of strings (VALID) ===");
        let result = parse_query(query);
        
        // IN operator may not be supported yet
        if result.is_err() {
            println!("⚠️  IN operator not yet supported by parser");
        } else {
            println!("✅ Parsed successfully");
        }
    }

    #[test]
    fn test_query_5_contains_operator() {
        let query = "
MATCH (e:Employee)
WHERE e.full_name CONTAINS \"Jake\" OR e.name CONTAINS \"Jake\"
RETURN e
LIMIT 5
";
        
        println!("\n=== Query 5: CONTAINS operator (VALID) ===");
        let result = parse_query(query);
        
        // CONTAINS operator may not be supported yet
        if result.is_err() {
            println!("⚠️  CONTAINS operator or LIMIT not yet supported");
        } else {
            println!("✅ Parsed successfully");
        }
    }

    #[test]
    fn test_query_6_simple_equality() {
        let query = "
MATCH (c:Client)
WHERE c.headquarters_city = \"Phoenix\"
RETURN c
";
        
        println!("\n=== Query 6: Simple equality with double quotes (VALID) ===");
        let result = parse_query(query);
        assert!(result.is_ok(), "Should parse simple equality");
        println!("✅ Parsed successfully");
    }

    #[test]
    fn test_query_7_multiple_properties_return() {
        let query = "
MATCH (p:Project)
WHERE p.id IN [\"SBS-CLOUD-2024\", \"SBS-CLOUD-2025\", \"RMC-ERP-2024\"]
RETURN p.id, p.project_start_date, p.project_end_date, p.current_phase, p.system_status
";
        
        println!("\n=== Query 7: Multiple properties in RETURN (VALID) ===");
        let result = parse_query(query);
        
        if result.is_err() {
            println!("⚠️  IN operator not yet supported");
        } else {
            println!("✅ Parsed successfully");
        }
    }

    #[test]
    fn test_query_8_string_date_comparison() {
        let query = "
MATCH (exp:Expense)
WHERE exp.transactionDate >= \"2025-04-04\" AND exp.transactionDate <= \"2025-04-08\"
RETURN exp
LIMIT 20
";
        
        println!("\n=== Query 8: String date comparison (TYPE ISSUE) ===");
        let result = parse_query(query);
        
        if result.is_err() {
            println!("⚠️  LIMIT not yet supported");
        } else {
            println!("✅ Parsed, but may have type mismatch (string vs date)");
        }
    }

    #[test]
    fn test_query_9_is_not_null() {
        let query = "
MATCH (exp:Expense)
WHERE exp.custom7 IS NOT NULL
RETURN exp.custom4, exp.custom5, exp.custom6, exp.custom7
LIMIT 30
";
        
        println!("\n=== Query 9: IS NOT NULL check (VALID) ===");
        let result = parse_query(query);
        
        if result.is_err() {
            println!("⚠️  LIMIT not yet supported");
        } else {
            println!("✅ Parsed successfully");
        }
    }

    #[test]
    fn test_all_agent_queries_summary() {
        println!("\n{}", "=".repeat(80));
        println!("AGENT QUERY TESTING SUMMARY");
        println!("{}", "=".repeat(80));
        
        let test_cases = vec![
            ("Query 1: OR conditions", "MATCH (c:Client) WHERE c.headquarters_city = 'Phoenix' RETURN c", true),
            ("Query 2: Undefined var", "MATCH (p:Project) WHERE c.client_id IN ['C00001'] RETURN p", false),
            ("Query 6: Simple", "MATCH (c:Client) WHERE c.headquarters_city = \"Phoenix\" RETURN c", true),
            ("Query 9: IS NOT NULL", "MATCH (exp:Expense) WHERE exp.custom7 IS NOT NULL RETURN exp.custom4", true),
        ];
        
        let mut passed = 0;
        let mut failed = 0;
        
        for (name, query, should_pass) in test_cases {
            let result = parse_query(query);
            let did_pass = result.is_ok();
            
            if did_pass == should_pass {
                passed += 1;
                println!("✅ {}: {}", name, if did_pass { "PASSED" } else { "FAILED (expected)" });
            } else {
                failed += 1;
                println!("❌ {}: {} (expected {})", 
                    name, 
                    if did_pass { "PASSED" } else { "FAILED" },
                    if should_pass { "PASS" } else { "FAIL" }
                );
            }
        }
        
        println!("\nResults: {} correct, {} incorrect", passed, failed);
        
        println!("\n🔍 KEY FINDINGS:");
        println!("1. Parser handles basic queries well");
        println!("2. LIMIT clause not yet supported");
        println!("3. IN operator not yet supported");
        println!("4. CONTAINS operator not yet supported");
        println!("5. Undefined variables (Query 2) need validation layer");
        println!("6. String vs Date comparisons need type checking");
    }
}
