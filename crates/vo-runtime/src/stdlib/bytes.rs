//! bytes package native function implementations.
//!
//! Provides byte slice manipulation functions for the bytes standard library package.

use vo_ffi_macro::vo_extern_std;

// ==================== Comparison ====================

#[vo_extern_std("bytes", "Equal")]
fn equal(a: &[u8], b: &[u8]) -> bool {
    a == b
}

#[vo_extern_std("bytes", "Compare")]
fn compare(a: &[u8], b: &[u8]) -> i64 {
    match a.cmp(b) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

// ==================== Search ====================

#[vo_extern_std("bytes", "Index")]
fn index(s: &[u8], sep: &[u8]) -> i64 {
    if sep.is_empty() {
        return 0;
    }
    s.windows(sep.len())
        .position(|w| w == sep)
        .map(|i| i as i64)
        .unwrap_or(-1)
}

#[vo_extern_std("bytes", "LastIndex")]
fn last_index(s: &[u8], sep: &[u8]) -> i64 {
    if sep.is_empty() {
        return s.len() as i64;
    }
    s.windows(sep.len())
        .rposition(|w| w == sep)
        .map(|i| i as i64)
        .unwrap_or(-1)
}

#[vo_extern_std("bytes", "IndexByte")]
fn index_byte(s: &[u8], c: u8) -> i64 {
    s.iter()
        .position(|&b| b == c)
        .map(|i| i as i64)
        .unwrap_or(-1)
}

#[vo_extern_std("bytes", "LastIndexByte")]
fn last_index_byte(s: &[u8], c: u8) -> i64 {
    s.iter()
        .rposition(|&b| b == c)
        .map(|i| i as i64)
        .unwrap_or(-1)
}

#[vo_extern_std("bytes", "Count")]
fn count(s: &[u8], sep: &[u8]) -> i64 {
    if sep.is_empty() {
        return (s.len() + 1) as i64;
    }
    let mut count = 0i64;
    let mut start = 0;
    while start + sep.len() <= s.len() {
        if &s[start..start + sep.len()] == sep {
            count += 1;
            start += sep.len();
        } else {
            start += 1;
        }
    }
    count
}

// ==================== Case conversion ====================

#[vo_extern_std("bytes", "ToLower")]
fn to_lower(s: &[u8]) -> Vec<u8> {
    s.to_ascii_lowercase()
}

#[vo_extern_std("bytes", "ToUpper")]
fn to_upper(s: &[u8]) -> Vec<u8> {
    s.to_ascii_uppercase()
}

// ==================== Repetition ====================

#[vo_extern_std("bytes", "Repeat")]
fn repeat(s: &[u8], count: i64) -> Vec<u8> {
    if count <= 0 {
        return Vec::new();
    }
    s.repeat(count as usize)
}
