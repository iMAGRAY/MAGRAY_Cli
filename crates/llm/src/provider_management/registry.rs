//! Provider Registry - Thread-safe provider storage and management

use super::*;
use crate::providers::{LlmProvider, ProviderCapabilities, ProviderId, ProviderWrapper};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Thread-safe registry for LLM providers
#[derive(Debug)]
pub struct ProviderRegistry {
    /// Registered providers
    providers: Arc<RwLock<HashMap<ProviderId, Arc<ProviderWrapper>>>>,
    /// Provider metadata
    metadata: Arc<RwLock<HashMap<ProviderId, ProviderMetadata>>>,
    /// Provider connections/pools
    connection_pools: Arc<RwLock<HashMap<ProviderId, ConnectionPool>>>,
    /// Registry configuration
    config: ProviderRegistryConfig,
}

/// Provider metadata for registry operations
#[derive(Debug, Clone)]
pub struct ProviderMetadata {
    pub id: ProviderId,
    pub capabilities: ProviderCapabilities,
    pub registration_time: Instant,
    pub last_used: Option<Instant>,
    pub total_requests: u64,
    pub active_requests: u32,
    pub tags: Vec<String>,
    pub priority: i32,
    pub enabled: bool,
}

/// Connection pool for provider instances
#[derive(Debug, Clone)]
pub struct ConnectionPool {
    pub max_connections: u32,
    pub current_connections: u32,
    pub connection_timeout: Duration,
    pub idle_timeout: Duration,
}

/// Registry configuration
#[derive(Debug, Clone)]
pub struct ProviderRegistryConfig {
    pub max_providers: usize,
    pub default_pool_size: u32,
    pub connection_timeout: Duration,
    pub cleanup_interval: Duration,
    pub enable_connection_pooling: bool,
}

impl Default for ProviderRegistryConfig {
    fn default() -> Self {
        Self {
            max_providers: 100,
            default_pool_size: 10,
            connection_timeout: Duration::from_secs(30),
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            enable_connection_pooling: true,
        }
    }
}

impl ProviderRegistry {
    /// Create a new provider registry
    pub fn new() -> Self {
        Self::new_with_config(ProviderRegistryConfig::default())
    }

    /// Create a new provider registry with custom configuration
    pub fn new_with_config(config: ProviderRegistryConfig) -> Self {
        debug!("Creating new provider registry with config: {:?}", config);

        Self {
            providers: Arc::new(RwLock::new(HashMap::new())),
            metadata: Arc::new(RwLock::new(HashMap::new())),
            connection_pools: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    /// Register a new provider
    pub async fn register_provider(
        &self,
        provider: Arc<ProviderWrapper>,
        tags: Vec<String>,
        priority: i32,
    ) -> Result<()> {
        let id = provider.id();

        debug!("Registering provider: {:?} with priority {}", id, priority);

        // Check if registry is at capacity
        {
            let providers = self.providers.read().await;
            if providers.len() >= self.config.max_providers {
                return Err(anyhow::anyhow!(
                    "Provider registry at capacity: {}/{}",
                    providers.len(),
                    self.config.max_providers
                ));
            }
        }

        let capabilities = provider.capabilities();
        let metadata = ProviderMetadata {
            id: id.clone(),
            capabilities,
            registration_time: Instant::now(),
            last_used: None,
            total_requests: 0,
            active_requests: 0,
            tags,
            priority,
            enabled: true,
        };

        // Create connection pool if enabled
        if self.config.enable_connection_pooling {
            let pool = ConnectionPool {
                max_connections: self.config.default_pool_size,
                current_connections: 0,
                connection_timeout: self.config.connection_timeout,
                idle_timeout: Duration::from_secs(300),
            };

            self.connection_pools.write().await.insert(id.clone(), pool);
        }

        // Register provider and metadata
        self.providers.write().await.insert(id.clone(), provider);
        self.metadata.write().await.insert(id.clone(), metadata);

        info!("Successfully registered provider: {:?}", id);
        Ok(())
    }

    /// Deregister a provider
    pub async fn deregister_provider(&self, id: &ProviderId) -> Result<()> {
        debug!("Deregistering provider: {:?}", id);

        // Check if provider exists
        {
            let providers = self.providers.read().await;
            if !providers.contains_key(id) {
                return Err(anyhow::anyhow!("Provider not found: {:?}", id));
            }
        }

        // Wait for active requests to complete
        self.wait_for_active_requests(id, Duration::from_secs(30))
            .await?;

        // Remove provider and associated data
        self.providers.write().await.remove(id);
        self.metadata.write().await.remove(id);
        self.connection_pools.write().await.remove(id);

        info!("Successfully deregistered provider: {:?}", id);
        Ok(())
    }

    /// Get a provider by ID
    pub async fn get_provider(&self, id: &ProviderId) -> Option<Arc<ProviderWrapper>> {
        let providers = self.providers.read().await;
        providers.get(id).cloned()
    }

    /// Get provider metadata
    pub async fn get_metadata(&self, id: &ProviderId) -> Option<ProviderMetadata> {
        let metadata = self.metadata.read().await;
        metadata.get(id).cloned()
    }

    /// List all registered providers
    pub async fn list_providers(&self) -> Vec<ProviderId> {
        let providers = self.providers.read().await;
        providers.keys().cloned().collect()
    }

    /// List providers by tag
    pub async fn list_providers_by_tag(&self, tag: &str) -> Vec<ProviderId> {
        let metadata = self.metadata.read().await;
        metadata
            .values()
            .filter(|m| m.tags.contains(&tag.to_string()) && m.enabled)
            .map(|m| m.id.clone())
            .collect()
    }

    /// List enabled providers
    pub async fn list_enabled_providers(&self) -> Vec<ProviderId> {
        let metadata = self.metadata.read().await;
        metadata
            .values()
            .filter(|m| m.enabled)
            .map(|m| m.id.clone())
            .collect()
    }

    /// Enable/disable a provider
    pub async fn set_provider_enabled(&self, id: &ProviderId, enabled: bool) -> Result<()> {
        let mut metadata = self.metadata.write().await;

        match metadata.get_mut(id) {
            Some(meta) => {
                meta.enabled = enabled;
                info!(
                    "Provider {:?} {} successfully",
                    id,
                    if enabled { "enabled" } else { "disabled" }
                );
                Ok(())
            }
            None => Err(anyhow::anyhow!("Provider not found: {:?}", id)),
        }
    }

    /// Update provider priority
    pub async fn set_provider_priority(&self, id: &ProviderId, priority: i32) -> Result<()> {
        let mut metadata = self.metadata.write().await;

        match metadata.get_mut(id) {
            Some(meta) => {
                meta.priority = priority;
                info!("Provider {:?} priority updated to {}", id, priority);
                Ok(())
            }
            None => Err(anyhow::anyhow!("Provider not found: {:?}", id)),
        }
    }

    /// Record provider usage
    pub async fn record_usage(&self, id: &ProviderId) -> Result<()> {
        let mut metadata = self.metadata.write().await;

        match metadata.get_mut(id) {
            Some(meta) => {
                meta.last_used = Some(Instant::now());
                meta.total_requests += 1;
                Ok(())
            }
            None => Err(anyhow::anyhow!("Provider not found: {:?}", id)),
        }
    }

    /// Increment active request counter
    pub async fn increment_active_requests(&self, id: &ProviderId) -> Result<()> {
        let mut metadata = self.metadata.write().await;

        match metadata.get_mut(id) {
            Some(meta) => {
                meta.active_requests += 1;
                Ok(())
            }
            None => Err(anyhow::anyhow!("Provider not found: {:?}", id)),
        }
    }

    /// Decrement active request counter
    pub async fn decrement_active_requests(&self, id: &ProviderId) -> Result<()> {
        let mut metadata = self.metadata.write().await;

        match metadata.get_mut(id) {
            Some(meta) => {
                if meta.active_requests > 0 {
                    meta.active_requests -= 1;
                }
                Ok(())
            }
            None => Err(anyhow::anyhow!("Provider not found: {:?}", id)),
        }
    }

    /// Get provider statistics
    pub async fn get_statistics(&self) -> RegistryStatistics {
        let providers = self.providers.read().await;
        let metadata = self.metadata.read().await;

        let total_providers = providers.len();
        let enabled_providers = metadata.values().filter(|m| m.enabled).count();
        let active_requests: u32 = metadata.values().map(|m| m.active_requests).sum();
        let total_requests: u64 = metadata.values().map(|m| m.total_requests).sum();

        RegistryStatistics {
            total_providers,
            enabled_providers,
            active_requests,
            total_requests,
            registry_uptime: Instant::now(),
        }
    }

    /// Wait for active requests to complete
    async fn wait_for_active_requests(&self, id: &ProviderId, timeout: Duration) -> Result<()> {
        let start = Instant::now();

        while start.elapsed() < timeout {
            {
                let metadata = self.metadata.read().await;
                if let Some(meta) = metadata.get(id) {
                    if meta.active_requests == 0 {
                        return Ok(());
                    }
                } else {
                    return Ok(()); // Provider already removed
                }
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        warn!(
            "Timeout waiting for active requests to complete for provider: {:?}",
            id
        );
        Ok(()) // Continue with deregistration even if there are active requests
    }

    /// Cleanup unused connections and stale metadata
    pub async fn cleanup(&self) {
        debug!("Starting registry cleanup");

        // This would be implemented to clean up unused connections,
        // remove stale metadata, and optimize memory usage

        // For now, just log the operation
        let stats = self.get_statistics().await;
        debug!("Registry cleanup completed. Stats: {:?}", stats);
    }

    /// Get connection pool info
    pub async fn get_connection_pool(&self, id: &ProviderId) -> Option<ConnectionPool> {
        let pools = self.connection_pools.read().await;
        pools.get(id).cloned()
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry statistics
#[derive(Debug, Clone)]
pub struct RegistryStatistics {
    pub total_providers: usize,
    pub enabled_providers: usize,
    pub active_requests: u32,
    pub total_requests: u64,
    pub registry_uptime: Instant,
}
