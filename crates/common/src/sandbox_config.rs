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
    fn default_path() -> std::path::PathBuf {
        // Avoid circular dep on cli; resolve home here
        let base = std::env::var("MAGRAY_HOME").ok().map(std::path::PathBuf::from).unwrap_or_else(|| {
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
        if !p.exists() { return None; }
        let data = std::fs::read_to_string(&p).ok()?;
        serde_json::from_str::<SandboxConfig>(&data).ok()
    }

    pub fn from_env() -> Self {
        // Start with file config if present
        let mut cfg = Self::from_file().unwrap_or_default();

        // FS
        if let Ok(fs_flag) = std::env::var("MAGRAY_FS_SANDBOX") {
            let f = fs_flag.to_lowercase();
            cfg.fs.enabled = matches!(f.as_str(), "1" | "true" | "on" | "enforce");
        }
        if let Ok(fs_roots_env) = std::env::var("MAGRAY_FS_ROOTS") {
            let roots: Vec<String> = fs_roots_env
                .split(':')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !roots.is_empty() { cfg.fs.roots = roots; }
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

        cfg
    }

    pub fn save_to_file(&self) -> std::io::Result<()> {
        let p = Self::default_path();
        if let Some(parent) = p.parent() { std::fs::create_dir_all(parent)?; }
        let s = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into());
        std::fs::write(p, s)
    }
}