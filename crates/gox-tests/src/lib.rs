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

    #[test]
    fn test_run_source_simple() {
        let result = run_source("simple_test", r#"
package main

func main() {
    x := 1 + 2
    assert(x == 3, "1 + 2 should be 3")
}
"#);
        assert!(result.passed, "Test failed: {:?}", result.error);
    }

    #[test]
    fn test_run_source_interface() {
        let result = run_source("interface_test", r#"
package main

type Adder interface {
    Add() int
}

type MyNum struct {
    value int
}

func (m MyNum) Add() int {
    return m.value + 100
}

func main() {
    var a Adder
    n := MyNum{value: 42}
    a = n
    result := a.Add()
    assert(result == 142, "interface method should work")
}
"#);
        assert!(result.passed, "Test failed: {:?}", result.error);
    }

    #[test]
    fn test_run_source_with_vfs() {
        let mut vfs = VirtualFs::new();
        vfs.add_file("main.gox", r#"
package main

func main() {
    s := []int{1, 2, 3}
    assert(len(s) == 3, "slice length")
    assert(s[0] == 1, "slice element")
}
"#);
        
        let result = run_source_with_vfs("vfs_test", &vfs);
        assert!(result.passed, "Test failed: {:?}", result.error);
    }
}
