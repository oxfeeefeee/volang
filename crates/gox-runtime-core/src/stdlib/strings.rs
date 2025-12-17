//! String manipulation core implementations.
//!
//! Pure logic for strings package functions.

use alloc::string::String;
use alloc::vec::Vec;

// ==================== Search Functions ====================

/// Check if string contains substring.
#[inline]
pub fn contains(s: &str, substr: &str) -> bool {
    s.contains(substr)
}

/// Check if string contains any character from chars.
pub fn contains_any(s: &str, chars: &str) -> bool {
    chars.chars().any(|c| s.contains(c))
}

/// Check if string starts with prefix.
#[inline]
pub fn has_prefix(s: &str, prefix: &str) -> bool {
    s.starts_with(prefix)
}

/// Check if string ends with suffix.
#[inline]
pub fn has_suffix(s: &str, suffix: &str) -> bool {
    s.ends_with(suffix)
}

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
        // Go behavior: empty substr returns len(s) + 1
        s.chars().count() + 1
    } else {
        s.matches(substr).count()
    }
}

// ==================== Transform Functions ====================

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

/// Remove prefix if present.
pub fn trim_prefix<'a>(s: &'a str, prefix: &str) -> &'a str {
    s.strip_prefix(prefix).unwrap_or(s)
}

/// Remove suffix if present.
pub fn trim_suffix<'a>(s: &'a str, suffix: &str) -> &'a str {
    s.strip_suffix(suffix).unwrap_or(s)
}

/// Replace first n occurrences of old with new. If n < 0, replace all.
pub fn replace(s: &str, old: &str, new: &str, n: i64) -> String {
    if n < 0 {
        s.replace(old, new)
    } else {
        s.replacen(old, new, n as usize)
    }
}

/// Replace all occurrences of old with new.
#[inline]
pub fn replace_all(s: &str, old: &str, new: &str) -> String {
    s.replace(old, new)
}

// ==================== Split/Join Functions ====================

/// Split string by separator.
pub fn split(s: &str, sep: &str) -> Vec<String> {
    if sep.is_empty() {
        // Split into characters (UTF-8 aware)
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

/// Repeat string n times.
pub fn repeat(s: &str, n: usize) -> String {
    s.repeat(n)
}

// ==================== Compare Functions ====================

/// Compare two strings lexicographically.
/// Returns -1 if a < b, 0 if a == b, 1 if a > b.
pub fn compare(a: &str, b: &str) -> i64 {
    match a.cmp(b) {
        core::cmp::Ordering::Less => -1,
        core::cmp::Ordering::Equal => 0,
        core::cmp::Ordering::Greater => 1,
    }
}

/// Case-insensitive string comparison.
#[inline]
pub fn equal_fold(s: &str, t: &str) -> bool {
    s.eq_ignore_ascii_case(t)
}
