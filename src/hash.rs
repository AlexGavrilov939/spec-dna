//! Normalized content hashes (ADR 0001, "Hashes exist from the first release").
//!
//! Normalization: Unicode NFC, runs of whitespace (including line ends)
//! collapsed to one space, leading and trailing whitespace removed. The hash
//! is the first 8 bytes (16 hexadecimal digits) of SHA-256 over the
//! normalized UTF-8 text.

use sha2::{Digest, Sha256};
use std::borrow::Cow;
use unicode_normalization::{IsNormalized, UnicodeNormalization, is_nfc_quick};

pub type Hash = [u8; 8];

/// Returns the text in Unicode NFC, borrowing when it already is.
pub fn nfc(text: &str) -> Cow<'_, str> {
    if text.is_ascii() || matches!(is_nfc_quick(text.chars()), IsNormalized::Yes) {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(text.nfc().collect())
    }
}

/// NFC, whitespace runs collapsed to one space, trimmed.
pub fn normalize(text: &str) -> String {
    let text = nfc(text);
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            space = !out.is_empty();
            continue;
        }
        if space {
            out.push(' ');
            space = false;
        }
        out.push(ch);
    }
    out
}

pub fn hash_normalized(text: &str) -> Hash {
    hash_bytes(normalize(text).as_bytes())
}

pub fn hash_bytes(bytes: &[u8]) -> Hash {
    let digest = Sha256::digest(bytes);
    let mut out = [0u8; 8];
    out.copy_from_slice(&digest[..8]);
    out
}

pub fn hex(hash: &Hash) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(16);
    for b in hash {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 15) as usize] as char);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_ignores_whitespace_layout_and_line_ends() {
        assert_eq!(normalize("  a\r\n  b\tc \n"), "a b c");
        assert_eq!(hash_normalized("a\r\nb"), hash_normalized("a b"));
        assert_ne!(hash_normalized("a b"), hash_normalized("ab"));
    }

    #[test]
    fn normalization_is_nfc() {
        // "é" as e + combining acute (NFD) and as one code point (NFC).
        assert_eq!(
            hash_normalized("calf\u{65}\u{301}"),
            hash_normalized("calf\u{e9}")
        );
    }

    #[test]
    fn hex_is_16_lowercase_digits() {
        let h = hex(&hash_normalized("x"));
        assert_eq!(h.len(), 16);
        assert!(
            h.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
        // SHA-256("x") starts with 2d711642b726b044.
        assert_eq!(h, "2d711642b726b044");
    }
}
