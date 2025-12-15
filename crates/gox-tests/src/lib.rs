//! File-based integration tests for the GoX compiler.
//!
//! This crate provides a test framework that reads `.gox` test files containing
//! both source code and expected output from various compiler phases.
//!
//! # Test File Format
//!
//! Test files use a simple format with sections separated by markers:
//!
//! ```text
//! // Test description (optional)
//! package main
//!
//! func main() {
//!     x := 1
//! }
//!
//! === parser ===
//! File {
//!     package: Some("main"),
//!     decls: [
//!         Func { name: "main", ... }
//!     ]
//! }
//!
//! === errors ===
//! // Expected error messages (if any)
//! ```
//!
//! Supported sections:
//! - `=== parser ===` - Expected parser AST output
//! - `=== errors ===` - Expected error messages
//! - `=== typecheck ===` - Expected type checker output (future)
//! - `=== bytecode ===` - Expected bytecode output (future)

mod printer;
mod runner;

pub use printer::AstPrinter;
pub use runner::{TestRunner, TestResult, run_test_file, run_all_tests};
pub use runner::{VirtualFs, CodegenTestResult, run_source, run_source_with_vfs};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_run_all_parser_tests() {
        let test_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data");
        if !test_dir.exists() {
            return; // No test files yet
        }
        
        let results = run_all_tests(&test_dir);
        let mut failures = Vec::new();
        
        for result in &results {
            if !result.passed {
                failures.push(format!(
                    "{}: {}\n  Expected:\n{}\n  Actual:\n{}",
                    result.file_name,
                    result.message,
                    result.expected.lines().map(|l| format!("    {}", l)).collect::<Vec<_>>().join("\n"),
                    result.actual.lines().map(|l| format!("    {}", l)).collect::<Vec<_>>().join("\n"),
                ));
            }
        }
        
        if !failures.is_empty() {
            panic!("Test failures:\n{}", failures.join("\n\n"));
        }
    }

    #[test]
    fn test_run_all_typecheck_tests() {
        let test_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data").join("typecheck");
        if !test_dir.exists() {
            return; // No typecheck test files yet
        }
        
        let results = run_all_tests(&test_dir);
        let mut failures = Vec::new();
        
        for result in &results {
            if !result.passed {
                failures.push(format!(
                    "{}: {}\n  Expected:\n{}\n  Actual:\n{}",
                    result.file_name,
                    result.message,
                    result.expected.lines().map(|l| format!("    {}", l)).collect::<Vec<_>>().join("\n"),
                    result.actual.lines().map(|l| format!("    {}", l)).collect::<Vec<_>>().join("\n"),
                ));
            }
        }
        
        if !failures.is_empty() {
            panic!("Typecheck test failures:\n{}", failures.join("\n\n"));
        }
    }

    // ========== Codegen tests from test_data/codegen ==========
    
    fn run_codegen_file(path: &Path) -> CodegenTestResult {
        let name = path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string());
        
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                return CodegenTestResult {
                    name: name.clone(),
                    passed: false,
                    output: String::new(),
                    error: Some(format!("Failed to read file: {}", e)),
                };
            }
        };
        
        run_source(&name, &source)
    }
    
    #[test]
    fn test_codegen_closure() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/codegen/closure/closure.gox");
        let result = run_codegen_file(&path);
        assert!(result.passed, "closure test failed: {:?}", result.error);
    }
    
    #[test]
    fn test_codegen_const() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/codegen/const/const.gox");
        let result = run_codegen_file(&path);
        assert!(result.passed, "const test failed: {:?}", result.error);
    }
    
    #[test]
    fn test_codegen_const_fold() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/codegen/const/const_fold.gox");
        let result = run_codegen_file(&path);
        assert!(result.passed, "const_fold test failed: {:?}", result.error);
    }
    
    #[test]
    fn test_codegen_constant() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/codegen/const/constant.gox");
        let result = run_codegen_file(&path);
        assert!(result.passed, "constant test failed: {:?}", result.error);
    }
    
    #[test]
    fn test_codegen_slice() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/codegen/slice/slice.gox");
        let result = run_codegen_file(&path);
        assert!(result.passed, "slice test failed: {:?}", result.error);
    }
    
    #[test]
    fn test_codegen_map() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/codegen/map/map.gox");
        let result = run_codegen_file(&path);
        assert!(result.passed, "map test failed: {:?}", result.error);
    }
    
    #[test]
    fn test_codegen_slice_map() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/codegen/map/slice_map.gox");
        let result = run_codegen_file(&path);
        assert!(result.passed, "slice_map test failed: {:?}", result.error);
    }
    
    #[test]
    fn test_codegen_struct_object() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/codegen/struct/struct_object.gox");
        let result = run_codegen_file(&path);
        assert!(result.passed, "struct_object test failed: {:?}", result.error);
    }
    
    #[test]
    fn test_codegen_complex_literal() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/codegen/struct/complex_literal.gox");
        let result = run_codegen_file(&path);
        assert!(result.passed, "complex_literal test failed: {:?}", result.error);
    }
    
    #[test]
    fn test_codegen_interface() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/codegen/interface/interface.gox");
        let result = run_codegen_file(&path);
        assert!(result.passed, "interface test failed: {:?}", result.error);
    }
}
