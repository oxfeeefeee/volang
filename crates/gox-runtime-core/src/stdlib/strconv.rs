//! strconv core implementations.
//!
//! Pure logic for string conversion functions.
//! GoX implementations: FormatBool, ParseBool (in stdlib/strconv/strconv.gox)

/// Parse integer from string (Atoi)
pub fn atoi(s: &str) -> Result<i64, ()> {
    s.trim().parse::<i64>().map_err(|_| ())
}

/// Format integer to string (Itoa)
pub fn itoa(i: i64) -> String {
    i.to_string()
}

/// Parse integer with base
pub fn parse_int(s: &str, base: u32) -> Result<i64, ()> {
    let (actual_base, num_str) = if base == 0 {
        if s.starts_with("0x") || s.starts_with("0X") {
            (16, &s[2..])
        } else if s.starts_with("0o") || s.starts_with("0O") {
            (8, &s[2..])
        } else if s.starts_with("0b") || s.starts_with("0B") {
            (2, &s[2..])
        } else if s.starts_with('0') && s.len() > 1 {
            (8, &s[1..])
        } else {
            (10, s)
        }
    } else {
        (base, s)
    };
    
    i64::from_str_radix(num_str.trim(), actual_base).map_err(|_| ())
}

/// Parse float from string
pub fn parse_float(s: &str) -> Result<f64, ()> {
    s.trim().parse::<f64>().map_err(|_| ())
}

/// Format integer with base
pub fn format_int(i: i64, base: u32) -> String {
    match base {
        2 => format!("{:b}", i),
        8 => format!("{:o}", i),
        16 => format!("{:x}", i),
        _ => i.to_string(),
    }
}

/// Format float with format and precision
pub fn format_float(f: f64, fmt: char, prec: i32) -> String {
    if prec < 0 {
        match fmt {
            'e' | 'E' => format!("{:e}", f),
            'f' | 'F' => format!("{}", f),
            'g' | 'G' => format!("{:?}", f),
            _ => format!("{}", f),
        }
    } else {
        let p = prec as usize;
        match fmt {
            'e' | 'E' => format!("{:.prec$e}", f, prec = p),
            'f' | 'F' => format!("{:.prec$}", f, prec = p),
            'g' | 'G' => format!("{:.prec$?}", f, prec = p),
            _ => format!("{:.prec$}", f, prec = p),
        }
    }
}

/// Quote a string (add quotes and escape special chars)
pub fn quote(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 2);
    result.push('"');
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => {
                result.push_str(&format!("\\x{:02x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result.push('"');
    result
}
