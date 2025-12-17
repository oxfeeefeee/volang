//! Time core implementations.
//!
//! Pure logic for time package functions.

/// Get current time as nanoseconds since Unix epoch.
#[cfg(feature = "std")]
pub fn now() -> i64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as i64)
        .unwrap_or(0)
}

/// Sleep for the specified number of nanoseconds.
#[cfg(feature = "std")]
pub fn sleep(nanos: i64) {
    use std::time::Duration;
    use std::thread;
    if nanos > 0 {
        thread::sleep(Duration::from_nanos(nanos as u64));
    }
}

/// Calculate elapsed time since start (in nanoseconds).
#[cfg(feature = "std")]
pub fn since(start: i64) -> i64 {
    now() - start
}

/// Convert seconds and nanoseconds to total nanoseconds.
pub fn unix(sec: i64, nsec: i64) -> i64 {
    sec * 1_000_000_000 + nsec
}

/// Convert milliseconds to nanoseconds.
pub fn unix_milli(msec: i64) -> i64 {
    msec * 1_000_000
}

/// Parse duration string like "1h30m", "100ms", "2.5s".
pub fn parse_duration(s: &str) -> i64 {
    let s = s.trim();
    if s.is_empty() {
        return 0;
    }
    
    let mut total: i64 = 0;
    let mut num_start = 0;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        while i < chars.len() && chars[i].is_whitespace() {
            i += 1;
            num_start = i;
        }
        
        let mut has_decimal = false;
        while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
            if chars[i] == '.' {
                has_decimal = true;
            }
            i += 1;
        }
        
        if i == num_start {
            break;
        }
        
        let num_str: String = chars[num_start..i].iter().collect();
        let value: f64 = num_str.parse().unwrap_or(0.0);
        
        let unit_start = i;
        while i < chars.len() && chars[i].is_alphabetic() {
            i += 1;
        }
        
        let unit: String = chars[unit_start..i].iter().collect();
        let multiplier: i64 = match unit.as_str() {
            "ns" => 1,
            "us" | "µs" => 1_000,
            "ms" => 1_000_000,
            "s" => 1_000_000_000,
            "m" => 60 * 1_000_000_000,
            "h" => 3600 * 1_000_000_000,
            _ => 1,
        };
        
        if has_decimal {
            total += (value * multiplier as f64) as i64;
        } else {
            total += value as i64 * multiplier;
        }
        
        num_start = i;
    }
    
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_duration() {
        assert_eq!(parse_duration("1s"), 1_000_000_000);
        assert_eq!(parse_duration("100ms"), 100_000_000);
        assert_eq!(parse_duration("1h30m"), 5400_000_000_000);
        assert_eq!(parse_duration("2.5s"), 2_500_000_000);
    }
    
    #[test]
    fn test_unix() {
        assert_eq!(unix(1, 500), 1_000_000_500);
    }
    
    #[test]
    fn test_unix_milli() {
        assert_eq!(unix_milli(1000), 1_000_000_000);
    }
}
