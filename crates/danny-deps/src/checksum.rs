//! Checksum verification for lockfiles

use crate::{Error, Result};
use sha2::{Digest, Sha256, Sha512};
use std::fmt;

/// Checksum algorithm type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChecksumAlgorithm {
    /// SHA-256 (used by Cargo.lock)
    Sha256,
    /// SHA-512 (used by npm package-lock.json)
    Sha512,
}

/// Checksum verifier
pub struct ChecksumVerifier {
    algorithm: ChecksumAlgorithm,
}

impl ChecksumVerifier {
    /// Create a new checksum verifier with the specified algorithm
    pub fn new(algorithm: ChecksumAlgorithm) -> Self {
        Self { algorithm }
    }

    /// Compute checksum of data
    pub fn compute(&self, data: &[u8]) -> String {
        match self.algorithm {
            ChecksumAlgorithm::Sha256 => {
                let mut hasher = Sha256::new();
                hasher.update(data);
                hex::encode(hasher.finalize())
            }
            ChecksumAlgorithm::Sha512 => {
                let mut hasher = Sha512::new();
                hasher.update(data);
                // npm uses base64 encoding for SHA-512
                use base64::{Engine as _, engine::general_purpose};
                format!("sha512-{}", general_purpose::STANDARD.encode(hasher.finalize()))
            }
        }
    }

    /// Verify that computed checksum matches expected
    ///
    /// Uses constant-time comparison to prevent timing attacks.
    pub fn verify(&self, data: &[u8], expected: &str) -> Result<()> {
        let computed = self.compute(data);

        // Constant-time comparison
        if computed.len() != expected.len() {
            return Err(Error::ChecksumMismatch(
                "data".to_string(),
                expected.to_string(),
                computed,
            ));
        }

        let mut diff = 0u8;
        for (a, b) in computed.bytes().zip(expected.bytes()) {
            diff |= a ^ b;
        }

        if diff == 0 {
            Ok(())
        } else {
            Err(Error::ChecksumMismatch(
                "data".to_string(),
                expected.to_string(),
                computed,
            ))
        }
    }

    /// Verify checksum from a string format (handles "sha512-..." prefix)
    pub fn verify_from_string(&self, data: &[u8], expected: &str) -> Result<()> {
        // Handle npm-style "sha512-..." prefix
        let expected_clean = expected.strip_prefix("sha512-").unwrap_or(expected);
        
        // For SHA-512, we need to compare base64-encoded values
        if matches!(self.algorithm, ChecksumAlgorithm::Sha512) {
            let computed = self.compute(data);
            let computed_clean = computed.strip_prefix("sha512-").unwrap_or(&computed);
            
            if computed_clean == expected_clean {
                Ok(())
            } else {
                Err(Error::ChecksumMismatch(
                    "data".to_string(),
                    expected.to_string(),
                    computed,
                ))
            }
        } else {
            self.verify(data, expected_clean)
        }
    }
}

impl fmt::Debug for ChecksumVerifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ChecksumVerifier")
            .field("algorithm", &self.algorithm)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_compute() {
        let verifier = ChecksumVerifier::new(ChecksumAlgorithm::Sha256);
        let data = b"hello world";
        let checksum = verifier.compute(data);
        assert_eq!(checksum.len(), 64); // SHA-256 produces 64 hex chars
    }

    #[test]
    fn test_sha512_compute() {
        let verifier = ChecksumVerifier::new(ChecksumAlgorithm::Sha512);
        let data = b"hello world";
        let checksum = verifier.compute(data);
        assert!(checksum.starts_with("sha512-"));
    }

    #[test]
    fn test_sha256_verify() {
        let verifier = ChecksumVerifier::new(ChecksumAlgorithm::Sha256);
        let data = b"hello world";
        let checksum = verifier.compute(data);
        assert!(verifier.verify(data, &checksum).is_ok());
    }

    #[test]
    fn test_sha256_verify_fail() {
        let verifier = ChecksumVerifier::new(ChecksumAlgorithm::Sha256);
        let data = b"hello world";
        assert!(verifier.verify(data, "wrong_checksum").is_err());
    }

    #[test]
    fn test_sha512_verify_from_string() {
        let verifier = ChecksumVerifier::new(ChecksumAlgorithm::Sha512);
        let data = b"hello world";
        let checksum = verifier.compute(data);
        // Should handle both with and without prefix
        assert!(verifier.verify_from_string(data, &checksum).is_ok());
        let without_prefix = checksum.strip_prefix("sha512-").unwrap();
        assert!(verifier.verify_from_string(data, without_prefix).is_ok());
    }
}

