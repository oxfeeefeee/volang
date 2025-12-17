//! Hex encoding/decoding core implementations.
//!
//! Pure logic for encoding/hex package functions.

use alloc::string::String;
use alloc::vec::Vec;

const HEX_CHARS: &[u8] = b"0123456789abcdef";

/// Encode bytes to hex string.
pub fn encode_to_string(src: &[u8]) -> String {
    let mut result = String::with_capacity(src.len() * 2);
    for &b in src {
        result.push(HEX_CHARS[(b >> 4) as usize] as char);
        result.push(HEX_CHARS[(b & 0x0f) as usize] as char);
    }
    result
}

/// Decode hex string to bytes.
pub fn decode_string(s: &str) -> Vec<u8> {
    let bytes: Vec<u8> = s.bytes().filter_map(decode_hex_char).collect();
    let mut result = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks(2) {
        if chunk.len() == 2 {
            result.push((chunk[0] << 4) | chunk[1]);
        }
    }
    result
}

/// Get encoded length for n bytes.
#[inline]
pub fn encoded_len(n: i64) -> i64 {
    n * 2
}

/// Get decoded length for x hex chars.
#[inline]
pub fn decoded_len(x: i64) -> i64 {
    x / 2
}

fn decode_hex_char(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_encode() {
        assert_eq!(encode_to_string(&[0x48, 0x65, 0x6c, 0x6c, 0x6f]), "48656c6c6f");
    }
    
    #[test]
    fn test_decode() {
        assert_eq!(decode_string("48656c6c6f"), vec![0x48, 0x65, 0x6c, 0x6c, 0x6f]);
    }
}
