// P1.2.8: Tool Signing for Tools Platform 2.0
// Cryptographic signing and verification of tool packages

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::{debug, error, info, warn};

/// Tool signature information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSignature {
    /// Algorithm used for signing (e.g., "ECDSA_P256_SHA256")
    pub algorithm: String,
    /// Base64-encoded signature bytes
    pub signature: String,
    /// Public key identifier (thumbprint or key ID)
    pub key_id: String,
    /// Timestamp when signature was created
    pub created_at: SystemTime,
    /// Optional additional metadata
    pub metadata: HashMap<String, String>,
}

/// Tool package manifest with signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedToolManifest {
    /// Original tool manifest content
    pub manifest: serde_json::Value,
    /// Signature of the manifest
    pub signature: ToolSignature,
    /// Hash of all files in the tool package
    pub files_hash: String,
    /// List of files included in the signature
    pub signed_files: Vec<String>,
}

/// Signing certificate/key information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigningCertificate {
    /// Certificate identifier
    pub id: String,
    /// Subject name (CN, O, etc.)
    pub subject: String,
    /// Issuer name
    pub issuer: String,
    /// Certificate validity period
    pub valid_from: SystemTime,
    pub valid_until: SystemTime,
    /// Public key (PEM format)
    pub public_key: String,
    /// Certificate chain (if applicable)
    pub certificate_chain: Option<Vec<String>>,
}

/// Tool signing errors
#[derive(Debug, thiserror::Error)]
pub enum SigningError {
    #[error("Key not found: {key_id}")]
    KeyNotFound { key_id: String },

    #[error("Invalid signature: {reason}")]
    InvalidSignature { reason: String },

    #[error("Certificate expired at {expired_at:?}")]
    CertificateExpired { expired_at: SystemTime },

    #[error("Untrusted certificate: {reason}")]
    UntrustedCertificate { reason: String },

    #[error("Cryptographic error: {error}")]
    CryptographicError { error: String },

    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("IO error: {error}")]
    IoError { error: String },

    #[error("Serialization error: {error}")]
    SerializationError { error: String },
}

/// Verification result
#[derive(Debug, Clone)]
pub enum VerificationResult {
    /// Signature is valid and trusted
    Valid { certificate: SigningCertificate },
    /// Signature is cryptographically valid but certificate is untrusted
    ValidButUntrusted { certificate: SigningCertificate },
    /// Signature is invalid
    Invalid { reason: String },
    /// Certificate is expired
    Expired { certificate: SigningCertificate },
}

/// Tool signing and verification engine
pub struct ToolSigner {
    /// Trusted certificate store
    trusted_certificates: HashMap<String, SigningCertificate>,
    /// Cache of verified signatures
    verification_cache: HashMap<String, (VerificationResult, SystemTime)>,
    /// Cache TTL for verified signatures
    cache_ttl: std::time::Duration,
}

impl Default for ToolSigner {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolSigner {
    /// Create new tool signer
    pub fn new() -> Self {
        Self {
            trusted_certificates: HashMap::new(),
            verification_cache: HashMap::new(),
            cache_ttl: std::time::Duration::from_secs(3600), // 1 hour cache
        }
    }

    /// Add trusted certificate to the store
    pub fn add_trusted_certificate(&mut self, certificate: SigningCertificate) {
        info!(
            "Adding trusted certificate: {} ({})",
            certificate.id, certificate.subject
        );
        self.trusted_certificates
            .insert(certificate.id.clone(), certificate);
    }

    /// Remove certificate from trusted store
    pub fn remove_trusted_certificate(&mut self, cert_id: &str) -> Option<SigningCertificate> {
        self.trusted_certificates.remove(cert_id)
    }

    /// Load trusted certificates from directory
    pub async fn load_trusted_certificates<P: AsRef<Path>>(
        &mut self,
        cert_dir: P,
    ) -> Result<usize> {
        let cert_dir = cert_dir.as_ref();
        if !cert_dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        let mut entries = tokio::fs::read_dir(cert_dir)
            .await
            .map_err(|e| anyhow!("Failed to read certificate directory: {}", e))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| anyhow!("Failed to read directory entry: {}", e))?
        {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                match self.load_certificate_from_file(&path).await {
                    Ok(cert) => {
                        self.add_trusted_certificate(cert);
                        count += 1;
                    }
                    Err(e) => {
                        warn!("Failed to load certificate from {}: {}", path.display(), e);
                    }
                }
            }
        }

        info!(
            "Loaded {} trusted certificates from {}",
            count,
            cert_dir.display()
        );
        Ok(count)
    }

    /// Sign a tool manifest and its associated files
    pub async fn sign_tool<P: AsRef<Path>>(
        &self,
        manifest_path: P,
        tool_directory: P,
        signing_key_id: &str,
        private_key: &str,
    ) -> Result<SignedToolManifest, SigningError> {
        let manifest_path = manifest_path.as_ref();
        let tool_directory = tool_directory.as_ref();

        debug!("Signing tool at: {}", manifest_path.display());

        // Read and parse manifest
        let manifest_content = tokio::fs::read_to_string(manifest_path)
            .await
            .map_err(|e| SigningError::FileNotFound {
                path: manifest_path.display().to_string(),
            })?;

        let manifest: serde_json::Value = serde_json::from_str(&manifest_content).map_err(|e| {
            SigningError::SerializationError {
                error: e.to_string(),
            }
        })?;

        // Compute hash of all files in the tool directory
        let (files_hash, signed_files) = self.compute_directory_hash(tool_directory).await?;

        // Create signature data (manifest + files hash)
        let signature_data = format!("{manifest_content}{files_hash}");
        let signature_bytes = signature_data.as_bytes();

        // Compute signature (simplified - in real implementation you'd use proper crypto)
        let signature = self.create_signature(signature_bytes, signing_key_id, private_key)?;

        Ok(SignedToolManifest {
            manifest,
            signature,
            files_hash,
            signed_files,
        })
    }

    /// Verify a signed tool manifest
    pub async fn verify_tool(
        &mut self,
        signed_manifest: &SignedToolManifest,
        tool_directory: Option<&Path>,
    ) -> Result<VerificationResult, SigningError> {
        let cache_key = format!(
            "{}_{}",
            signed_manifest.signature.key_id, signed_manifest.files_hash
        );

        // Check cache first
        if let Some((cached_result, cached_at)) = self.verification_cache.get(&cache_key) {
            if cached_at.elapsed().unwrap_or(self.cache_ttl) < self.cache_ttl {
                debug!("Using cached verification result for {}", cache_key);
                return Ok(cached_result.clone());
            }
        }

        debug!(
            "Verifying tool signature with key ID: {}",
            signed_manifest.signature.key_id
        );

        // Get certificate
        let certificate = self
            .trusted_certificates
            .get(&signed_manifest.signature.key_id)
            .ok_or_else(|| SigningError::KeyNotFound {
                key_id: signed_manifest.signature.key_id.clone(),
            })?;

        // Check certificate validity
        let now = SystemTime::now();
        if now > certificate.valid_until {
            let result = VerificationResult::Expired {
                certificate: certificate.clone(),
            };
            self.verification_cache
                .insert(cache_key, (result.clone(), now));
            return Ok(result);
        }

        // Verify files hash if tool directory is provided
        if let Some(tool_dir) = tool_directory {
            let (computed_hash, _) = self.compute_directory_hash(tool_dir).await?;
            if computed_hash != signed_manifest.files_hash {
                let result = VerificationResult::Invalid {
                    reason: "Files hash mismatch - tool has been tampered with".to_string(),
                };
                self.verification_cache
                    .insert(cache_key, (result.clone(), now));
                return Ok(result);
            }
        }

        // Verify signature
        let manifest_content = serde_json::to_string(&signed_manifest.manifest).map_err(|e| {
            SigningError::SerializationError {
                error: e.to_string(),
            }
        })?;

        let signature_data = format!("{}{}", manifest_content, signed_manifest.files_hash);
        let signature_valid = self.verify_signature(
            signature_data.as_bytes(),
            &signed_manifest.signature,
            &certificate.public_key,
        )?;

        let result = if signature_valid {
            if self.is_certificate_trusted(&certificate.id) {
                VerificationResult::Valid {
                    certificate: certificate.clone(),
                }
            } else {
                VerificationResult::ValidButUntrusted {
                    certificate: certificate.clone(),
                }
            }
        } else {
            VerificationResult::Invalid {
                reason: "Cryptographic signature verification failed".to_string(),
            }
        };

        // Cache result
        self.verification_cache
            .insert(cache_key, (result.clone(), now));

        info!(
            "Tool signature verification completed: {:?}",
            match &result {
                VerificationResult::Valid { .. } => "Valid",
                VerificationResult::ValidButUntrusted { .. } => "Valid but untrusted",
                VerificationResult::Invalid { .. } => "Invalid",
                VerificationResult::Expired { .. } => "Expired",
            }
        );

        Ok(result)
    }

    /// Check if a tool signature is valid and trusted
    pub async fn is_tool_trusted(&mut self, signed_manifest: &SignedToolManifest) -> Result<bool> {
        match self.verify_tool(signed_manifest, None).await? {
            VerificationResult::Valid { .. } => Ok(true),
            _ => Ok(false),
        }
    }

    /// Export signed manifest to file
    pub async fn export_signed_manifest<P: AsRef<Path>>(
        &self,
        signed_manifest: &SignedToolManifest,
        output_path: P,
    ) -> Result<(), SigningError> {
        let json = serde_json::to_string_pretty(signed_manifest).map_err(|e| {
            SigningError::SerializationError {
                error: e.to_string(),
            }
        })?;

        tokio::fs::write(output_path, json)
            .await
            .map_err(|e| SigningError::IoError {
                error: e.to_string(),
            })?;

        Ok(())
    }

    /// Load signed manifest from file
    pub async fn load_signed_manifest<P: AsRef<Path>>(
        path: P,
    ) -> Result<SignedToolManifest, SigningError> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| SigningError::IoError {
                error: e.to_string(),
            })?;

        serde_json::from_str(&content).map_err(|e| SigningError::SerializationError {
            error: e.to_string(),
        })
    }

    /// Compute hash of all files in a directory
    async fn compute_directory_hash<P: AsRef<Path>>(
        &self,
        directory: P,
    ) -> Result<(String, Vec<String>), SigningError> {
        let directory = directory.as_ref();
        let mut hasher = Sha256::new();
        let mut files = Vec::new();

        let mut entries: Vec<PathBuf> = Vec::new();
        let mut stack = vec![directory.to_path_buf()];

        while let Some(current_dir) = stack.pop() {
            let mut dir_entries =
                tokio::fs::read_dir(&current_dir)
                    .await
                    .map_err(|e| SigningError::IoError {
                        error: e.to_string(),
                    })?;

            while let Some(entry) =
                dir_entries
                    .next_entry()
                    .await
                    .map_err(|e| SigningError::IoError {
                        error: e.to_string(),
                    })?
            {
                let path = entry.path();
                let metadata = entry.metadata().await.map_err(|e| SigningError::IoError {
                    error: e.to_string(),
                })?;

                if metadata.is_dir() {
                    stack.push(path);
                } else {
                    entries.push(path);
                }
            }
        }

        // Sort entries for consistent hashing
        entries.sort();

        for path in entries {
            // Get relative path from directory
            let relative_path =
                path.strip_prefix(directory)
                    .map_err(|e| SigningError::IoError {
                        error: e.to_string(),
                    })?;

            // Read file content
            let content = tokio::fs::read(&path)
                .await
                .map_err(|e| SigningError::FileNotFound {
                    path: path.display().to_string(),
                })?;

            // Update hasher with path and content
            hasher.update(relative_path.to_string_lossy().as_bytes());
            hasher.update(&content);

            files.push(relative_path.display().to_string());
        }

        let hash = format!("{:x}", hasher.finalize());
        Ok((hash, files))
    }

    /// Create a signature (simplified implementation)
    fn create_signature(
        &self,
        data: &[u8],
        key_id: &str,
        _private_key: &str,
    ) -> Result<ToolSignature, SigningError> {
        // In a real implementation, you would:
        // 1. Parse the private key
        // 2. Use proper cryptographic library (e.g., ring, openssl)
        // 3. Create actual ECDSA or RSA signature

        // For this demo, we'll create a simple hash-based signature
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.update(key_id.as_bytes());
        let signature_hash = hasher.finalize();

        use base64::Engine;
        Ok(ToolSignature {
            algorithm: "DEMO_SHA256".to_string(),
            signature: base64::engine::general_purpose::STANDARD.encode(signature_hash),
            key_id: key_id.to_string(),
            created_at: SystemTime::now(),
            metadata: HashMap::new(),
        })
    }

    /// Verify a signature (simplified implementation)
    fn verify_signature(
        &self,
        data: &[u8],
        signature: &ToolSignature,
        _public_key: &str,
    ) -> Result<bool, SigningError> {
        // In a real implementation, you would:
        // 1. Parse the public key
        // 2. Use proper cryptographic verification
        // 3. Check signature against actual cryptographic algorithms

        // For this demo, recreate the signature and compare
        if signature.algorithm != "DEMO_SHA256" {
            return Ok(false);
        }

        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.update(signature.key_id.as_bytes());
        let expected_hash = hasher.finalize();
        use base64::Engine;
        let expected_signature = base64::engine::general_purpose::STANDARD.encode(expected_hash);

        Ok(signature.signature == expected_signature)
    }

    /// Load certificate from JSON file
    async fn load_certificate_from_file<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<SigningCertificate> {
        let content = tokio::fs::read_to_string(path).await?;
        let certificate: SigningCertificate = serde_json::from_str(&content)?;
        Ok(certificate)
    }

    /// Check if certificate is in trusted store
    fn is_certificate_trusted(&self, cert_id: &str) -> bool {
        self.trusted_certificates.contains_key(cert_id)
    }

    /// Clear verification cache
    pub fn clear_cache(&mut self) {
        self.verification_cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        let total_entries = self.verification_cache.len();
        let expired_entries = self
            .verification_cache
            .values()
            .filter(|(_, cached_at)| {
                cached_at.elapsed().unwrap_or(self.cache_ttl) >= self.cache_ttl
            })
            .count();

        (total_entries, expired_entries)
    }
}

// Add base64 to Cargo.toml dependencies
use base64;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_certificate_management() {
        let mut signer = ToolSigner::new();

        let cert = SigningCertificate {
            id: "test-cert-1".to_string(),
            subject: "CN=Test Certificate".to_string(),
            issuer: "CN=Test CA".to_string(),
            valid_from: SystemTime::now(),
            valid_until: SystemTime::now() + std::time::Duration::from_secs(86400),
            public_key: "-----BEGIN PUBLIC KEY-----\nTEST\n-----END PUBLIC KEY-----".to_string(),
            certificate_chain: None,
        };

        signer.add_trusted_certificate(cert.clone());

        assert!(signer.is_certificate_trusted("test-cert-1"));
        assert!(!signer.is_certificate_trusted("non-existent"));

        let removed = signer.remove_trusted_certificate("test-cert-1");
        assert!(removed.is_some());
        assert!(!signer.is_certificate_trusted("test-cert-1"));
    }

    #[tokio::test]
    async fn test_directory_hashing() {
        let temp_dir = tempdir().unwrap();
        let signer = ToolSigner::new();

        // Create test files
        tokio::fs::write(temp_dir.path().join("file1.txt"), b"content1")
            .await
            .unwrap();
        tokio::fs::write(temp_dir.path().join("file2.txt"), b"content2")
            .await
            .unwrap();

        let (hash1, files1) = signer
            .compute_directory_hash(temp_dir.path())
            .await
            .unwrap();
        assert_eq!(files1.len(), 2);
        assert!(files1.contains(&"file1.txt".to_string()));
        assert!(files1.contains(&"file2.txt".to_string()));

        // Create same files in different order - should produce same hash
        let temp_dir2 = tempdir().unwrap();
        tokio::fs::write(temp_dir2.path().join("file2.txt"), b"content2")
            .await
            .unwrap();
        tokio::fs::write(temp_dir2.path().join("file1.txt"), b"content1")
            .await
            .unwrap();

        let (hash2, _files2) = signer
            .compute_directory_hash(temp_dir2.path())
            .await
            .unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_signature_creation_and_verification() {
        let signer = ToolSigner::new();
        let data = b"test data to sign";
        let key_id = "test-key";

        let signature = signer
            .create_signature(data, key_id, "dummy-private-key")
            .unwrap();
        assert_eq!(signature.key_id, key_id);
        assert_eq!(signature.algorithm, "DEMO_SHA256");

        let is_valid = signer
            .verify_signature(data, &signature, "dummy-public-key")
            .unwrap();
        assert!(is_valid);

        // Test with different data
        let different_data = b"different data";
        let is_valid_different = signer
            .verify_signature(different_data, &signature, "dummy-public-key")
            .unwrap();
        assert!(!is_valid_different);
    }

    #[test]
    fn test_cache_functionality() {
        let mut signer = ToolSigner::new();
        let (total, expired) = signer.cache_stats();
        assert_eq!(total, 0);
        assert_eq!(expired, 0);

        signer.clear_cache();
        assert_eq!(signer.verification_cache.len(), 0);
    }
}
