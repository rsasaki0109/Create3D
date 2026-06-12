use std::fmt;

use serde::{Deserialize, Serialize};

/// Immutable content hash for asset blobs.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    /// Hash raw bytes with BLAKE3.
    pub fn of_bytes(bytes: &[u8]) -> Self {
        Self(*blake3::hash(bytes).as_bytes())
    }

    /// Returns the canonical lowercase hex representation.
    pub fn to_hex(self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    /// Parse a lowercase hex hash.
    pub fn from_hex(value: &str) -> Option<Self> {
        if value.len() != 64 || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
            return None;
        }
        let mut bytes = [0u8; 32];
        for (index, chunk) in value.as_bytes().chunks(2).enumerate() {
            let hex = std::str::from_utf8(chunk).ok()?;
            bytes[index] = u8::from_str_radix(hex, 16).ok()?;
        }
        Some(Self(bytes))
    }

    /// Returns raw hash bytes.
    pub fn as_bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Debug for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ContentHash({})", self.to_hex())
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_round_trip() {
        let hash = ContentHash::of_bytes(b"hello");
        let parsed = ContentHash::from_hex(&hash.to_hex()).expect("parse hash");
        assert_eq!(hash, parsed);
    }
}
