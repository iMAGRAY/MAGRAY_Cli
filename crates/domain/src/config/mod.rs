use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

/// Configuration profile for different environments
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Profile {
    /// Development profile with relaxed security and verbose logging
    #[default]
    Dev,
    /// Production profile with strict security and minimal logging
    Prod,
    /// Custom profile with user-defined name
    Custom(String),
}

impl Profile {
    /// Get the profile name as a string
    pub fn name(&self) -> &str {
        match self {
            Profile::Dev => "dev",
            Profile::Prod => "prod",
            Profile::Custom(name) => name,
        }
    }

    /// Parse profile from string (legacy method, prefer FromStr trait)
    pub fn parse_from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "dev" | "development" => Profile::Dev,
            "prod" | "production" => Profile::Prod,
            custom => Profile::Custom(custom.to_string()),
        }
    }
}

impl FromStr for Profile {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "dev" | "development" => Profile::Dev,
            "prod" | "production" => Profile::Prod,
            custom => Profile::Custom(custom.to_string()),
        })
    }
}

/// Profile-specific configuration overrides
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProfileConfig {
    /// Security policy configuration for this profile
    #[serde(default)]
    pub security: SecurityConfig,

    /// Logging configuration for this profile  
    #[serde(default)]
    pub logging: ProfileLoggingConfig,

    /// Performance optimizations for this profile
    #[serde(default)]
    pub performance: ProfilePerformanceConfig,

    /// Tools configuration for this profile
    #[serde(default)]
    pub tools: ProfileToolsConfig,
}

/// Security configuration per profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Default policy mode (ask/allow/deny)
    #[serde(default = "default_dev_policy_mode")]
    pub default_policy_mode: String,

    /// Risk tolerance level (low/medium/high)
    #[serde(default = "default_dev_risk_level")]
    pub risk_level: String,

    /// Whether to enable permissive mode for rapid development
    #[serde(default)]
    pub permissive_mode: bool,

    /// Whether to ask user by default for unknown operations
    #[serde(default = "default_ask_by_default")]
    pub ask_by_default: bool,
}

/// Profile-specific logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileLoggingConfig {
    /// Log level override for this profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level_override: Option<String>,

    /// Whether to enable console output
    #[serde(default = "default_console_enabled")]
    pub console_enabled: bool,

    /// Whether to enable debug symbol output
    #[serde(default)]
    pub debug_symbols: bool,

    /// Whether to use structured logging only
    #[serde(default)]
    pub structured_only: bool,
}

/// Profile-specific performance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilePerformanceConfig {
    /// Whether to enable debug allocation tracking
    #[serde(default)]
    pub debug_allocation_tracking: bool,

    /// Memory limits - relaxed for dev, strict for prod
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_limit_override_mb: Option<usize>,

    /// Whether to enable production optimizations
    #[serde(default)]
    pub production_optimizations: bool,
}

/// Profile-specific tools configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileToolsConfig {
    /// Tool whitelist mode (expanded/minimal/custom)
    #[serde(default = "default_tool_whitelist_mode")]
    pub whitelist_mode: String,

    /// Whether to enable dry-run by default
    #[serde(default)]
    pub dry_run_default: bool,

    /// Whether to require signed tools
    #[serde(default = "default_require_signed_tools")]
    pub require_signed_tools: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MagrayConfig {
    /// Active configuration profile
    #[serde(default)]
    pub profile: Profile,

    /// Profile-specific configuration overrides
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_config: Option<ProfileConfig>,

    #[serde(default)]
    pub ai: AiConfig,

    #[serde(default)]
    pub memory: MemoryConfig,

    #[serde(default)]
    pub mcp: McpConfig,

    #[serde(default)]
    pub plugins: PluginsConfig,

    #[serde(default)]
    pub logging: LoggingConfig,

    #[serde(default)]
    pub paths: PathsConfig,

    #[serde(default)]
    pub performance: PerformanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiConfig {
    #[serde(default = "default_ai_provider")]
    pub default_provider: String,

    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,

    #[serde(default)]
    pub fallback_chain: Vec<String>,

    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,

    #[serde(default = "default_temperature")]
    pub temperature: f32,

    #[serde(default)]
    pub retry_config: RetryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub provider_type: ProviderType,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_base: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_path: Option<PathBuf>,

    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Google,
    Local,
    Azure,
    Groq,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    #[serde(default = "default_memory_backend")]
    pub backend: MemoryBackend,

    #[serde(default = "default_hnsw_config")]
    pub hnsw: HnswConfig,

    #[serde(default)]
    pub embedding: EmbeddingConfig,

    #[serde(default = "default_cache_size")]
    pub cache_size_mb: usize,

    #[serde(default = "default_flush_interval")]
    pub flush_interval_sec: u64,

    #[serde(default)]
    pub persistence: PersistenceConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryBackend {
    SQLite,
    InMemory,
    Hybrid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HnswConfig {
    #[serde(default = "default_hnsw_m")]
    pub m: usize,

    #[serde(default = "default_hnsw_ef_construction")]
    pub ef_construction: usize,

    #[serde(default = "default_hnsw_ef_search")]
    pub ef_search: usize,

    #[serde(default = "default_hnsw_max_elements")]
    pub max_elements: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    #[serde(default = "default_embedding_model")]
    pub model: String,

    #[serde(default = "default_embedding_dimension")]
    pub dimension: usize,

    #[serde(default)]
    pub use_gpu: bool,

    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,

    #[serde(default = "default_auto_save_interval")]
    pub auto_save_interval_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub servers: Vec<McpServerConfig>,

    #[serde(default = "default_mcp_timeout")]
    pub timeout_sec: u64,

    #[serde(default)]
    pub auto_discovery: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub url: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth_token: Option<String>,

    #[serde(default)]
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginsConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub plugin_dir: Option<PathBuf>,

    #[serde(default)]
    pub auto_load: Vec<String>,

    #[serde(default)]
    pub sandbox_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,

    #[serde(default)]
    pub file_enabled: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<PathBuf>,

    #[serde(default)]
    pub structured: bool,

    #[serde(default = "default_max_log_size")]
    pub max_size_mb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PathsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_dir: Option<PathBuf>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<PathBuf>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub models_dir: Option<PathBuf>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub logs_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    #[serde(default = "default_worker_threads")]
    pub worker_threads: usize,

    #[serde(default = "default_max_concurrent_requests")]
    pub max_concurrent_requests: usize,

    #[serde(default)]
    pub enable_gpu: bool,

    #[serde(default = "default_memory_limit")]
    pub memory_limit_mb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    #[serde(default = "default_max_retries")]
    pub max_retries: usize,

    #[serde(default = "default_initial_delay")]
    pub initial_delay_ms: u64,

    #[serde(default = "default_max_delay")]
    pub max_delay_ms: u64,

    #[serde(default = "default_exponential_base")]
    pub exponential_base: f32,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            default_provider: default_ai_provider(),
            providers: HashMap::new(),
            fallback_chain: Vec::new(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            retry_config: RetryConfig::default(),
        }
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            backend: default_memory_backend(),
            hnsw: default_hnsw_config(),
            embedding: EmbeddingConfig::default(),
            cache_size_mb: default_cache_size(),
            flush_interval_sec: default_flush_interval(),
            persistence: PersistenceConfig::default(),
        }
    }
}

impl Default for HnswConfig {
    fn default() -> Self {
        default_hnsw_config()
    }
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            model: default_embedding_model(),
            dimension: default_embedding_dimension(),
            use_gpu: false,
            batch_size: default_batch_size(),
        }
    }
}

impl Default for PersistenceConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            path: None,
            auto_save_interval_sec: default_auto_save_interval(),
        }
    }
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            servers: Vec::new(),
            timeout_sec: default_mcp_timeout(),
            auto_discovery: false,
        }
    }
}

impl Default for PluginsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            plugin_dir: None,
            auto_load: Vec::new(),
            sandbox_enabled: true,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file_enabled: false,
            file_path: None,
            structured: false,
            max_size_mb: default_max_log_size(),
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            worker_threads: default_worker_threads(),
            max_concurrent_requests: default_max_concurrent_requests(),
            enable_gpu: false,
            memory_limit_mb: default_memory_limit(),
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            initial_delay_ms: default_initial_delay(),
            max_delay_ms: default_max_delay(),
            exponential_base: default_exponential_base(),
        }
    }
}

fn default_ai_provider() -> String {
    "openai".to_string()
}

fn default_max_tokens() -> usize {
    4096
}

fn default_temperature() -> f32 {
    0.7
}

fn default_memory_backend() -> MemoryBackend {
    MemoryBackend::SQLite
}

fn default_hnsw_config() -> HnswConfig {
    HnswConfig {
        m: default_hnsw_m(),
        ef_construction: default_hnsw_ef_construction(),
        ef_search: default_hnsw_ef_search(),
        max_elements: default_hnsw_max_elements(),
    }
}

fn default_hnsw_m() -> usize {
    16
}

fn default_hnsw_ef_construction() -> usize {
    200
}

fn default_hnsw_ef_search() -> usize {
    50
}

fn default_hnsw_max_elements() -> usize {
    1_000_000
}

fn default_embedding_model() -> String {
    "qwen3-0.6b".to_string()
}

fn default_embedding_dimension() -> usize {
    384
}

fn default_batch_size() -> usize {
    32
}

fn default_cache_size() -> usize {
    256
}

fn default_flush_interval() -> u64 {
    60
}

fn default_auto_save_interval() -> u64 {
    300
}

fn default_mcp_timeout() -> u64 {
    30
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_max_log_size() -> usize {
    100
}

fn default_worker_threads() -> usize {
    4
}

fn default_max_concurrent_requests() -> usize {
    10
}

fn default_memory_limit() -> usize {
    1024
}

fn default_max_retries() -> usize {
    3
}

fn default_initial_delay() -> u64 {
    100
}

fn default_max_delay() -> u64 {
    10000
}

fn default_exponential_base() -> f32 {
    2.0
}

// Profile-specific default functions

/// Default policy mode for development profile (permissive)
fn default_dev_policy_mode() -> String {
    "ask".to_string()
}

/// Default risk level for development profile  
fn default_dev_risk_level() -> String {
    "medium".to_string()
}

/// Default ask-by-default for development (false for rapid development)
fn default_ask_by_default() -> bool {
    true
}

/// Default console enabled (true for dev, false for prod)
fn default_console_enabled() -> bool {
    true
}

/// Default tool whitelist mode (expanded for dev, minimal for prod)
fn default_tool_whitelist_mode() -> String {
    "expanded".to_string()
}

/// Default require signed tools (false for dev, true for prod)
fn default_require_signed_tools() -> bool {
    false
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            default_policy_mode: default_dev_policy_mode(),
            risk_level: default_dev_risk_level(),
            permissive_mode: true, // Dev default
            ask_by_default: default_ask_by_default(),
        }
    }
}

impl Default for ProfileLoggingConfig {
    fn default() -> Self {
        Self {
            level_override: None,
            console_enabled: default_console_enabled(),
            debug_symbols: true,    // Dev default
            structured_only: false, // Dev default
        }
    }
}

impl Default for ProfilePerformanceConfig {
    fn default() -> Self {
        Self {
            debug_allocation_tracking: true, // Dev default
            memory_limit_override_mb: None,
            production_optimizations: false, // Dev default
        }
    }
}

impl Default for ProfileToolsConfig {
    fn default() -> Self {
        Self {
            whitelist_mode: default_tool_whitelist_mode(),
            dry_run_default: true, // Dev default for safety
            require_signed_tools: default_require_signed_tools(),
        }
    }
}

impl ProfileConfig {
    /// Create production profile configuration
    pub fn prod() -> Self {
        Self {
            security: SecurityConfig {
                default_policy_mode: "ask".to_string(),
                risk_level: "low".to_string(),
                permissive_mode: false,
                ask_by_default: true,
            },
            logging: ProfileLoggingConfig {
                level_override: Some("warn".to_string()),
                console_enabled: false,
                debug_symbols: false,
                structured_only: true,
            },
            performance: ProfilePerformanceConfig {
                debug_allocation_tracking: false,
                memory_limit_override_mb: Some(512),
                production_optimizations: true,
            },
            tools: ProfileToolsConfig {
                whitelist_mode: "minimal".to_string(),
                dry_run_default: false,
                require_signed_tools: true,
            },
        }
    }

    /// Create development profile configuration  
    pub fn dev() -> Self {
        Self::default()
    }
}

impl MagrayConfig {
    /// Apply profile configuration to base config
    pub fn apply_profile(&mut self, profile_config: &ProfileConfig) {
        // Apply logging overrides
        if let Some(level) = &profile_config.logging.level_override {
            self.logging.level = level.clone();
        }

        self.logging.structured = profile_config.logging.structured_only;

        // Apply performance overrides
        if let Some(memory_limit) = profile_config.performance.memory_limit_override_mb {
            self.performance.memory_limit_mb = memory_limit;
        }

        // Apply plugin sandbox settings from security config
        self.plugins.sandbox_enabled = !profile_config.security.permissive_mode;
    }

    /// Get effective security settings based on profile
    pub fn effective_security(&self) -> &SecurityConfig {
        if let Some(ref profile_config) = self.profile_config {
            &profile_config.security
        } else {
            // Fallback to default security based on profile
            match self.profile {
                Profile::Prod => {
                    static PROD_SECURITY: SecurityConfig = SecurityConfig {
                        default_policy_mode: String::new(), // Will be updated
                        risk_level: String::new(),
                        permissive_mode: false,
                        ask_by_default: true,
                    };
                    &PROD_SECURITY
                }
                Profile::Dev | Profile::Custom(_) => {
                    static DEV_SECURITY: SecurityConfig = SecurityConfig {
                        default_policy_mode: String::new(),
                        risk_level: String::new(),
                        permissive_mode: true,
                        ask_by_default: true,
                    };
                    &DEV_SECURITY
                }
            }
        }
    }

    /// Get effective tools settings based on profile  
    pub fn effective_tools(&self) -> &ProfileToolsConfig {
        if let Some(ref profile_config) = self.profile_config {
            &profile_config.tools
        } else {
            static DEFAULT_TOOLS: ProfileToolsConfig = ProfileToolsConfig {
                whitelist_mode: String::new(),
                dry_run_default: true,
                require_signed_tools: false,
            };
            &DEFAULT_TOOLS
        }
    }
}
