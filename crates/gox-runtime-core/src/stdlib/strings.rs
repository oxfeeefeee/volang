//! String manipulation core implementations.
//!
//! Native functions only - GoX-implemented functions are compiled directly.
//! GoX implementations: contains, has_prefix, has_suffix, trim_prefix, trim_suffix,
//!                      repeat, compare, replace_all

use alloc::string::String;
use alloc::vec::Vec;

// ==================== Search Functions (Native) ====================

/// Find index of first occurrence of substr, or -1 if not found.
pub fn index(s: &str, substr: &str) -> i64 {
    s.find(substr).map(|i| i as i64).unwrap_or(-1)
}

/// Find index of last occurrence of substr, or -1 if not found.
pub fn last_index(s: &str, substr: &str) -> i64 {
    s.rfind(substr).map(|i| i as i64).unwrap_or(-1)
}

/// Count non-overlapping occurrences of substr.
pub fn count(s: &str, substr: &str) -> usize {
    if substr.is_empty() {
        s.chars().count() + 1
    } else {
        s.matches(substr).count()
    }
}

/// Check if string contains any character from chars.
pub fn contains_any(s: &str, chars: &str) -> bool {
    chars.chars().any(|c| s.contains(c))
}

// ==================== Transform Functions (Native) ====================

/// Convert string to lowercase.
pub fn to_lower(s: &str) -> String {
    s.to_lowercase()
}

/// Convert string to uppercase.
pub fn to_upper(s: &str) -> String {
    s.to_uppercase()
}

/// Trim whitespace from both ends.
pub fn trim_space(s: &str) -> &str {
    s.trim()
}

/// Trim characters in cutset from both ends.
pub fn trim(s: &str, cutset: &str) -> String {
    let chars: Vec<char> = cutset.chars().collect();
    s.trim_matches(|c| chars.contains(&c)).to_string()
}

/// Replace first n occurrences of old with new. If n < 0, replace all.
pub fn replace(s: &str, old: &str, new: &str, n: i64) -> String {
    if n < 0 {
        s.replace(old, new)
    } else {
        s.replacen(old, new, n as usize)
    }
}

// ==================== Split/Join Functions (Native) ====================

/// Split string by separator.
pub fn split(s: &str, sep: &str) -> Vec<String> {
    if sep.is_empty() {
        s.chars().map(|c| c.to_string()).collect()
    } else {
        s.split(sep).map(|p| p.to_string()).collect()
    }
}

/// Split string by separator, at most n parts.
pub fn split_n(s: &str, sep: &str, n: usize) -> Vec<String> {
    if n == 0 {
        Vec::new()
    } else {
        s.splitn(n, sep).map(|p| p.to_string()).collect()
    }
}

/// Join strings with separator.
pub fn join(parts: &[&str], sep: &str) -> String {
    parts.join(sep)
}

// ==================== Compare Functions (Native) ====================

/// Case-insensitive string comparison.
#[inline]
pub fn equal_fold(s: &str, t: &str) -> bool {
    s.eq_ignore_ascii_case(t)
}
