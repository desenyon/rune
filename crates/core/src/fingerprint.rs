use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

const HEX_LEN: usize = 64;

/// Blake3 hash of canonical bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentHash([u8; 32]);

impl ContentHash {
    pub fn hash(bytes: impl AsRef<[u8]>) -> Self {
        let hash = blake3::hash(bytes.as_ref());
        Self(*hash.as_bytes())
    }

    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub fn to_hex(self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(value: &str) -> Result<Self, FingerprintError> {
        if value.len() != HEX_LEN {
            return Err(FingerprintError::InvalidLength(value.len()));
        }
        let bytes = hex::decode(value).map_err(FingerprintError::InvalidHex)?;
        let mut out = [0u8; 32];
        out.copy_from_slice(&bytes);
        Ok(Self(out))
    }
}

impl Display for ContentHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// Composite fingerprint used to invalidate derived artifacts.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Fingerprint {
    pub algorithm: String,
    pub hash: ContentHash,
    pub inputs: Vec<ContentHash>,
}

impl Fingerprint {
    pub fn of(label: &str, parts: &[&[u8]]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(label.as_bytes());
        let mut inputs = Vec::with_capacity(parts.len());
        for part in parts {
            let hash = ContentHash::hash(part);
            hasher.update(hash.as_bytes());
            inputs.push(hash);
        }
        Fingerprint {
            algorithm: "blake3".to_string(),
            hash: ContentHash::from_bytes(*hasher.finalize().as_bytes()),
            inputs,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FingerprintError {
    #[error("content hash hex must be {HEX_LEN} characters, got {0}")]
    InvalidLength(usize),
    #[error("invalid hex: {0}")]
    InvalidHex(#[from] hex::FromHexError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable() {
        let a = ContentHash::hash(b"rune");
        let b = ContentHash::hash(b"rune");
        assert_eq!(a, b);
        assert_eq!(a, ContentHash::from_hex(&a.to_hex()).unwrap());
    }

    #[test]
    fn fingerprint_changes_when_input_changes() {
        let a = Fingerprint::of("syntax", &[b"fn main() {}"]);
        let b = Fingerprint::of("syntax", &[b"fn main() { }"]);
        assert_ne!(a.hash, b.hash);
    }
}
