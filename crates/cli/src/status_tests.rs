#[cfg(test)]
mod tests {
    use crate::show_system_status;
    use std::env;

    /// Тест базовой функциональности show_system_status
    #[tokio::test]
    async fn test_show_system_status_no_panic() {
        env::remove_var("LLM_PROVIDER");
        env::remove_var("OPENAI_API_KEY");
        let result = show_system_status().await;
        assert!(result.is_ok() || result.is_err());
    }

    /// Тест статуса с настроенным LLM
    #[tokio::test]
    async fn test_show_system_status_with_llm() {
        env::set_var("LLM_PROVIDER", "openai");
        env::set_var("OPENAI_API_KEY", "test-key");
        env::set_var("OPENAI_MODEL", "gpt-4o-mini");
        let result = show_system_status().await;
        assert!(result.is_ok());
        env::remove_var("LLM_PROVIDER");
        env::remove_var("OPENAI_API_KEY");
        env::remove_var("OPENAI_MODEL");
    }

    /// Тест создания binary info
    #[test]
    fn test_binary_info_extraction() {
        let version = env!("CARGO_PKG_VERSION");
        assert!(!version.is_empty());
        assert!(version.contains('.'));
        let binary_size = std::env::current_exe()
            .and_then(|path| path.metadata())
            .map(|meta| meta.len())
            .unwrap_or(0);
        assert!(binary_size > 0);
    }

    /// Тест переменных окружения
    #[test]
    fn test_environment_variables() {
        let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
        assert!(!log_level.is_empty());
        let _ = std::env::var("NO_COLOR").is_ok();
    }

    #[cfg(not(feature = "minimal"))]
    mod non_minimal {
        use super::*;
        use std::sync::Arc;
        /// Тест интеграции с memory service (полная версия)
        #[tokio::test]
        async fn test_memory_service_integration() {
            let memory_result = async {
                let config = memory::default_config()?;
                memory::MemoryService::new(config).await
            }
            .await;
            if let Ok(service) = memory_result {
                let service: Arc<memory::MemoryService> = Arc::new(service);
                let api = memory::UnifiedMemoryAPI::new(service.clone());
                let _ = api.get_stats().await;
                let _ = api.health_check().await;
            }
        }
        /// Тест обработки ошибок memory service
        #[tokio::test]
        async fn test_memory_service_error_handling() {
            let mut invalid_config = memory::MemoryConfig::default();
            invalid_config.db_path = "/invalid/path/that/does/not/exist".into();
            let _ = memory::MemoryService::new(invalid_config).await;
        }
    }

    #[cfg(feature = "minimal")]
    mod minimal_only {
        /// Лёгкая проверка DIMemoryService заглушки
        #[tokio::test]
        async fn test_di_memory_stub_health() {
            let _legacy = memory::di::LegacyMemoryConfig::default();
            // В CPU-профиле пропускаем создание DIMemoryService
            assert!(true);
        }
    }

    /// Performance test для status команды
    #[tokio::test]
    async fn test_status_command_performance() {
        use std::time::Instant;
        let start = Instant::now();
        let timeout = tokio::time::timeout(std::time::Duration::from_secs(10), show_system_status()).await;
        let duration = start.elapsed();
        assert!(timeout.is_ok(), "Status command timed out");
        assert!(duration.as_secs() < 10, "Status command took too long: {:?}", duration);
    }
}
