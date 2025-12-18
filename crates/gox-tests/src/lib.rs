//! GoX Test Framework
//!
//! File-based integration tests for the GoX compiler.
//! See README.md for test file format documentation.

mod printer;
mod runner;

pub use printer::AstPrinter;
pub use runner::{
    TestResult, TestSummary, RunMode,
    run_all, run_all_with_mode,
    run_single_file, run_single_file_with_mode,
    run_multi_file, run_multi_file_with_mode,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_all() {
        let test_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("test_data");
        if !test_dir.exists() {
            return;
        }
        
        let modes = [RunMode::Vm, RunMode::Jit];
        let mut all_failures = Vec::new();
        
        for mode in modes {
            let summary = run_all_with_mode(&test_dir, mode);
            
            eprintln!("\n══════════════════════════════════════════");
            eprintln!("  GoX Test Results [{}]: {} passed, {} failed, {} skipped", 
                mode, summary.passed, summary.failed, summary.skipped);
            eprintln!("══════════════════════════════════════════\n");
            
            for failure in summary.failures {
                all_failures.push((mode, failure));
            }
        }
        
        if !all_failures.is_empty() {
            let mut msg = format!("\n{} tests failed:\n", all_failures.len());
            for (mode, failure) in &all_failures {
                msg.push_str(&format!("  ✗ {} [{}]\n", failure.path, mode));
                if let Some(err) = &failure.error {
                    for line in err.lines().take(5) {
                        msg.push_str(&format!("    {}\n", line));
                    }
                }
            }
            panic!("{}", msg);
        }
    }
}
