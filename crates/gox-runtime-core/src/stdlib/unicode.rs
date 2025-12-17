//! Unicode character classification and conversion core implementations.
//!
//! Pure logic for unicode package functions.

/// Check if rune is a letter.
pub fn is_letter(r: char) -> bool {
    r.is_alphabetic()
}

/// Check if rune is a digit.
pub fn is_digit(r: char) -> bool {
    r.is_ascii_digit()
}

/// Check if rune is whitespace.
pub fn is_space(r: char) -> bool {
    r.is_whitespace()
}

/// Check if rune is uppercase.
pub fn is_upper(r: char) -> bool {
    r.is_uppercase()
}

/// Check if rune is lowercase.
pub fn is_lower(r: char) -> bool {
    r.is_lowercase()
}

/// Check if rune is printable.
pub fn is_print(r: char) -> bool {
    !r.is_control()
}

/// Check if rune is graphic.
pub fn is_graphic(r: char) -> bool {
    !r.is_control() && !r.is_whitespace()
}

/// Check if rune is a control character.
pub fn is_control(r: char) -> bool {
    r.is_control()
}

/// Check if rune is punctuation.
pub fn is_punct(r: char) -> bool {
    r.is_ascii_punctuation()
}

/// Check if rune is a symbol.
pub fn is_symbol(r: char) -> bool {
    matches!(r, '$' | '+' | '<' | '=' | '>' | '^' | '`' | '|' | '~' | '¢'..='¥' | '©' | '®' | '°' | '±' | '×' | '÷')
}

/// Check if rune is a number.
pub fn is_number(r: char) -> bool {
    r.is_numeric()
}

/// Convert rune to uppercase.
pub fn to_upper(r: char) -> char {
    r.to_uppercase().next().unwrap_or(r)
}

/// Convert rune to lowercase.
pub fn to_lower(r: char) -> char {
    r.to_lowercase().next().unwrap_or(r)
}

/// Convert rune to title case.
pub fn to_title(r: char) -> char {
    r.to_uppercase().next().unwrap_or(r)
}
