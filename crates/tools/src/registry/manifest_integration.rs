// Tool Registry Manifest Validation Integration
// P1.2.2.b: Интегрировать manifest validation в tool loading систему

use crate::manifest::schema::ToolManifest;
use crate::manifest::validation::{
    validate_tool_manifest, validate_tool_manifest_json, ToolManifestValidator, ValidationResult,
};
use crate::{Tool, ToolRegistry, ToolSpec};
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use tracing::{debug, error, info, warn};

/// Error types for manifest-based tool loading
#[derive(Debug, thiserror::Error)]
pub enum ManifestLoadError {
    #[error("Tool manifest validation failed: {0}")]
    ValidationFailed(String),

    #[error("Tool implementation not found: {0}")]
    ImplementationNotFound(String),

    #[error("Tool registration failed: {0}")]
    RegistrationFailed(String),

    #[error("Invalid manifest path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Tool manifest loader with validation
pub struct ManifestToolLoader {
    validator: ToolManifestValidator,
    strict_mode: bool,
    auto_reject_invalid: bool,
}

impl Default for ManifestToolLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ManifestToolLoader {
    /// Create new manifest tool loader with default settings
    pub fn new() -> Self {
        Self {
            validator: ToolManifestValidator::new(),
            strict_mode: false,
            auto_reject_invalid: true,
        }
    }

    /// Enable strict validation mode
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self.validator = self.validator.with_strict_mode(strict);
        self
    }

    /// Configure automatic rejection of invalid tools
    pub fn with_auto_reject(mut self, auto_reject: bool) -> Self {
        self.auto_reject_invalid = auto_reject;
        self
    }

    /// Load and validate a tool from manifest file
    pub fn load_tool_from_manifest<P: AsRef<Path>>(
        &self,
        manifest_path: P,
    ) -> Result<ToolManifest, ManifestLoadError> {
        let path = manifest_path.as_ref();
        debug!("Loading tool from manifest: {}", path.display());

        // Validate the manifest first
        let validation_result = self.validator.validate_file(path);

        // Log validation result
        if validation_result.is_valid {
            info!("Tool manifest validation PASSED: {}", path.display());
            if !validation_result.warnings.is_empty() {
                for warning in &validation_result.warnings {
                    warn!("Manifest warning: {}", warning);
                }
            }
        } else {
            error!("Tool manifest validation FAILED: {}", path.display());
            for error in &validation_result.errors {
                error!("Validation error: {}", error);
            }

            if self.auto_reject_invalid {
                return Err(ManifestLoadError::ValidationFailed(
                    validation_result.report(),
                ));
            }
        }

        // Return the validated manifest
        if let Some(manifest) = validation_result.manifest {
            Ok(manifest)
        } else {
            Err(ManifestLoadError::ValidationFailed(
                "No manifest found in validation result".to_string(),
            ))
        }
    }

    /// Load tool from JSON string
    pub fn load_tool_from_json(&self, json: &str) -> Result<ToolManifest, ManifestLoadError> {
        debug!("Loading tool from JSON string");

        let validation_result = self.validator.validate_json(json);

        if validation_result.is_valid {
            info!("Tool JSON validation PASSED");
            if let Some(manifest) = validation_result.manifest {
                Ok(manifest)
            } else {
                Err(ManifestLoadError::ValidationFailed(
                    "No manifest found in validation result".to_string(),
                ))
            }
        } else {
            error!("Tool JSON validation FAILED");
            if self.auto_reject_invalid {
                Err(ManifestLoadError::ValidationFailed(
                    validation_result.report(),
                ))
            } else {
                // In non-strict mode, try to return the manifest anyway
                if let Some(manifest) = validation_result.manifest {
                    Ok(manifest)
                } else {
                    Err(ManifestLoadError::ValidationFailed(
                        validation_result.report(),
                    ))
                }
            }
        }
    }

    /// Scan directory for tool manifests and return valid ones
    pub fn scan_tool_directory<P: AsRef<Path>>(
        &self,
        dir_path: P,
    ) -> Result<Vec<(PathBuf, ToolManifest)>, ManifestLoadError> {
        let dir = dir_path.as_ref();
        debug!("Scanning directory for tool manifests: {}", dir.display());

        let mut valid_tools = Vec::new();
        let mut rejected_count = 0;

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            // Look for tool.json or *.tool.json files
            if path.is_file()
                && path.extension().is_some_and(|ext| ext == "json")
                && (path.file_name().is_some_and(|name| {
                    name == "tool.json" || name.to_string_lossy().ends_with(".tool.json")
                }))
            {
                match self.load_tool_from_manifest(&path) {
                    Ok(manifest) => {
                        info!(
                            "Successfully loaded tool: {} from {}",
                            manifest.name,
                            path.display()
                        );
                        valid_tools.push((path, manifest));
                    }
                    Err(e) => {
                        if self.auto_reject_invalid {
                            warn!("Rejected invalid tool manifest {}: {}", path.display(), e);
                            rejected_count += 1;
                        } else {
                            error!("Failed to load tool manifest {}: {}", path.display(), e);
                            return Err(e);
                        }
                    }
                }
            }
        }

        info!(
            "Tool directory scan completed: {} valid tools, {} rejected",
            valid_tools.len(),
            rejected_count
        );

        Ok(valid_tools)
    }
}

/// Extension trait for ToolRegistry to add manifest validation support
pub trait ToolRegistryManifestExt {
    /// Register tool from manifest file with validation
    fn register_from_manifest<P: AsRef<Path>>(
        &mut self,
        manifest_path: P,
    ) -> Result<String, ManifestLoadError>;

    /// Register tool from manifest JSON with validation
    fn register_from_manifest_json(&mut self, json: &str) -> Result<String, ManifestLoadError>;

    /// Register multiple tools from directory scan
    fn register_from_directory<P: AsRef<Path>>(
        &mut self,
        dir_path: P,
    ) -> Result<Vec<String>, ManifestLoadError>;

    /// Register tool from validated manifest
    fn register_from_validated_manifest(
        &mut self,
        manifest: ToolManifest,
    ) -> Result<String, ManifestLoadError>;

    /// Check if tool is valid without registering
    fn validate_tool_manifest<P: AsRef<Path>>(&self, manifest_path: P) -> bool;

    /// Get validation report for tool manifest
    fn get_manifest_validation_report<P: AsRef<Path>>(&self, manifest_path: P) -> ValidationResult;
}

impl ToolRegistryManifestExt for ToolRegistry {
    /// Register tool from manifest file with validation
    fn register_from_manifest<P: AsRef<Path>>(
        &mut self,
        manifest_path: P,
    ) -> Result<String, ManifestLoadError> {
        let loader = ManifestToolLoader::new().with_auto_reject(true);
        let manifest = loader.load_tool_from_manifest(manifest_path)?;
        self.register_from_validated_manifest(manifest)
    }

    /// Register tool from manifest JSON with validation
    fn register_from_manifest_json(&mut self, json: &str) -> Result<String, ManifestLoadError> {
        let loader = ManifestToolLoader::new().with_auto_reject(true);
        let manifest = loader.load_tool_from_json(json)?;
        self.register_from_validated_manifest(manifest)
    }

    /// Register multiple tools from directory scan
    fn register_from_directory<P: AsRef<Path>>(
        &mut self,
        dir_path: P,
    ) -> Result<Vec<String>, ManifestLoadError> {
        let loader = ManifestToolLoader::new().with_auto_reject(true);
        let tools = loader.scan_tool_directory(dir_path)?;

        let mut registered_names = Vec::new();

        for (_path, manifest) in tools {
            match self.register_from_validated_manifest(manifest) {
                Ok(name) => registered_names.push(name),
                Err(e) => {
                    error!("Failed to register tool: {}", e);
                    // Continue with other tools instead of failing entirely
                }
            }
        }

        Ok(registered_names)
    }

    /// Register tool from validated manifest
    fn register_from_validated_manifest(
        &mut self,
        manifest: ToolManifest,
    ) -> Result<String, ManifestLoadError> {
        let tool_name = manifest.name.clone();

        debug!("Registering tool from validated manifest: {}", tool_name);

        // Create a basic tool implementation from manifest
        // This is a simple implementation - in a real scenario, you'd need
        // to load the actual tool implementation based on the manifest
        let tool = ManifestBasedTool::new(manifest);

        // Register the tool
        self.register(&tool_name, Box::new(tool));

        info!("Successfully registered tool from manifest: {}", tool_name);
        Ok(tool_name)
    }

    /// Check if tool is valid without registering
    fn validate_tool_manifest<P: AsRef<Path>>(&self, manifest_path: P) -> bool {
        let loader = ManifestToolLoader::new();
        loader.load_tool_from_manifest(manifest_path).is_ok()
    }

    /// Get validation report for tool manifest
    fn get_manifest_validation_report<P: AsRef<Path>>(&self, manifest_path: P) -> ValidationResult {
        let validator = ToolManifestValidator::new();
        validator.validate_file(manifest_path)
    }
}

/// Basic tool implementation based on manifest
/// This is a minimal implementation for demonstration - real tools would need proper implementations
pub struct ManifestBasedTool {
    manifest: ToolManifest,
    spec: ToolSpec,
}

impl ManifestBasedTool {
    pub fn new(manifest: ToolManifest) -> Self {
        // Convert manifest to ToolSpec
        let spec = ToolSpec {
            name: manifest.name.clone(),
            description: manifest.description.clone(),
            usage: format!("Tool: {}", manifest.name),
            examples: vec![format!("{} --help", manifest.name)],
            input_schema: "{}".to_string(), // Basic empty JSON schema
            usage_guide: None,
            permissions: None,
            supports_dry_run: false,
        };

        Self { manifest, spec }
    }
}

#[async_trait::async_trait]
impl Tool for ManifestBasedTool {
    fn spec(&self) -> ToolSpec {
        self.spec.clone()
    }

    async fn execute(&self, _input: crate::ToolInput) -> Result<crate::ToolOutput> {
        // This is a placeholder implementation
        // Real tools would need to execute based on their manifest configuration
        Err(anyhow!(
            "Tool '{}' is not yet implemented - this is a manifest-only registration",
            self.manifest.name
        ))
    }

    async fn parse_natural_language(&self, query: &str) -> Result<crate::ToolInput> {
        // Basic natural language parsing
        Ok(crate::ToolInput {
            command: self.manifest.name.clone(),
            args: std::collections::HashMap::new(),
            context: Some(query.to_string()),
            dry_run: false,
            timeout_ms: self.manifest.effective_timeout().into(),
        })
    }
}

/// Convenience functions for manifest validation integration
pub mod convenience {
    use super::*;

    /// Validate a tool manifest file and return validation status
    pub fn validate_manifest_file<P: AsRef<Path>>(path: P) -> bool {
        validate_tool_manifest(path).is_ok()
    }

    /// Load and register tool from manifest with detailed error reporting
    pub fn load_and_register_tool<P: AsRef<Path>>(
        registry: &mut ToolRegistry,
        manifest_path: P,
    ) -> Result<String> {
        registry
            .register_from_manifest(manifest_path)
            .map_err(|e| anyhow!("Failed to register tool: {}", e))
    }

    /// Batch load tools from directory
    pub fn batch_load_tools<P: AsRef<Path>>(
        registry: &mut ToolRegistry,
        dir_path: P,
    ) -> Result<Vec<String>> {
        registry
            .register_from_directory(dir_path)
            .map_err(|e| anyhow!("Failed to load tools from directory: {}", e))
    }

    /// Get validation report as formatted string
    pub fn get_validation_report<P: AsRef<Path>>(path: P) -> String {
        let validator = ToolManifestValidator::new();
        validator.validate_file(path).report()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::schema::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_manifest_tool_loader() {
        let loader = ManifestToolLoader::new();

        // Test with valid JSON
        let json = r#"
        {
            "name": "test-tool",
            "version": "1.0.0",
            "description": "Test tool",
            "type": "native",
            "capabilities": [],
            "entry_point": "test.exe",
            "metadata": {
                "author": "Test Author",
                "license": "MIT"
            }
        }
        "#;

        let result = loader.load_tool_from_json(json);
        assert!(result.is_ok());
    }

    #[test]
    fn test_manifest_tool_loader_invalid_json() {
        let loader = ManifestToolLoader::new().with_auto_reject(true);

        // Test with invalid JSON (missing required fields)
        let json = r#"
        {
            "name": "",
            "version": "invalid-version"
        }
        "#;

        let result = loader.load_tool_from_json(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_registry_manifest_integration() {
        let mut registry = ToolRegistry::new();

        let json = r#"
        {
            "name": "registry-test-tool",
            "version": "1.0.0",
            "description": "Registry test tool",
            "type": "native",
            "capabilities": [],
            "entry_point": "test.exe",
            "metadata": {
                "author": "Test Author",
                "license": "MIT"
            }
        }
        "#;

        let result = registry.register_from_manifest_json(json);
        assert!(result.is_ok());

        // Verify tool was registered
        let tool_name = result.expect("Operation failed - converted from unwrap()");
        assert!(registry.get(&tool_name).is_some());
    }

    #[test]
    fn test_directory_scan() {
        let dir = tempdir().expect("Operation failed - converted from unwrap()");
        let manifest_path = dir.path().join("test.tool.json");

        let json = r#"
        {
            "name": "scan-test-tool",
            "version": "1.0.0",
            "description": "Scan test tool",
            "type": "native",
            "capabilities": [],
            "entry_point": "scan.exe",
            "metadata": {
                "author": "Scan Author",
                "license": "MIT"
            }
        }
        "#;

        fs::write(&manifest_path, json).expect("Operation failed - converted from unwrap()");

        let mut registry = ToolRegistry::new();
        let result = registry.register_from_directory(dir.path());

        assert!(result.is_ok());
        let registered = result.expect("Operation failed - converted from unwrap()");
        assert_eq!(registered.len(), 1);
        assert_eq!(registered[0], "scan-test-tool");
    }

    #[test]
    fn test_validation_report() {
        let dir = tempdir().expect("Operation failed - converted from unwrap()");
        let manifest_path = dir.path().join("tool.json");

        let json = r#"
        {
            "name": "report-test-tool",
            "version": "1.0.0",
            "description": "Report test tool",
            "type": "native",
            "capabilities": [],
            "entry_point": "report.exe",
            "metadata": {
                "author": "Report Author",
                "license": "MIT"
            }
        }
        "#;

        fs::write(&manifest_path, json).expect("Operation failed - converted from unwrap()");

        let registry = ToolRegistry::new();
        let report = registry.get_manifest_validation_report(&manifest_path);

        assert!(report.is_valid);
        let report_text = report.report();
        assert!(report_text.contains("PASSED"));
    }
}
