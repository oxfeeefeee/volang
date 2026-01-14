//! regexp package native function implementations.
//!
//! Regular expression matching using Rust's regex crate.

use vo_ffi_macro::vo_extern_std;

#[cfg(feature = "std")]
use regex::Regex;

// ==================== Matching ====================

#[cfg(feature = "std")]
#[vo_extern_std("regexp", "matchString")]
fn match_string(pattern: &str, s: &str) -> (bool, bool) {
    // Returns (matched, valid_pattern)
    match Regex::new(pattern) {
        Ok(re) => (re.is_match(s), true),
        Err(_) => (false, false),
    }
}

#[cfg(feature = "std")]
#[vo_extern_std("regexp", "matchBytes")]
fn match_bytes(pattern: &str, b: &[u8]) -> (bool, bool) {
    // Returns (matched, valid_pattern)
    match Regex::new(pattern) {
        Ok(re) => {
            match std::str::from_utf8(b) {
                Ok(s) => (re.is_match(s), true),
                Err(_) => (false, true), // Valid pattern but invalid UTF-8
            }
        }
        Err(_) => (false, false),
    }
}

// ==================== Find ====================

#[cfg(feature = "std")]
#[vo_extern_std("regexp", "findString")]
fn find_string(pattern: &str, s: &str) -> String {
    match Regex::new(pattern) {
        Ok(re) => re.find(s).map(|m| m.as_str().to_string()).unwrap_or_default(),
        Err(_) => String::new(),
    }
}

#[cfg(feature = "std")]
#[vo_extern_std("regexp", "findStringIndex")]
fn find_string_index(pattern: &str, s: &str) -> (i64, i64) {
    match Regex::new(pattern) {
        Ok(re) => re.find(s).map(|m| (m.start() as i64, m.end() as i64)).unwrap_or((-1, -1)),
        Err(_) => (-1, -1),
    }
}

#[cfg(feature = "std")]
#[vo_extern_std("regexp", "findAllString")]
fn find_all_string(pattern: &str, s: &str, n: i64) -> Vec<String> {
    match Regex::new(pattern) {
        Ok(re) => {
            if n < 0 {
                re.find_iter(s).map(|m| m.as_str().to_string()).collect()
            } else if n == 0 {
                Vec::new()
            } else {
                re.find_iter(s).take(n as usize).map(|m| m.as_str().to_string()).collect()
            }
        }
        Err(_) => Vec::new(),
    }
}

// ==================== Replace ====================

#[cfg(feature = "std")]
#[vo_extern_std("regexp", "replaceAllString")]
fn replace_all_string(pattern: &str, src: &str, repl: &str) -> String {
    match Regex::new(pattern) {
        Ok(re) => re.replace_all(src, repl).into_owned(),
        Err(_) => src.to_string(),
    }
}

#[cfg(feature = "std")]
#[vo_extern_std("regexp", "replaceAllLiteralString")]
fn replace_all_literal_string(pattern: &str, src: &str, repl: &str) -> String {
    match Regex::new(pattern) {
        Ok(re) => re.replace_all(src, regex::NoExpand(repl)).into_owned(),
        Err(_) => src.to_string(),
    }
}

// ==================== Split ====================

#[cfg(feature = "std")]
#[vo_extern_std("regexp", "splitString")]
fn split_string(pattern: &str, s: &str, n: i64) -> Vec<String> {
    match Regex::new(pattern) {
        Ok(re) => {
            if n < 0 {
                re.split(s).map(|s| s.to_string()).collect()
            } else if n == 0 {
                Vec::new()
            } else {
                re.splitn(s, n as usize).map(|s| s.to_string()).collect()
            }
        }
        Err(_) => vec![s.to_string()],
    }
}

// ==================== Submatch ====================

#[cfg(feature = "std")]
#[vo_extern_std("regexp", "findStringSubmatch")]
fn find_string_submatch(pattern: &str, s: &str) -> Vec<String> {
    match Regex::new(pattern) {
        Ok(re) => {
            re.captures(s)
                .map(|caps| {
                    caps.iter()
                        .map(|m| m.map(|m| m.as_str().to_string()).unwrap_or_default())
                        .collect()
                })
                .unwrap_or_default()
        }
        Err(_) => Vec::new(),
    }
}

// ==================== Quote ====================

#[cfg(feature = "std")]
#[vo_extern_std("regexp", "quoteMeta")]
fn quote_meta(s: &str) -> String {
    regex::escape(s)
}
