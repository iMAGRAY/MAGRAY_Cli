use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct FsSandboxConfig {
    pub enabled: bool,
    pub roots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct NetSandboxConfig {
    pub allowlist: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ShellSandboxConfig {
    pub allow_shell: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct SandboxConfig {
    pub fs: FsSandboxConfig,
    pub net: NetSandboxConfig,
    pub shell: ShellSandboxConfig,
}

impl SandboxConfig {
    pub fn from_env() -> Self {
        // FS
        let fs_flag = std::env::var("MAGRAY_FS_SANDBOX").unwrap_or_default().to_lowercase();
        let fs_enabled = matches!(fs_flag.as_str(), "1" | "true" | "on" | "enforce");
        let fs_roots_env = std::env::var("MAGRAY_FS_ROOTS").unwrap_or_default();
        let fs_roots: Vec<String> = fs_roots_env
            .split(':')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // NET
        let net_allow_env = std::env::var("MAGRAY_NET_ALLOW").unwrap_or_default();
        let net_allowlist: Vec<String> = net_allow_env
            .split(',')
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .collect();

        // SHELL
        let allow_shell_env = std::env::var("MAGRAY_ALLOW_SHELL").unwrap_or_default().to_lowercase();
        let allow_shell = matches!(allow_shell_env.as_str(), "1" | "true" | "yes" | "on");

        SandboxConfig {
            fs: FsSandboxConfig { enabled: fs_enabled, roots: fs_roots },
            net: NetSandboxConfig { allowlist: net_allowlist },
            shell: ShellSandboxConfig { allow_shell },
        }
    }
}