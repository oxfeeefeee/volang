//! CLI tool for running file-based GoX tests.
//!
//! Usage:
//!   gox-tests                    # Run all tests in test_data/
//!   gox-tests <file.gox>         # Run a single test file
//!   gox-tests <dir>              # Run all tests in a directory

use std::env;
use std::path::Path;
use std::process::ExitCode;

use gox_tests::{run_test_file, run_all_tests};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    
    let path = if args.len() > 1 {
        Path::new(&args[1]).to_path_buf()
    } else {
        // Default to test_data directory relative to manifest
        Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data")
    };
    
    if !path.exists() {
        eprintln!("Error: path does not exist: {}", path.display());
        return ExitCode::FAILURE;
    }
    
    let results = if path.is_file() {
        vec![run_test_file(&path)]
    } else {
        run_all_tests(&path)
    };
    
    let total = results.len();
    let passed = results.iter().filter(|r| r.passed).count();
    let failed = total - passed;
    
    println!("\n{}", "=".repeat(60));
    
    for result in &results {
        let status = if result.passed { "✓" } else { "✗" };
        println!("{} {}: {}", status, result.file_name, result.message);
        
        if !result.passed && (!result.expected.is_empty() || !result.actual.is_empty()) {
            println!("\n  Expected:");
            for line in result.expected.lines() {
                println!("    {}", line);
            }
            println!("\n  Actual:");
            for line in result.actual.lines() {
                println!("    {}", line);
            }
            println!();
        }
    }
    
    println!("{}", "=".repeat(60));
    println!("Results: {} passed, {} failed, {} total", passed, failed, total);
    
    if failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
