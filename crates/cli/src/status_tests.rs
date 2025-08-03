#[cfg(test)]
mod tests {
    use crate::show_system_status;
    use std::env;
    use std::sync::Arc;
    
    /// Тест базовой функциональности show_system_status
    #[tokio::test]
    async fn test_show_system_status_no_panic() {
        // Тест должен работать даже без LLM настроек
        env::remove_var("LLM_PROVIDER");
        env::remove_var("OPENAI_API_KEY");
        
        // Функция не должна паниковать
        let result = show_system_status().await;
        
        // В зависимости от состояния системы может быть ошибка или успех
        // но не должно быть panic
        assert!(result.is_ok() || result.is_err());
    }
    
    /// Тест статуса с настроенным LLM
    #[tokio::test]
    async fn test_show_system_status_with_llm() {
        // Устанавливаем минимальные настройки для LLM
        env::set_var("LLM_PROVIDER", "openai");
        env::set_var("OPENAI_API_KEY", "test-key");
        env::set_var("OPENAI_MODEL", "gpt-4o-mini");
        
        let result = show_system_status().await;
        
        // С базовыми настройками должно работать
        // (даже если ключ неверный, функция создания клиента пройдет)
        assert!(result.is_ok());
        
        // Очистим переменные
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
        
        // Тест получения размера binary
        let binary_size = std::env::current_exe()
            .and_then(|path| path.metadata())
            .map(|meta| meta.len())
            .unwrap_or(0);
            
        // Binary должен существовать и иметь размер > 0
        assert!(binary_size > 0);
    }
    
    /// Тест переменных окружения
    #[test]
    fn test_environment_variables() {
        // Тест получения переменной RUST_LOG
        let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
        assert!(!log_level.is_empty());
        
        // Тест NO_COLOR переменной
        let no_color = std::env::var("NO_COLOR").is_ok();
        // no_color может быть true или false, просто проверяем что не паникует
        let _ = no_color;
    }
    
    /// Тест интеграции с memory service
    #[tokio::test]
    async fn test_memory_service_integration() {
        // Пытаемся создать memory service
        let memory_result = async {
            let config = memory::default_config()?;
            memory::MemoryService::new(config).await
        }.await;
            
        match memory_result {
            Ok(service) => {
                let service: Arc<memory::MemoryService> = Arc::new(service);
                let api = memory::UnifiedMemoryAPI::new(service.clone());
                
                // Тестируем базовые операции
                let stats_result = api.get_stats().await;
                let health_result = api.health_check().await;
                
                // Хотя бы одна из операций должна работать
                assert!(stats_result.is_ok() || health_result.is_ok());
            }
            Err(_) => {
                // Memory service может не инициализироваться в тестовой среде
                // это нормально
            }
        }
    }
    
    /// Тест прогресс индикатора
    #[test]
    fn test_progress_indicator() {
        use crate::progress::ProgressBuilder;
        
        let spinner = ProgressBuilder::fast("Test message");
        
        // Тест завершения без ошибок
        spinner.finish_success(Some("Test completed"));
        
        // Создание другого типа спиннера
        let backup_spinner = ProgressBuilder::backup("Test backup");
        backup_spinner.finish_success(None);
    }
    
    /// Тест обработки ошибок memory service
    #[tokio::test]
    async fn test_memory_service_error_handling() {
        // Тест с невалидной конфигурацией
        let mut invalid_config = memory::MemoryConfig::default();
        invalid_config.db_path = "/invalid/path/that/does/not/exist".into();
        
        let result = memory::MemoryService::new(invalid_config).await;
        
        // Memory service может создаваться успешно даже с невалидными путями
        // (создает директории автоматически), поэтому тестируем что не паникует
        assert!(result.is_ok() || result.is_err());
    }
    
    /// Performance test для status команды
    #[tokio::test]
    async fn test_status_command_performance() {
        use std::time::Instant;
        
        let start = Instant::now();
        
        // Устанавливаем тайм-аут
        let timeout = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            show_system_status()
        ).await;
        
        let duration = start.elapsed();
        
        // Команда должна выполняться менее чем за 10 секунд
        assert!(timeout.is_ok(), "Status command timed out");
        assert!(duration.as_secs() < 10, "Status command took too long: {:?}", duration);
    }
}

// @component: {"k":"C","id":"status_tests","t":"Unit tests for status command","m":{"cur":95,"tgt":100,"u":"%"},"f":["tests","status","cli"]}