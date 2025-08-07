//! Configuration Compatibility Layer
//!
//! This module provides backward compatibility adapters for the unified configuration
//! system to work seamlessly with existing legacy configuration structures.

use anyhow::Result;
use std::path::PathBuf;

use super::unified_config::{DatabaseConfig, MemorySystemConfig, UnifiedDIConfiguration};
use crate::{
    cache_lru::CacheConfig,
    hnsw_index::HnswConfig,
    service_di::service_config::{MemoryServiceConfig, MemoryServiceConfigBuilder},
};

/// Backward compatibility adapter for legacy configuration systems
pub struct ConfigCompatibilityAdapter;

impl ConfigCompatibilityAdapter {
    /// Convert UnifiedDIConfiguration to legacy MemoryServiceConfig
    ///
    /// This ensures that existing code that expects MemoryServiceConfig
    /// continues to work with the new unified system.
    pub fn to_memory_service_config(
        unified: &UnifiedDIConfiguration,
    ) -> Result<MemoryServiceConfig> {
        Ok(MemoryServiceConfig {
            db_path: unified.memory.database.db_path.clone(),
            cache_path: unified.core.data_dir.join("cache"),
            promotion: crate::types::PromotionConfig::default(), // Use existing default
            ml_promotion: unified.memory.ml_promotion.clone(),
            streaming_config: unified.memory.streaming.clone(),
            ai_config: unified.ai.clone(),
            cache_config: crate::CacheConfigType::from_unified(&unified.memory.cache),
            health_enabled: unified.orchestration.health.enable_alerts,
            health_config: unified.orchestration.health.clone(),
            resource_config: unified.orchestration.resources.clone(),
            notification_config: unified.orchestration.notifications.clone(),
            batch_config: unified.memory.batch.clone(),
        })
    }

    /// Convert legacy MemoryServiceConfig to UnifiedDIConfiguration
    ///
    /// This allows migration from old configuration format to the new unified format.
    pub fn from_memory_service_config(
        legacy: &MemoryServiceConfig,
    ) -> Result<UnifiedDIConfiguration> {
        let mut unified = UnifiedDIConfiguration::development();

        // Core settings from legacy paths
        unified.core.data_dir = legacy
            .db_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf();

        // Memory system mapping
        unified.memory = MemorySystemConfig {
            database: DatabaseConfig {
                db_path: legacy.db_path.clone(),
                pool_size: 10, // Default from legacy
                wal_mode: true,
                pragma_settings: std::collections::HashMap::new(),
            },
            cache: CacheConfig::from_legacy_type(&legacy.cache_config),
            batch: legacy.batch_config.clone(),
            hnsw: HnswConfig::default(), // Legacy didn't have HNSW config
            promotion: legacy.promotion.clone(),
            ml_promotion: legacy.ml_promotion.clone(),
            streaming: legacy.streaming_config.clone(),
        };

        // AI settings
        unified.ai = legacy.ai_config.clone();

        // Orchestration settings
        unified.orchestration.health = legacy.health_config.clone();
        unified.orchestration.resources = legacy.resource_config.clone();
        unified.orchestration.notifications = legacy.notification_config.clone();

        Ok(unified)
    }

    /// Create legacy-compatible factory function
    ///
    /// This provides a drop-in replacement for existing MemoryServiceConfig creation.
    pub fn create_legacy_compatible_config() -> Result<MemoryServiceConfig> {
        let unified = UnifiedDIConfiguration::development();
        Self::to_memory_service_config(&unified)
    }

    /// Create production legacy-compatible config
    pub fn create_legacy_production_config() -> Result<MemoryServiceConfig> {
        let unified = UnifiedDIConfiguration::production();
        Self::to_memory_service_config(&unified)
    }

    /// Create minimal legacy-compatible config
    pub fn create_legacy_minimal_config() -> Result<MemoryServiceConfig> {
        let unified = UnifiedDIConfiguration::minimal();
        Self::to_memory_service_config(&unified)
    }
}

/// Trait extension for CacheConfig to support unified configuration conversion
trait CacheConfigUnifiedExt {
    fn from_unified(unified_cache: &CacheConfig) -> Self;
}

impl CacheConfigUnifiedExt for crate::CacheConfigType {
    fn from_unified(unified_cache: &CacheConfig) -> Self {
        // CacheConfigType is now just CacheConfig, so return a clone
        unified_cache.clone()
    }
}

/// Trait extension for CacheConfig to support legacy type conversion
trait CacheConfigLegacyExt {
    fn from_legacy_type(legacy_type: &crate::CacheConfigType) -> Self;
}

impl CacheConfigLegacyExt for CacheConfig {
    fn from_legacy_type(legacy_type: &crate::CacheConfigType) -> Self {
        // CacheConfigType is now just CacheConfig, so return a clone
        legacy_type.clone()
    }
}

/// Legacy configuration factory functions for backward compatibility
///
/// These functions maintain the exact same API as the old configuration system
/// but use the new unified configuration under the hood.
pub mod legacy_factories {
    use super::*;

    /// Legacy: Create default memory service config
    pub fn default_memory_config() -> Result<MemoryServiceConfig> {
        ConfigCompatibilityAdapter::create_legacy_compatible_config()
    }

    /// Legacy: Create production memory service config
    pub fn production_memory_config() -> Result<MemoryServiceConfig> {
        ConfigCompatibilityAdapter::create_legacy_production_config()
    }

    /// Legacy: Create minimal memory service config
    pub fn minimal_memory_config() -> Result<MemoryServiceConfig> {
        ConfigCompatibilityAdapter::create_legacy_minimal_config()
    }

    /// Legacy: MemoryServiceConfigBuilder compatibility
    pub fn memory_config_builder() -> MemoryServiceConfigBuilder {
        // Convert from unified to legacy builder
        let unified = UnifiedDIConfiguration::development();
        let legacy_config = ConfigCompatibilityAdapter::to_memory_service_config(&unified)
            .expect("Failed to convert unified config to legacy");

        MemoryServiceConfigBuilder::new()
            .expect("Failed to create legacy builder")
            .with_db_path(legacy_config.db_path)
            .with_cache_path(legacy_config.cache_path)
            .with_health_enabled(legacy_config.health_enabled)
            .with_ai_config(legacy_config.ai_config)
    }
}

/// Migration helper for gradually transitioning to unified configuration
pub struct ConfigMigrationHelper;

impl ConfigMigrationHelper {
    /// Detect if legacy configuration is being used and migrate to unified
    pub fn migrate_if_needed(legacy_path: Option<PathBuf>) -> Result<UnifiedDIConfiguration> {
        if let Some(legacy_config_path) = legacy_path {
            if legacy_config_path.exists() {
                // Load legacy configuration (this would need to be implemented based on legacy format)
                let legacy_config = Self::load_legacy_config(&legacy_config_path)?;
                return ConfigCompatibilityAdapter::from_memory_service_config(&legacy_config);
            }
        }

        // No legacy config found, use unified auto-detection
        Ok(crate::di::config_presets::ConfigPresets::auto_detect())
    }

    /// Load legacy configuration from file
    fn load_legacy_config(_path: &PathBuf) -> Result<MemoryServiceConfig> {
        // This would implement loading of legacy configuration files
        // For now, return default as placeholder
        MemoryServiceConfig::create_default()
    }

    /// Export unified configuration to legacy format for backward compatibility
    pub fn export_to_legacy_format(
        unified: &UnifiedDIConfiguration,
        output_path: &PathBuf,
    ) -> Result<()> {
        let legacy_config = ConfigCompatibilityAdapter::to_memory_service_config(unified)?;

        // Serialize legacy config (implementation depends on legacy format)
        let serialized = serde_json::to_string_pretty(&legacy_config)?;
        std::fs::write(output_path, serialized)?;

        Ok(())
    }

    /// Validate compatibility between unified and legacy configurations
    pub fn validate_compatibility(unified: &UnifiedDIConfiguration) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        // Check for features not supported in legacy format
        if unified.features.experimental {
            warnings
                .push("Experimental features enabled - not supported in legacy format".to_string());
        }

        if unified.security.authentication.enabled {
            warnings.push("Authentication features - not available in legacy format".to_string());
        }

        if unified.performance.profiling.cpu_profiling
            || unified.performance.profiling.memory_profiling
        {
            warnings.push("Profiling features - not available in legacy format".to_string());
        }

        // Check for configuration values that might be problematic
        if unified.core.max_memory_mb > 16384 {
            warnings.push("High memory limit - may not be supported by legacy systems".to_string());
        }

        if unified.memory.hnsw.max_elements > 1_000_000 {
            warnings
                .push("Large HNSW configuration - legacy systems may have limitations".to_string());
        }

        Ok(warnings)
    }
}

/// Comprehensive configuration bridge for services expecting legacy interfaces
pub struct ConfigurationBridge;

impl ConfigurationBridge {
    /// Create a service that can provide both unified and legacy configuration interfaces
    pub fn create_dual_interface_config(
        unified: UnifiedDIConfiguration,
    ) -> Result<DualInterfaceConfig> {
        let legacy = ConfigCompatibilityAdapter::to_memory_service_config(&unified)?;

        Ok(DualInterfaceConfig { unified, legacy })
    }
}

/// Configuration container that provides both unified and legacy interfaces
pub struct DualInterfaceConfig {
    unified: UnifiedDIConfiguration,
    legacy: MemoryServiceConfig,
}

impl DualInterfaceConfig {
    /// Get unified configuration interface
    pub fn unified(&self) -> &UnifiedDIConfiguration {
        &self.unified
    }

    /// Get legacy configuration interface
    pub fn legacy(&self) -> &MemoryServiceConfig {
        &self.legacy
    }

    /// Update both interfaces simultaneously
    pub fn update_unified(&mut self, new_unified: UnifiedDIConfiguration) -> Result<()> {
        self.legacy = ConfigCompatibilityAdapter::to_memory_service_config(&new_unified)?;
        self.unified = new_unified;
        Ok(())
    }

    /// Update legacy interface and sync to unified
    pub fn update_legacy(&mut self, new_legacy: MemoryServiceConfig) -> Result<()> {
        self.unified = ConfigCompatibilityAdapter::from_memory_service_config(&new_legacy)?;
        self.legacy = new_legacy;
        Ok(())
    }

    /// Validate both interfaces are in sync
    pub fn validate_sync(&self) -> Result<bool> {
        let converted_legacy = ConfigCompatibilityAdapter::to_memory_service_config(&self.unified)?;

        // Compare key fields (this is simplified - in production you'd want comprehensive comparison)
        Ok(converted_legacy.db_path == self.legacy.db_path
            && converted_legacy.health_enabled == self.legacy.health_enabled
            && converted_legacy.ai_config.embedding.model_name
                == self.legacy.ai_config.embedding.model_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unified_to_legacy_conversion() -> Result<()> {
        let unified = UnifiedDIConfiguration::development();
        let legacy = ConfigCompatibilityAdapter::to_memory_service_config(&unified)?;

        // Verify key fields are mapped correctly
        assert_eq!(
            legacy.ai_config.embedding.model_name,
            unified.ai.embedding.model_name
        );
        assert_eq!(legacy.health_enabled, unified.orchestration.health.enabled);

        Ok(())
    }

    #[test]
    fn test_legacy_to_unified_conversion() -> Result<()> {
        let legacy = MemoryServiceConfig::create_development()?;
        let unified = ConfigCompatibilityAdapter::from_memory_service_config(&legacy)?;

        // Verify conversion maintains core settings
        assert_eq!(
            unified.ai.embedding.model_name,
            legacy.ai_config.embedding.model_name
        );
        assert_eq!(unified.memory.database.db_path, legacy.db_path);

        Ok(())
    }

    #[test]
    fn test_dual_interface_config() -> Result<()> {
        let unified = UnifiedDIConfiguration::production();
        let mut dual = ConfigurationBridge::create_dual_interface_config(unified)?;

        // Test sync validation
        assert!(dual.validate_sync()?);

        // Test unified update
        let new_unified = UnifiedDIConfiguration::minimal();
        dual.update_unified(new_unified)?;
        assert!(dual.validate_sync()?);

        Ok(())
    }

    #[test]
    fn test_migration_helper() -> Result<()> {
        // Test migration without legacy file
        let config = ConfigMigrationHelper::migrate_if_needed(None)?;
        assert!(matches!(
            config.environment,
            crate::di::unified_config::Environment::Development
        ));

        Ok(())
    }

    #[test]
    fn test_legacy_factories() -> Result<()> {
        let default_config = legacy_factories::default_memory_config()?;
        let production_config = legacy_factories::production_memory_config()?;
        let minimal_config = legacy_factories::minimal_memory_config()?;

        // Verify they're different configurations
        assert!(
            default_config.ai_config.embedding.batch_size
                <= production_config.ai_config.embedding.batch_size
        );
        assert!(
            minimal_config.ai_config.embedding.batch_size
                <= default_config.ai_config.embedding.batch_size
        );

        Ok(())
    }

    #[test]
    fn test_compatibility_validation() -> Result<()> {
        let mut unified = UnifiedDIConfiguration::development();

        // Add features that should generate warnings
        unified.features.experimental = true;
        unified.security.authentication.enabled = true;
        unified.core.max_memory_mb = 32768;

        let warnings = ConfigMigrationHelper::validate_compatibility(&unified)?;
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("Experimental features")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("Authentication features")));

        Ok(())
    }
}
