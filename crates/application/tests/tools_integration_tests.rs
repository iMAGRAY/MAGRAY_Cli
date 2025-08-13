#![allow(unused_imports)]
#![allow(unused_attributes)]
use anyhow::Result;
use application::services::ToolsApplicationService;
use std::collections::HashMap;
use std::sync::Arc;
use tools::ToolRegistry;

/// Интеграционные тесты для Tools functionality
mod tools_integration {
    use super::*;

    /// Создать тестовый Tools Application Service
    fn create_test_tools_service() -> ToolsApplicationService {
        let registry = Arc::new(ToolRegistry::new());
        ToolsApplicationService::new(registry)
    }

    #[tokio::test]
    async fn test_tools_service_list_all_tools() -> Result<()> {
        let service = create_test_tools_service();

        let response = service.list_tools(None, true, None).await?;

        // Проверяем что есть базовые инструменты
        assert!(response.total_count > 0);
        assert!(!response.filtered);

        // Проверяем наличие основных инструментов
        let tool_names: Vec<String> = response.tools.iter().map(|t| t.name.clone()).collect();
        assert!(tool_names.contains(&"file_read".to_string()));
        assert!(tool_names.contains(&"file_write".to_string()));
        assert!(tool_names.contains(&"web_fetch".to_string()));
        assert!(tool_names.contains(&"git_status".to_string()));

        // Проверяем что у инструментов есть детальная информация
        for tool in &response.tools {
            assert!(!tool.name.is_empty());
            assert!(!tool.description.is_empty());
            assert!(tool.usage_guide.is_some()); // include_details = true
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_tools_service_filter_by_name() -> Result<()> {
        let service = create_test_tools_service();

        let response = service
            .list_tools(Some("file".to_string()), true, None)
            .await?;

        assert!(response.filtered);
        assert!(!response.tools.is_empty());

        // Все инструменты должны содержать "file" в имени
        for tool in &response.tools {
            assert!(tool.name.to_lowercase().contains("file"));
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_tools_service_search_functionality() -> Result<()> {
        let service = create_test_tools_service();

        // Поиск по ключевому слову "web"
        let response = service.search_tools("web".to_string()).await?;

        assert!(response.filtered);
        assert!(!response.tools.is_empty());

        // Должны найти web-related инструменты
        let tool_names: Vec<String> = response.tools.iter().map(|t| t.name.clone()).collect();
        assert!(tool_names.iter().any(|name| name.contains("web")));

        Ok(())
    }

    #[tokio::test]
    async fn test_tools_service_get_tool_spec() -> Result<()> {
        let service = create_test_tools_service();

        // Тест существующего инструмента
        let response = service.get_tool_spec("file_read".to_string()).await?;

        assert!(response.found);
        assert!(response.spec.is_some());

        let spec = response.spec.expect("Test operation should succeed");
        assert_eq!(spec.name, "file_read");
        assert!(!spec.description.is_empty());

        // Тест несуществующего инструмента
        let response = service
            .get_tool_spec("non_existent_tool".to_string())
            .await?;

        assert!(!response.found);
        assert!(response.spec.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_tools_service_get_high_risk_tools() -> Result<()> {
        let service = create_test_tools_service();

        let response = service.get_high_risk_tools().await?;

        // Все возвращенные инструменты должны иметь высокий risk_score
        for tool in &response.tools {
            if let Some(guide) = &tool.usage_guide {
                assert!(
                    guide.risk_score >= 4,
                    "Tool '{}' has risk_score {} but should be >= 4",
                    tool.name,
                    guide.risk_score
                );
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_tools_service_count_consistency() -> Result<()> {
        let service = create_test_tools_service();

        // Получаем количество через метод
        let count = service.get_tools_count().await?;

        // Получаем полный список
        let response = service.list_tools(None, false, None).await?;

        // Количества должны совпадать
        assert_eq!(count, response.total_count);
        assert_eq!(response.tools.len(), response.total_count);

        Ok(())
    }

    #[tokio::test]
    async fn test_tools_service_filtering_consistency() -> Result<()> {
        let service = create_test_tools_service();

        // Получаем все инструменты
        let all_tools = service.list_tools(None, true, None).await?;
        let total_count = all_tools.total_count;

        // Фильтруем по имени
        let filtered = service
            .list_tools(Some("git".to_string()), true, None)
            .await?;

        // Отфильтрованный список должен быть меньше или равен общему
        assert!(filtered.total_count <= total_count);
        assert!(filtered.filtered);

        // Проверяем что фильтр действительно применился
        for tool in &filtered.tools {
            assert!(tool.name.to_lowercase().contains("git"));
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_tools_service_details_flag() -> Result<()> {
        let service = create_test_tools_service();

        // Без деталей
        let without_details = service.list_tools(None, false, None).await?;

        // С деталями
        let with_details = service.list_tools(None, true, None).await?;

        // Количество инструментов должно быть одинаковое
        assert_eq!(without_details.total_count, with_details.total_count);

        // Проверяем различия в деталях
        if let (Some(tool_without), Some(tool_with)) =
            (without_details.tools.first(), with_details.tools.first())
        {
            // Без деталей - usage_guide должен быть None
            assert!(tool_without.usage_guide.is_none());

            // С деталями - usage_guide должен присутствовать
            assert!(tool_with.usage_guide.is_some());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_tools_registry_default_tools_available() -> Result<()> {
        let service = create_test_tools_service();
        let response = service.list_tools(None, true, None).await?;

        // Проверяем наличие всех основных категорий инструментов
        let tool_names: Vec<String> = response.tools.iter().map(|t| t.name.clone()).collect();

        // File operations
        assert!(tool_names.contains(&"file_read".to_string()));
        assert!(tool_names.contains(&"file_write".to_string()));
        assert!(tool_names.contains(&"file_delete".to_string()));
        assert!(tool_names.contains(&"dir_list".to_string()));
        assert!(tool_names.contains(&"file_search".to_string()));

        // Git operations
        assert!(tool_names.contains(&"git_status".to_string()));
        assert!(tool_names.contains(&"git_commit".to_string()));
        assert!(tool_names.contains(&"git_diff".to_string()));

        // Web operations
        assert!(tool_names.contains(&"web_search".to_string()));
        assert!(tool_names.contains(&"web_fetch".to_string()));

        // Shell operations
        assert!(tool_names.contains(&"shell_exec".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_register_mcp_tool_workflow() -> Result<()> {
        let service = create_test_tools_service();

        // Попытка регистрации MCP инструмента
        let response = service
            .register_mcp_tool_secure(
                "test_mcp_tool".to_string(),
                "python".to_string(),
                vec!["-m".to_string(), "some_mcp_server".to_string()],
                "remote_tool_name".to_string(),
                "Test MCP tool description".to_string(),
                vec!["/tmp/test".to_string()], // fs_read_roots - limited test access
                vec!["/tmp/output".to_string()], // fs_write_roots - limited test write
                vec!["api.test.com".to_string()], // net_allowlist - test network access
                false,                         // allow_shell - no shell access for tests
                true,                          // supports_dry_run - enable dry run for safety
            )
            .await?;

        // NOTE: Текущая реализация не поддерживает реальную регистрацию
        // из-за Arc<ToolRegistry> vs &mut ToolRegistry проблемы
        // Но тест проверяет что метод работает без паники
        assert_eq!(response.tool_name, "test_mcp_tool");
        assert!(response.registered);

        Ok(())
    }

    #[tokio::test]
    async fn test_register_mcp_tool_secure_validates_shell_permissions() -> Result<()> {
        let service = create_test_tools_service();

        // Попытка регистрации с shell access но без network allowlist - должна быть отклонена
        let response = service
            .register_mcp_tool_secure(
                "unsafe_shell_tool".to_string(),
                "bash".to_string(),
                vec!["-c".to_string(), "dangerous_script.sh".to_string()],
                "shell_tool".to_string(),
                "Dangerous shell tool".to_string(),
                vec!["/".to_string()],    // fs_read_roots - full access
                vec!["/tmp".to_string()], // fs_write_roots
                vec![],                   // net_allowlist - EMPTY - should cause security rejection
                true,                     // allow_shell - HIGH RISK with empty network list
                false,                    // supports_dry_run
            )
            .await?;

        // SECURITY: Должно отклонить регистрацию
        assert_eq!(response.tool_name, "unsafe_shell_tool");
        assert!(!response.registered);
        assert!(response
            .message
            .contains("Shell access requires explicit network allowlist"));

        Ok(())
    }

    #[tokio::test]
    async fn test_register_mcp_tool_secure_allows_safe_shell() -> Result<()> {
        let service = create_test_tools_service();

        // Безопасная регистрация с shell access и explicit network allowlist
        let response = service
            .register_mcp_tool_secure(
                "safe_shell_tool".to_string(),
                "bash".to_string(),
                vec!["-c".to_string(), "safe_script.sh".to_string()],
                "shell_tool".to_string(),
                "Safe shell tool".to_string(),
                vec!["/project/scripts".to_string()], // fs_read_roots - limited
                vec!["/tmp/output".to_string()],      // fs_write_roots - limited
                vec!["api.safe.com".to_string()],     // net_allowlist - explicit
                true, // allow_shell - but with explicit network controls
                true, // supports_dry_run
            )
            .await?;

        // SECURITY: Должно разрешить безопасную регистрацию
        assert_eq!(response.tool_name, "safe_shell_tool");
        assert!(response.registered);
        assert!(response.message.contains("shell: true"));

        Ok(())
    }
}
