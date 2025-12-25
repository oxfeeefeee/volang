//! Builtin native function implementations.
//!
//! These are low-level builtin functions called directly by runtime.
//! They don't have corresponding .vo declarations and skip signature validation.

use vo_ffi_macro::vo_builtin;

/// vo_print - print string without newline
#[vo_builtin("vo_print")]
fn print(s: &str) -> i64 {
    print!("{}", s);
    s.len() as i64
}

/// vo_println - print string with newline
#[vo_builtin("vo_println")]
fn println(s: &str) -> i64 {
    println!("{}", s);
    s.len() as i64 + 1
}

/// vo_assert - assert condition
#[vo_builtin("vo_assert")]
fn assert(cond: bool) {
    if !cond {
        panic!("assertion failed");
    }
}
