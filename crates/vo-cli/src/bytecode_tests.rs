//! Bytecode tests for VM verification.
//!
//! Run with: `vo run-bytecode --test <name>`
//!
//! TODO: Rewrite tests to use new vo-vm interfaces

/// Run a bytecode test by name.
pub fn run_test(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Bytecode tests temporarily disabled ===");
    println!("Requested test: {}", name);
    println!("TODO: Rewrite tests to use new vo-vm interfaces");
    Err("bytecode tests not yet implemented for new VM".into())
}
