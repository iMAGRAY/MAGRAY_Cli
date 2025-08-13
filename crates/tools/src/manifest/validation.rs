// Tool Manifest Validation Engine
// P1.2.2.a: JSON Schema Validation and Error Reporting

use super::schema::{ToolCapability, ToolManifest, ToolType};
use anyhow::{anyhow, Result};
use serde_json;
use std::path::Path;
use tracing::{debug, error, warn};

/// Validation error types
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("JSON parsing error: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("IO error reading manifest: {0}")]
    Io(#[from] std::io::Error),

    #[error("Schema validation error: {0}")]
    Schema(String),

    #[error("Semantic validation error: {0}")]
    Semantic(String),

    #[error("Security validation error: {0}")]
    Security(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid tool type '{0}' for entry point '{1}'")]
    InvalidToolType(String, String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Invalid version format: {0}")]
    InvalidVersion(String),
}

/// Validation result with detailed error information
#[derive(Debug)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<String>,
    pub manifest: Option<ToolManifest>,
}

impl ValidationResult {
    pub fn valid(manifest: ToolManifest) -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            manifest: Some(manifest),
        }
    }

    pub fn invalid(errors: Vec<ValidationError>) -> Self {
        Self {
            is_valid: false,
            errors,
            warnings: Vec::new(),
            manifest: None,
        }
    }

    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings = warnings;
        self
    }

    /// Check if there are only warnings (recoverable issues)
    pub fn has_warnings_only(&self) -> bool {
        self.is_valid && !self.warnings.is_empty()
    }

    /// Get all error messages
    pub fn error_messages(&self) -> Vec<String> {
        self.errors.iter().map(|e| e.to_string()).collect()
    }

    /// Get formatted validation report
    pub fn report(&self) -> String {
        let mut report = String::new();

        if self.is_valid {
            report.push_str("✓ Manifest validation: PASSED\n");
        } else {
            report.push_str("✗ Manifest validation: FAILED\n");
        }

        if !self.errors.is_empty() {
            report.push_str("\nErrors:\n");
            for (i, error) in self.errors.iter().enumerate() {
                report.push_str(&format!("  {}. {}\n", i + 1, error));
            }
        }

        if !self.warnings.is_empty() {
            report.push_str("\nWarnings:\n");
            for (i, warning) in self.warnings.iter().enumerate() {
                report.push_str(&format!("  {}. {}\n", i + 1, warning));
            }
        }

        report
    }
}

/// Tool manifest validator
pub struct ToolManifestValidator {
    strict_mode: bool,
    check_file_existence: bool,
}

impl Default for ToolManifestValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolManifestValidator {
    /// Create new validator with default settings
    pub fn new() -> Self {
        Self {
            strict_mode: false,
            check_file_existence: true,
        }
    }

    /// Enable strict validation mode
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    /// Enable/disable file existence checking
    pub fn with_file_existence_check(mut self, check: bool) -> Self {
        self.check_file_existence = check;
        self
    }

    /// Validate tool manifest from JSON string
    pub fn validate_json(&self, json: &str) -> ValidationResult {
        debug!("Validating tool manifest JSON");

        // Step 1: Parse JSON
        let manifest: ToolManifest = match serde_json::from_str(json) {
            Ok(manifest) => manifest,
            Err(e) => {
                error!("JSON parsing failed: {}", e);
                return ValidationResult::invalid(vec![ValidationError::JsonParse(e)]);
            }
        };

        // Step 2: Validate parsed manifest
        self.validate_manifest(manifest)
    }

    /// Validate tool manifest from file
    pub fn validate_file<P: AsRef<Path>>(&self, path: P) -> ValidationResult {
        let path = path.as_ref();
        debug!("Validating tool manifest file: {}", path.display());

        // Read file
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                error!("Failed to read manifest file: {}", e);
                return ValidationResult::invalid(vec![ValidationError::Io(e)]);
            }
        };

        // Validate with file context
        let mut result = self.validate_json(&content);

        // Check entry point file existence if enabled
        if self.check_file_existence && result.is_valid {
            if let Some(ref manifest) = result.manifest {
                let entry_path = path
                    .parent()
                    .unwrap_or(Path::new("."))
                    .join(&manifest.entry_point);
                if !entry_path.exists() {
                    result.warnings.push(format!(
                        "Entry point file not found: {}",
                        entry_path.display()
                    ));
                }
            }
        }

        result
    }

    /// Validate parsed tool manifest
    pub fn validate_manifest(&self, manifest: ToolManifest) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        debug!("Validating manifest for tool: {}", manifest.name);

        // Step 1: Schema validation
        if let Err(e) = self.validate_schema(&manifest) {
            errors.extend(e);
        }

        // Step 2: Semantic validation
        if let Err(e) = self.validate_semantics(&manifest) {
            errors.extend(e);
        }

        // Step 3: Security validation
        if let Err(e) = self.validate_security(&manifest) {
            errors.extend(e);
        }

        // Step 4: Consistency validation
        if let Err(msg) = manifest.validate_consistency() {
            errors.push(ValidationError::Semantic(msg));
        }

        // Step 5: Additional warnings
        warnings.extend(self.generate_warnings(&manifest));

        if errors.is_empty() {
            ValidationResult::valid(manifest).with_warnings(warnings)
        } else {
            ValidationResult::invalid(errors).with_warnings(warnings)
        }
    }

    /// Validate JSON schema compliance
    fn validate_schema(&self, manifest: &ToolManifest) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Required fields validation
        if manifest.name.is_empty() {
            errors.push(ValidationError::MissingField("name".to_string()));
        }

        if manifest.version.is_empty() {
            errors.push(ValidationError::MissingField("version".to_string()));
        }

        if manifest.description.is_empty() {
            errors.push(ValidationError::MissingField("description".to_string()));
        }

        if manifest.entry_point.is_empty() {
            errors.push(ValidationError::MissingField("entry_point".to_string()));
        }

        if manifest.metadata.author.is_empty() {
            errors.push(ValidationError::MissingField("metadata.author".to_string()));
        }

        if manifest.metadata.license.is_empty() {
            errors.push(ValidationError::MissingField(
                "metadata.license".to_string(),
            ));
        }

        // Version format validation (basic semver check)
        if !self.is_valid_version(&manifest.version) {
            errors.push(ValidationError::InvalidVersion(manifest.version.clone()));
        }

        // Name validation (alphanumeric + hyphens/underscores)
        if !self.is_valid_name(&manifest.name) {
            errors.push(ValidationError::Schema(
                "Tool name must contain only alphanumeric characters, hyphens, and underscores"
                    .to_string(),
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate semantic consistency
    fn validate_semantics(&self, manifest: &ToolManifest) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Tool type vs entry point validation
        match manifest.tool_type {
            ToolType::Wasm => {
                if !manifest.entry_point.ends_with(".wasm") {
                    errors.push(ValidationError::InvalidToolType(
                        "wasm".to_string(),
                        manifest.entry_point.clone(),
                    ));
                }
            }
            ToolType::Native => {
                // Native tools can have any extension or no extension
                if manifest.entry_point.ends_with(".wasm") {
                    errors.push(ValidationError::InvalidToolType(
                        "native".to_string(),
                        manifest.entry_point.clone(),
                    ));
                }
            }
            ToolType::Script => {
                // Scripts should have common script extensions
                let valid_extensions = [".py", ".js", ".sh", ".ps1", ".rb"];
                if !valid_extensions
                    .iter()
                    .any(|ext| manifest.entry_point.ends_with(ext))
                    && self.strict_mode
                {
                    errors.push(ValidationError::InvalidToolType(
                        "script".to_string(),
                        manifest.entry_point.clone(),
                    ));
                }
            }
        }

        // Capability validation
        if manifest.capabilities.is_empty() && self.strict_mode {
            errors.push(ValidationError::Semantic(
                "Tool must declare at least one capability in strict mode".to_string(),
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate security constraints
    fn validate_security(&self, manifest: &ToolManifest) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Check for dangerous capability combinations
        if manifest.has_capability(&ToolCapability::Shell)
            && manifest.has_capability(&ToolCapability::Network)
            && manifest.has_capability(&ToolCapability::Filesystem)
            && self.strict_mode
        {
            errors.push(ValidationError::Security(
                "Tool requests all three high-risk capabilities (shell, network, filesystem)"
                    .to_string(),
            ));
        }

        // Validate resource limits are within security bounds
        let memory_limit = manifest.effective_memory_limit();
        if memory_limit > 512 && self.strict_mode {
            errors.push(ValidationError::Security(
                "Memory limit exceeds security threshold (512MB) in strict mode".to_string(),
            ));
        }

        let timeout = manifest.effective_timeout();
        if timeout > 120_000 && self.strict_mode {
            // 2 minutes
            errors.push(ValidationError::Security(
                "Execution timeout exceeds security threshold (120s) in strict mode".to_string(),
            ));
        }

        // Entry point security checks
        if manifest.entry_point.contains("..") || manifest.entry_point.starts_with('/') {
            errors.push(ValidationError::Security(
                "Entry point contains potentially dangerous path components".to_string(),
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Generate warnings for potential issues
    fn generate_warnings(&self, manifest: &ToolManifest) -> Vec<String> {
        let mut warnings = Vec::new();

        // Repository/documentation warnings
        if manifest.metadata.repository.is_none() {
            warnings.push("No repository URL specified in metadata".to_string());
        }

        if manifest.metadata.documentation.is_none() {
            warnings.push("No documentation URL specified in metadata".to_string());
        }

        // Resource usage warnings
        if manifest.effective_memory_limit() > 256 {
            warnings.push("High memory limit (>256MB) - consider optimization".to_string());
        }

        if manifest.effective_timeout() > 60_000 {
            warnings.push("Long execution timeout (>60s) - may impact user experience".to_string());
        }

        // Capability warnings
        if manifest.has_capability(&ToolCapability::Shell) {
            warnings.push("Tool requests shell access - ensure proper sandboxing".to_string());
        }

        if manifest.capabilities.len() > 3 {
            warnings.push(
                "Tool requests many capabilities - consider principle of least privilege"
                    .to_string(),
            );
        }

        warnings
    }

    /// Validate semantic version format (basic check)
    fn is_valid_version(&self, version: &str) -> bool {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return false;
        }

        parts.iter().all(|part| {
            part.parse::<u32>().is_ok() && !part.is_empty() && !part.starts_with('0')
                || *part == "0"
        })
    }

    /// Validate tool name format
    fn is_valid_name(&self, name: &str) -> bool {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            && !name.starts_with('-')
            && !name.ends_with('-')
    }
}

/// Convenience function to validate manifest file
pub fn validate_tool_manifest<P: AsRef<Path>>(path: P) -> Result<ToolManifest> {
    let validator = ToolManifestValidator::new();
    let result = validator.validate_file(path);

    if result.is_valid {
        Ok(result
            .manifest
            .expect("Operation failed - converted from unwrap()"))
    } else {
        Err(anyhow!("Manifest validation failed:\n{}", result.report()))
    }
}

/// Convenience function to validate manifest JSON
pub fn validate_tool_manifest_json(json: &str) -> Result<ToolManifest> {
    let validator = ToolManifestValidator::new();
    let result = validator.validate_json(json);

    if result.is_valid {
        Ok(result
            .manifest
            .expect("Operation failed - converted from unwrap()"))
    } else {
        Err(anyhow!("Manifest validation failed:\n{}", result.report()))
    }
}

/// Check if a tool manifest is valid without detailed validation
pub fn is_valid_tool_manifest<P: AsRef<Path>>(path: P) -> bool {
    let validator = ToolManifestValidator::new();
    validator.validate_file(path).is_valid
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::schema::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_valid_manifest_validation() {
        let manifest = ToolManifest::new(
            "test-tool".to_string(),
            "1.0.0".to_string(),
            "A test tool".to_string(),
            ToolType::Wasm,
            "main.wasm".to_string(),
            "Test Author".to_string(),
            "MIT".to_string(),
        );

        let validator = ToolManifestValidator::new();
        let result = validator.validate_manifest(manifest);

        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_invalid_manifest_validation() {
        let manifest = ToolManifest::new(
            "".to_string(),                // Invalid empty name
            "invalid-version".to_string(), // Invalid version
            "Description".to_string(),
            ToolType::Wasm,
            "main.exe".to_string(), // Wrong extension for WASM
            "Author".to_string(),
            "MIT".to_string(),
        );

        let validator = ToolManifestValidator::new();
        let result = validator.validate_manifest(manifest);

        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_json_validation() {
        let json = r#"
        {
            "name": "example-tool",
            "version": "1.2.3",
            "description": "Example tool",
            "type": "wasm",
            "capabilities": ["filesystem"],
            "entry_point": "example.wasm",
            "runtime_config": {
                "max_memory_mb": 64,
                "max_execution_time_ms": 30000,
                "fuel_limit": 1000000
            },
            "permissions": {
                "filesystem": ["read", "write"]
            },
            "metadata": {
                "author": "Test Author",
                "license": "MIT"
            }
        }
        "#;

        let validator = ToolManifestValidator::new();
        let result = validator.validate_json(json);

        assert!(result.is_valid);
        assert!(result.manifest.is_some());
    }

    #[test]
    fn test_file_validation() {
        let dir = tempdir().expect("Operation failed - converted from unwrap()");
        let manifest_path = dir.path().join("tool.json");

        let json = r#"
        {
            "name": "file-tool",
            "version": "1.0.0", 
            "description": "File-based tool",
            "type": "native",
            "capabilities": [],
            "entry_point": "tool.exe",
            "metadata": {
                "author": "File Author",
                "license": "Apache-2.0"
            }
        }
        "#;

        fs::write(&manifest_path, json).expect("Operation failed - converted from unwrap()");

        let validator = ToolManifestValidator::new().with_file_existence_check(false);
        let result = validator.validate_file(&manifest_path);

        assert!(result.is_valid);
    }

    #[test]
    fn test_strict_mode_validation() {
        let manifest = ToolManifest::new(
            "security-risk-tool".to_string(),
            "1.0.0".to_string(),
            "High-risk tool".to_string(),
            ToolType::Native,
            "tool".to_string(),
            "Author".to_string(),
            "MIT".to_string(),
        )
        .with_capability(ToolCapability::Shell)
        .with_capability(ToolCapability::Network)
        .with_capability(ToolCapability::Filesystem)
        .with_runtime_config(RuntimeConfig {
            max_memory_mb: Some(1024),            // High memory
            max_execution_time_ms: Some(180_000), // 3 minutes
            fuel_limit: Some(1_000_000),
        });

        let validator = ToolManifestValidator::new().with_strict_mode(true);
        let result = validator.validate_manifest(manifest);

        assert!(!result.is_valid);
        assert!(result
            .errors
            .iter()
            .any(|e| matches!(e, ValidationError::Security(_))));
    }
}
