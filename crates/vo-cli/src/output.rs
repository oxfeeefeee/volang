//! Output tags for script parsing.
//!
//! These tags are used by test scripts to determine execution results.

/// Success tag - execution completed without errors.
pub const TAG_OK: &str = "[VO:OK]";

/// Panic tag prefix - program panicked during execution.
pub const TAG_PANIC_PREFIX: &str = "[VO:PANIC:";

/// Error tag prefix - compilation or analysis error.
pub const TAG_ERROR_PREFIX: &str = "[VO:ERROR:";

/// Format a panic message with the standard tag.
pub fn format_panic(msg: &str) -> String {
    format!("{}{}]", TAG_PANIC_PREFIX, msg)
}

/// Format an error message with the standard tag.
pub fn format_error(msg: &str) -> String {
    format!("{}{}]", TAG_ERROR_PREFIX, msg)
}
