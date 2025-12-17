//! bytes core implementations.
//!
//! Pure logic for bytes package functions.
//! GoX implementations: Equal, Compare, HasPrefix, HasSuffix, Contains, TrimPrefix, TrimSuffix

/// Find first occurrence of sep in b
pub fn index(b: &[u8], sep: &[u8]) -> i64 {
    if sep.is_empty() {
        return 0;
    }
    b.windows(sep.len())
        .position(|w| w == sep)
        .map(|i| i as i64)
        .unwrap_or(-1)
}

/// Find last occurrence of sep in b
pub fn last_index(b: &[u8], sep: &[u8]) -> i64 {
    if sep.is_empty() {
        return b.len() as i64;
    }
    b.windows(sep.len())
        .rposition(|w| w == sep)
        .map(|i| i as i64)
        .unwrap_or(-1)
}

/// Count non-overlapping occurrences of sep in b
pub fn count(b: &[u8], sep: &[u8]) -> usize {
    if sep.is_empty() {
        return b.len() + 1;
    }
    let mut count = 0;
    let mut start = 0;
    while start + sep.len() <= b.len() {
        if &b[start..start + sep.len()] == sep {
            count += 1;
            start += sep.len();
        } else {
            start += 1;
        }
    }
    count
}

/// Find first occurrence of byte c in b
pub fn index_byte(b: &[u8], c: u8) -> i64 {
    b.iter().position(|&x| x == c).map(|i| i as i64).unwrap_or(-1)
}

/// Find last occurrence of byte c in b
pub fn last_index_byte(b: &[u8], c: u8) -> i64 {
    b.iter().rposition(|&x| x == c).map(|i| i as i64).unwrap_or(-1)
}

/// Convert to lowercase
pub fn to_lower(b: &[u8]) -> Vec<u8> {
    b.iter().map(|&c| c.to_ascii_lowercase()).collect()
}

/// Convert to uppercase
pub fn to_upper(b: &[u8]) -> Vec<u8> {
    b.iter().map(|&c| c.to_ascii_uppercase()).collect()
}

/// Trim whitespace from both ends
pub fn trim_space(b: &[u8]) -> &[u8] {
    let start = b.iter().position(|&c| !c.is_ascii_whitespace()).unwrap_or(b.len());
    let end = b.iter().rposition(|&c| !c.is_ascii_whitespace()).map(|i| i + 1).unwrap_or(0);
    if start >= end { &[] } else { &b[start..end] }
}

/// Trim specified bytes from both ends
pub fn trim<'a>(b: &'a [u8], cutset: &[u8]) -> &'a [u8] {
    let start = b.iter().position(|c| !cutset.contains(c)).unwrap_or(b.len());
    let end = b.iter().rposition(|c| !cutset.contains(c)).map(|i| i + 1).unwrap_or(0);
    if start >= end { &[] } else { &b[start..end] }
}

/// Repeat b count times
pub fn repeat(b: &[u8], count: usize) -> Vec<u8> {
    b.repeat(count)
}

/// Join slices with separator
pub fn join(slices: &[&[u8]], sep: &[u8]) -> Vec<u8> {
    if slices.is_empty() {
        return Vec::new();
    }
    let total_len = slices.iter().map(|s| s.len()).sum::<usize>() 
        + sep.len() * slices.len().saturating_sub(1);
    let mut result = Vec::with_capacity(total_len);
    for (i, slice) in slices.iter().enumerate() {
        if i > 0 {
            result.extend_from_slice(sep);
        }
        result.extend_from_slice(slice);
    }
    result
}

/// Split by separator
pub fn split(b: &[u8], sep: &[u8]) -> Vec<Vec<u8>> {
    if sep.is_empty() {
        return b.iter().map(|&c| vec![c]).collect();
    }
    let mut result = Vec::new();
    let mut start = 0;
    while start <= b.len() {
        match b[start..].windows(sep.len()).position(|w| w == sep) {
            Some(pos) => {
                result.push(b[start..start + pos].to_vec());
                start = start + pos + sep.len();
            }
            None => {
                result.push(b[start..].to_vec());
                break;
            }
        }
    }
    result
}
