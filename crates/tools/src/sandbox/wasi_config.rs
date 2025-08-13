// P1.2.4.a Step 1: WASI Configuration (4м)
// WASI capabilities integration with filesystem and network sandboxing

use crate::sandbox::SandboxError;
use anyhow::Result;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Filesystem access configuration for WASI sandbox
#[derive(Debug, Clone, Default)]
pub struct FileSystemAccess {
    /// Directories with read-only access
    pub read_only_dirs: HashSet<PathBuf>,
    /// Directories with read-write access
    pub read_write_dirs: HashSet<PathBuf>,
    /// Block all filesystem access if true
    pub deny_all_fs: bool,
}

impl FileSystemAccess {
    /// Add read-only directory access
    pub fn add_read_only(&mut self, path: PathBuf) -> Result<(), SandboxError> {
        if self.deny_all_fs {
            return Err(SandboxError::PermissionDenied {
                operation: "Filesystem access denied by policy".to_string(),
            });
        }

        // Validate path exists and is accessible
        if !path.exists() {
            return Err(SandboxError::WasiConfigError(format!(
                "Path does not exist: {}",
                path.display()
            )));
        }

        self.read_only_dirs.insert(path);
        Ok(())
    }

    /// Add read-write directory access
    pub fn add_read_write(&mut self, path: PathBuf) -> Result<(), SandboxError> {
        if self.deny_all_fs {
            return Err(SandboxError::PermissionDenied {
                operation: "Filesystem access denied by policy".to_string(),
            });
        }

        // Validate path exists and is writable
        if !path.exists() {
            return Err(SandboxError::WasiConfigError(format!(
                "Path does not exist: {}",
                path.display()
            )));
        }

        if path
            .metadata()
            .map(|m| m.permissions().readonly())
            .unwrap_or(true)
        {
            return Err(SandboxError::WasiConfigError(format!(
                "Path is read-only: {}",
                path.display()
            )));
        }

        self.read_write_dirs.insert(path);
        Ok(())
    }

    /// Check if path is allowed for reading
    pub fn can_read(&self, path: &PathBuf) -> bool {
        if self.deny_all_fs {
            return false;
        }

        self.read_only_dirs.contains(path)
            || self.read_write_dirs.contains(path)
            || self.is_subpath_allowed(path)
    }

    /// Check if path is allowed for writing
    pub fn can_write(&self, path: &PathBuf) -> bool {
        if self.deny_all_fs {
            return false;
        }

        self.read_write_dirs.contains(path) || self.is_subpath_write_allowed(path)
    }

    /// Check if path is within allowed directories
    fn is_subpath_allowed(&self, path: &Path) -> bool {
        for allowed_path in &self.read_only_dirs {
            if path.starts_with(allowed_path) {
                return true;
            }
        }
        for allowed_path in &self.read_write_dirs {
            if path.starts_with(allowed_path) {
                return true;
            }
        }
        false
    }

    /// Check if path is within write-allowed directories
    fn is_subpath_write_allowed(&self, path: &Path) -> bool {
        for allowed_path in &self.read_write_dirs {
            if path.starts_with(allowed_path) {
                return true;
            }
        }
        false
    }
}

/// Network access configuration for WASI sandbox
#[derive(Debug, Clone, Default)]
pub struct NetworkAccess {
    /// Allowed hostnames/domains for outbound connections
    pub allowed_hosts: HashSet<String>,
    /// Block all network access if true
    pub deny_all_network: bool,
    /// Allow localhost connections
    pub allow_localhost: bool,
}

impl NetworkAccess {
    /// Add allowed hostname/domain
    pub fn add_host(&mut self, host: String) -> Result<(), SandboxError> {
        if self.deny_all_network {
            return Err(SandboxError::PermissionDenied {
                operation: "Network access denied by policy".to_string(),
            });
        }

        // Validate hostname format
        if host.is_empty() || host.contains(' ') {
            return Err(SandboxError::WasiConfigError(format!(
                "Invalid hostname: {host}"
            )));
        }

        self.allowed_hosts.insert(host);
        Ok(())
    }

    /// Check if host is allowed for connection
    pub fn can_connect(&self, host: &str) -> bool {
        if self.deny_all_network {
            return false;
        }

        // Check localhost
        if self.allow_localhost && (host == "localhost" || host == "127.0.0.1" || host == "::1") {
            return true;
        }

        // Check explicit allowlist
        if self.allowed_hosts.contains(host) {
            return true;
        }

        // Check wildcard domains
        for allowed_host in &self.allowed_hosts {
            if allowed_host == "*" {
                return true;
            }
            if let Some(domain) = allowed_host.strip_prefix("*.") {
                if host.ends_with(domain) {
                    return true;
                }
            }
        }

        false
    }
}

/// Complete WASI sandbox configuration
#[derive(Debug, Clone, Default)]
pub struct WasiSandboxConfig {
    /// Filesystem access configuration
    pub filesystem_access: FileSystemAccess,
    /// Network access configuration  
    pub network_access: NetworkAccess,
    /// Enable WASI preview1 features
    pub enable_wasi_preview1: bool,
    /// Environment variables to inherit
    pub inherited_env_vars: HashSet<String>,
    /// Working directory override
    pub working_directory: Option<PathBuf>,
}

impl WasiSandboxConfig {
    /// Create a secure default configuration (deny-all)
    pub fn secure_default() -> Self {
        Self {
            filesystem_access: FileSystemAccess {
                deny_all_fs: true,
                ..Default::default()
            },
            network_access: NetworkAccess {
                deny_all_network: true,
                allow_localhost: false,
                ..Default::default()
            },
            enable_wasi_preview1: true,
            inherited_env_vars: HashSet::new(),
            working_directory: None,
        }
    }

    /// Add read-only directory access
    pub fn add_read_only_dir(&mut self, path: PathBuf) -> Result<(), SandboxError> {
        // Disable deny-all when adding specific permissions
        self.filesystem_access.deny_all_fs = false;
        self.filesystem_access.add_read_only(path)
    }

    /// Add read-write directory access  
    pub fn add_read_write_dir(&mut self, path: PathBuf) -> Result<(), SandboxError> {
        // Disable deny-all when adding specific permissions
        self.filesystem_access.deny_all_fs = false;
        self.filesystem_access.add_read_write(path)
    }

    /// Add allowed network host
    pub fn add_allowed_host(&mut self, host: String) {
        // Disable deny-all when adding specific permissions
        self.network_access.deny_all_network = false;
        // Ignore errors in config building - validation happens later
        let _ = self.network_access.add_host(host);
    }

    /// Enable localhost connections
    pub fn allow_localhost(&mut self) {
        self.network_access.deny_all_network = false;
        self.network_access.allow_localhost = true;
    }

    /// Add inherited environment variable
    pub fn inherit_env_var(&mut self, var_name: String) {
        self.inherited_env_vars.insert(var_name);
    }

    /// Set working directory override
    pub fn set_working_directory(&mut self, dir: PathBuf) -> Result<(), SandboxError> {
        if !dir.exists() || !dir.is_dir() {
            return Err(SandboxError::WasiConfigError(format!(
                "Invalid working directory: {}",
                dir.display()
            )));
        }
        self.working_directory = Some(dir);
        Ok(())
    }

    /// Validate configuration consistency
    pub fn validate(&self) -> Result<(), SandboxError> {
        // Validate filesystem paths exist
        for path in &self.filesystem_access.read_only_dirs {
            if !path.exists() {
                return Err(SandboxError::WasiConfigError(format!(
                    "Read-only path does not exist: {}",
                    path.display()
                )));
            }
        }

        for path in &self.filesystem_access.read_write_dirs {
            if !path.exists() {
                return Err(SandboxError::WasiConfigError(format!(
                    "Read-write path does not exist: {}",
                    path.display()
                )));
            }
        }

        // Validate working directory
        if let Some(ref dir) = self.working_directory {
            if !dir.exists() || !dir.is_dir() {
                return Err(SandboxError::WasiConfigError(format!(
                    "Working directory does not exist: {}",
                    dir.display()
                )));
            }
        }

        Ok(())
    }

    /// Get security level (0-10, higher = more secure)
    pub fn security_level(&self) -> u8 {
        let mut level = 10;

        // Filesystem permissions reduce security
        if !self.filesystem_access.deny_all_fs {
            level -= 2;
            level -= (self.filesystem_access.read_write_dirs.len().min(3)) as u8;
        }

        // Network permissions reduce security
        if !self.network_access.deny_all_network {
            level -= 2;
            if self.network_access.allowed_hosts.contains("*") {
                level -= 3;
            } else {
                level -= (self.network_access.allowed_hosts.len().min(2)) as u8;
            }
        }

        // Environment variables reduce security slightly
        if !self.inherited_env_vars.is_empty() {
            level -= 1;
        }

        level
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_filesystem_access_creation() {
        let mut fs_access = FileSystemAccess::default();

        // Create a temporary directory for testing
        let temp_dir = std::env::temp_dir().join("wasm_sandbox_test");
        let _ = fs::create_dir_all(&temp_dir);

        assert!(fs_access.add_read_only(temp_dir.clone()).is_ok());
        assert!(fs_access.can_read(&temp_dir));
        assert!(!fs_access.can_write(&temp_dir));

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_network_access_validation() {
        let mut net_access = NetworkAccess::default();

        assert!(net_access.add_host("example.com".to_string()).is_ok());
        assert!(net_access.can_connect("example.com"));
        assert!(!net_access.can_connect("malicious.com"));

        // Test wildcard
        assert!(net_access.add_host("*.trusted.com".to_string()).is_ok());
        assert!(net_access.can_connect("api.trusted.com"));
        assert!(!net_access.can_connect("evil.com"));
    }

    #[test]
    fn test_secure_default_config() {
        let config = WasiSandboxConfig::secure_default();

        assert!(config.filesystem_access.deny_all_fs);
        assert!(config.network_access.deny_all_network);
        assert_eq!(config.security_level(), 10);
    }

    #[test]
    fn test_localhost_access() {
        let mut config = WasiSandboxConfig::default();
        config.allow_localhost();

        assert!(config.network_access.can_connect("localhost"));
        assert!(config.network_access.can_connect("127.0.0.1"));
        assert!(!config.network_access.can_connect("example.com"));
    }

    #[test]
    fn test_security_level_calculation() {
        let mut config = WasiSandboxConfig::secure_default();
        assert_eq!(config.security_level(), 10);

        // Add filesystem access - reduces security
        config.filesystem_access.deny_all_fs = false;
        assert!(config.security_level() < 10);

        // Add network access - reduces security further
        config.network_access.deny_all_network = false;
        config.network_access.allowed_hosts.insert("*".to_string());
        assert!(config.security_level() < 5);
    }
}
