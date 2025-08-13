use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct FsSandboxConfig {
    pub enabled: bool,
    /// DEPRECATED: Use fs_read_roots and fs_write_roots instead
    #[deprecated(
        since = "0.1.0",
        note = "Use fs_read_roots and fs_write_roots for better security"
    )]
    pub roots: Vec<String>,
    /// Allowed root directories for READ operations (file_read, dir_list, file_search)
    pub fs_read_roots: Vec<String>,
    /// Allowed root directories for WRITE operations (file_write, file_delete)
    pub fs_write_roots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct NetSandboxConfig {
    pub allowlist: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ShellSandboxConfig {
    pub allow_shell: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpServerConfig {
    /// Whitelist of allowed MCP servers (empty = block all)
    pub server_whitelist: Vec<String>,
    /// Blacklist of banned MCP servers (higher priority than whitelist)
    pub server_blacklist: Vec<String>,
    /// CRITICAL P0.2.5: Connection timeout in milliseconds (default 30s)
    pub connection_timeout_ms: u64,
    /// CRITICAL P0.2.5: Heartbeat interval in milliseconds (default 60s)
    pub heartbeat_interval_ms: u64,
    /// CRITICAL P0.2.5: Maximum execution time in milliseconds (default 5 minutes)
    pub max_execution_time_ms: u64,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            server_whitelist: Vec::new(),
            server_blacklist: Vec::new(),
            // CRITICAL P0.2.5: Secure default timeouts
            connection_timeout_ms: 30_000,  // 30 seconds
            heartbeat_interval_ms: 60_000,  // 60 seconds
            max_execution_time_ms: 300_000, // 5 minutes
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SandboxConfig {
    pub fs: FsSandboxConfig,
    pub net: NetSandboxConfig,
    pub shell: ShellSandboxConfig,
    /// CRITICAL P0.2.4: MCP server filtering configuration
    pub mcp: McpServerConfig,
}

impl SandboxConfig {
    fn default_path() -> std::path::PathBuf {
        // Avoid circular dep on cli; resolve home here
        let base = std::env::var("MAGRAY_HOME")
            .ok()
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                let mut d = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
                d.push(".magray");
                d
            });
        let mut p = base;
        std::fs::create_dir_all(&p).ok();
        p.push("sandbox.json");
        p
    }

    fn from_file() -> Option<Self> {
        let p = Self::default_path();
        if !p.exists() {
            return None;
        }
        let data = std::fs::read_to_string(&p).ok()?;
        serde_json::from_str::<SandboxConfig>(&data).ok()
    }

    pub fn from_env() -> Self {
        let mut cfg = Self::from_file().unwrap_or_default();

        // FS
        if let Ok(fs_flag) = std::env::var("MAGRAY_FS_SANDBOX") {
            let f = fs_flag.to_lowercase();
            cfg.fs.enabled = matches!(f.as_str(), "1" | "true" | "on" | "enforce");
        }

        // LEGACY: MAGRAY_FS_ROOTS (deprecated but still supported)
        if let Ok(fs_roots_env) = std::env::var("MAGRAY_FS_ROOTS") {
            let roots: Vec<String> = fs_roots_env
                .split(':')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !roots.is_empty() {
                // Set the deprecated field for backward compatibility (will be removed)
                #[allow(deprecated)]
                {
                    cfg.fs.roots = roots.clone();
                }
                // If new specific roots are not set, use legacy for both
                if cfg.fs.fs_read_roots.is_empty() {
                    cfg.fs.fs_read_roots = roots.clone();
                }
                if cfg.fs.fs_write_roots.is_empty() {
                    cfg.fs.fs_write_roots = roots;
                }
            }
        }

        // NEW: Separate read/write roots for enhanced security
        if let Ok(fs_read_roots_env) = std::env::var("MAGRAY_FS_READ_ROOTS") {
            let roots: Vec<String> = fs_read_roots_env
                .split(':')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !roots.is_empty() {
                cfg.fs.fs_read_roots = roots;
            }
        }

        if let Ok(fs_write_roots_env) = std::env::var("MAGRAY_FS_WRITE_ROOTS") {
            let roots: Vec<String> = fs_write_roots_env
                .split(':')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !roots.is_empty() {
                cfg.fs.fs_write_roots = roots;
            }
        }

        // NET
        if let Ok(net_allow_env) = std::env::var("MAGRAY_NET_ALLOW") {
            let allowlist: Vec<String> = net_allow_env
                .split(',')
                .map(|s| s.trim().to_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
            cfg.net.allowlist = allowlist;
        }

        // SHELL
        if let Ok(allow_shell_env) = std::env::var("MAGRAY_ALLOW_SHELL") {
            let val = allow_shell_env.to_lowercase();
            cfg.shell.allow_shell = matches!(val.as_str(), "1" | "true" | "yes" | "on");
        }

        // MCP SERVER FILTERING - CRITICAL P0.2.4
        if let Ok(mcp_whitelist_env) = std::env::var("MAGRAY_MCP_SERVER_WHITELIST") {
            let whitelist: Vec<String> = mcp_whitelist_env
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            cfg.mcp.server_whitelist = whitelist;
        }

        if let Ok(mcp_blacklist_env) = std::env::var("MAGRAY_MCP_SERVER_BLACKLIST") {
            let blacklist: Vec<String> = mcp_blacklist_env
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            cfg.mcp.server_blacklist = blacklist;
        }

        // CRITICAL P0.2.5: MCP TIMEOUT/HEARTBEAT CONFIGURATION
        if let Ok(conn_timeout_env) = std::env::var("MAGRAY_MCP_CONNECTION_TIMEOUT") {
            if let Ok(timeout_ms) = conn_timeout_env.parse::<u64>() {
                // Enforce security limits (1s to 5 minutes)
                cfg.mcp.connection_timeout_ms = timeout_ms.clamp(1_000, 300_000);
            }
        }

        if let Ok(heartbeat_env) = std::env::var("MAGRAY_MCP_HEARTBEAT_INTERVAL") {
            if let Ok(heartbeat_ms) = heartbeat_env.parse::<u64>() {
                // Enforce security limits (10s to 10 minutes)
                cfg.mcp.heartbeat_interval_ms = heartbeat_ms.clamp(10_000, 600_000);
            }
        }

        if let Ok(max_exec_env) = std::env::var("MAGRAY_MCP_MAX_EXECUTION_TIME") {
            if let Ok(max_time_ms) = max_exec_env.parse::<u64>() {
                // Enforce security limits (5s to 30 minutes)
                cfg.mcp.max_execution_time_ms = max_time_ms.clamp(5_000, 1_800_000);
            }
        }

        cfg
    }

    pub fn save_to_file(&self) -> std::io::Result<()> {
        let p = Self::default_path();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let s = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into());
        std::fs::write(p, s)
    }

    /// SECURITY P0.1.6: Validate path access against allowed read roots
    pub fn validate_read_access(&self, path: &str) -> Result<(), anyhow::Error> {
        if !self.fs.enabled {
            return Ok(());
        }

        let allowed_roots = &self.fs.fs_read_roots;
        if allowed_roots.is_empty() {
            return Err(anyhow::anyhow!(
                "🔒 FILESYSTEM SECURITY: No read roots configured. Set MAGRAY_FS_READ_ROOTS environment variable."
            ));
        }

        validate_path_against_roots(path, allowed_roots, "read")
    }

    /// SECURITY P0.1.6: Validate path access against allowed write roots
    pub fn validate_write_access(&self, path: &str) -> Result<(), anyhow::Error> {
        if !self.fs.enabled {
            return Ok(());
        }

        let allowed_roots = &self.fs.fs_write_roots;
        if allowed_roots.is_empty() {
            return Err(anyhow::anyhow!(
                "🔒 FILESYSTEM SECURITY: No write roots configured. Set MAGRAY_FS_WRITE_ROOTS environment variable."
            ));
        }

        validate_path_against_roots(path, allowed_roots, "write")
    }

    /// CRITICAL P0.2.4: Validate MCP server against whitelist/blacklist
    /// SECURE BY DEFAULT: Empty whitelist blocks ALL servers
    pub fn validate_mcp_server(&self, server_url: &str) -> Result<(), anyhow::Error> {
        // 1. Check blacklist first (highest priority)
        if self.mcp.server_blacklist.contains(&server_url.to_string()) {
            return Err(anyhow::anyhow!(
                "🔒 MCP SECURITY VIOLATION: Server '{}' is BLACKLISTED. Connection blocked for security.",
                server_url
            ));
        }

        // 2. Check whitelist (secure by default)
        if self.mcp.server_whitelist.is_empty() {
            return Err(anyhow::anyhow!(
                "🔒 MCP SECURITY POLICY: No MCP servers whitelisted. Set MAGRAY_MCP_SERVER_WHITELIST environment variable to allow servers. Server '{}' blocked by default security policy.",
                server_url
            ));
        }

        if !self.mcp.server_whitelist.contains(&server_url.to_string()) {
            return Err(anyhow::anyhow!(
                "🔒 MCP SECURITY VIOLATION: Server '{}' not in whitelist. Allowed servers: {:?}. Add to MAGRAY_MCP_SERVER_WHITELIST to grant access.",
                server_url,
                self.mcp.server_whitelist
            ));
        }

        // Server is whitelisted and not blacklisted
        eprintln!("🔓 MCP SERVER APPROVED: '{server_url}' found in whitelist. Connection allowed.");
        Ok(())
    }
}

/// SECURITY P0.1.6: Core path validation logic against directory traversal attacks
fn validate_path_against_roots(
    path: &str,
    allowed_roots: &[String],
    operation: &str,
) -> Result<(), anyhow::Error> {
    // 1. Basic path traversal protection
    if path.contains("..") {
        return Err(anyhow::anyhow!(
            "🔒 SECURITY VIOLATION: Path traversal attack detected in '{}' (operation: {})",
            path,
            operation
        ));
    }

    // 2. Handle path validation based on whether file exists or not
    let path_obj = std::path::Path::new(path);
    let target_path =
        if path_obj.exists() {
            // File/directory exists - canonicalize it directly
            std::fs::canonicalize(path_obj).map_err(|e| {
                anyhow::anyhow!(
                "🔒 FILESYSTEM ERROR: Cannot canonicalize existing path '{}' for {} operation: {}",
                path, operation, e
            )
            })?
        } else {
            // File doesn't exist - check parent directory and construct expected path
            let parent = path_obj.parent().unwrap_or(std::path::Path::new("."));
            let parent_canonical =
                std::fs::canonicalize(parent).map_err(|e| {
                    anyhow::anyhow!(
                "🔒 FILESYSTEM ERROR: Cannot access parent directory '{}' for {} operation: {}",
                parent.display(), operation, e
            )
                })?;

            // Construct the expected canonical path
            if let Some(filename) = path_obj.file_name() {
                parent_canonical.join(filename)
            } else {
                parent_canonical
            }
        };

    // 3. Check if path is within any allowed root
    for root_str in allowed_roots {
        if let Ok(canonical_root) = std::fs::canonicalize(std::path::Path::new(root_str)) {
            if target_path.starts_with(&canonical_root) {
                return Ok(());
            }
        }
    }

    Err(anyhow::anyhow!(
        "🔒 FILESYSTEM SECURITY VIOLATION: Path '{}' not allowed for {} operation.\nTarget path: {}\nAllowed roots: {:?}\nConfigure MAGRAY_FS_{}_ROOTS to grant access.",
        path,
        target_path.display(),
        operation,
        allowed_roots,
        operation.to_uppercase()
    ))
}
