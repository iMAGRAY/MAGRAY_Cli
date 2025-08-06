//! DI Configuration для Services Layer
//! 
//! Регистрация всех сервисов в DI контейнере с правильными зависимостями
//! и жизненными циклами. Интеграция с существующим memory DI контейнером.

use anyhow::Result;
use std::sync::Arc;
use memory::{DIContainer, Lifetime};
use llm::{LlmClient, IntentAnalyzerAgent};
use router::SmartRouter;
use memory::DIMemoryService;

use super::{
    IntentAnalysisService, RequestRoutingService, LlmCommunicationService,
    ResilienceService, ServiceOrchestrator,
    intent_analysis::{create_intent_analysis_service, DefaultIntentAnalysisService},
    request_routing::{create_request_routing_service, DefaultRequestRoutingService},
    llm_communication::{create_llm_communication_service, DefaultLlmCommunicationService},
    resilience::{create_resilience_service, DefaultResilienceService},
    orchestrator::{create_service_orchestrator, DefaultServiceOrchestrator},
};

/// Регистрация всех сервисов в DI контейнере
pub fn register_services(container: &DIContainer, llm_client: LlmClient) -> Result<()> {
    tracing::info!("🔧 Регистрация Services Layer в DI контейнере");
    
    // 1. Регистрируем базовые зависимости
    container.register_instance(llm_client.clone())?;
    
    // 2. Регистрируем Intent Analysis Service
    container.register(
        |container| {
            let llm_client = container.resolve::<LlmClient>()?;
            let intent_analyzer = IntentAnalyzerAgent::new(llm_client.clone());
            Ok(create_intent_analysis_service(intent_analyzer))
        },
        Lifetime::Singleton,
    )?;
    
    // Добавляем информацию о зависимости
    container.add_dependency_info::<Arc<dyn IntentAnalysisService>, LlmClient>()?;
    
    // 3. Регистрируем Request Routing Service  
    container.register(
        |container| {
            let llm_client = container.resolve::<LlmClient>()?;
            let smart_router = SmartRouter::new(llm_client.clone());
            Ok(create_request_routing_service(smart_router))
        },
        Lifetime::Singleton,
    )?;
    
    container.add_dependency_info::<Arc<dyn RequestRoutingService>, LlmClient>()?;
    
    // 4. Регистрируем LLM Communication Service
    container.register(
        |container| {
            let llm_client = container.resolve::<LlmClient>()?;
            Ok(create_llm_communication_service(llm_client.clone()))
        },
        Lifetime::Singleton,
    )?;
    
    container.add_dependency_info::<Arc<dyn LlmCommunicationService>, LlmClient>()?;
    
    // 5. Регистрируем Resilience Service (без зависимостей)
    container.register(
        |_container| {
            Ok(create_resilience_service())
        },
        Lifetime::Singleton,
    )?;
    
    // 6. Регистрируем Service Orchestrator (зависит от всех остальных)
    container.register(
        |container| {
            let intent_analysis = container.resolve::<Arc<dyn IntentAnalysisService>>()?;
            let request_routing = container.resolve::<Arc<dyn RequestRoutingService>>()?;
            let llm_communication = container.resolve::<Arc<dyn LlmCommunicationService>>()?;
            let resilience = container.resolve::<Arc<dyn ResilienceService>>()?;
            let memory_service = container.resolve::<DIMemoryService>()?;
            
            Ok(create_service_orchestrator(
                intent_analysis,
                request_routing,
                llm_communication,
                resilience,
                memory_service,
            ))
        },
        Lifetime::Singleton,
    )?;
    
    // Добавляем информацию о зависимостях оркестратора
    container.add_dependency_info::<Arc<dyn ServiceOrchestrator>, Arc<dyn IntentAnalysisService>>()?;
    container.add_dependency_info::<Arc<dyn ServiceOrchestrator>, Arc<dyn RequestRoutingService>>()?;
    container.add_dependency_info::<Arc<dyn ServiceOrchestrator>, Arc<dyn LlmCommunicationService>>()?;
    container.add_dependency_info::<Arc<dyn ServiceOrchestrator>, Arc<dyn ResilienceService>>()?;
    container.add_dependency_info::<Arc<dyn ServiceOrchestrator>, DIMemoryService>()?;
    
    tracing::info!("✅ Services Layer зарегистрированы в DI контейнере");
    Ok(())
}

/// Создать полностью настроенный DI контейнер с services layer
pub async fn create_services_container() -> Result<DIContainer> {
    use memory::{DIContainerBuilder, default_config};
    
    tracing::info!("🏗️ Создание DI контейнера с Services Layer");
    
    // Создаем базовый контейнер
    let container = DIContainerBuilder::new().build()?;
    
    // Регистрируем memory сервисы
    let memory_config = default_config()?;
    let memory_service = DIMemoryService::new(memory_config).await?;
    container.register_instance(memory_service)?;
    
    // Создаем LLM клиент
    let llm_client = LlmClient::from_env()?;
    
    // Регистрируем все сервисы
    register_services(&container, llm_client)?;  
    
    // Валидируем зависимости
    container.validate_dependencies()?;
    
    tracing::info!("✅ DI контейнер с Services Layer готов");
    Ok(container)
}

/// Вспомогательные функции для тестирования
#[cfg(test)]
pub mod test_helpers {
    use super::*;
    use memory::DIMemoryService;
    
    pub async fn create_test_container() -> Result<DIContainer> {
        let container = DIContainerBuilder::new().build()?;
        
        // Создаем mock memory service для тестов
        let memory_config = default_config()?;
        let memory_service = DIMemoryService::new(memory_config).await?;
        container.register_instance(memory_service)?;
        
        // Создаем test LLM client
        let llm_client = LlmClient::from_env()?;
        
        register_services(&container, llm_client)?;
        container.validate_dependencies()?;
        
        Ok(container)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_services_di_registration() -> Result<()> {
        let container = test_helpers::create_test_container().await?;
        
        // Проверяем что все сервисы зарегистрированы
        assert!(container.is_registered::<Arc<dyn IntentAnalysisService>>());
        assert!(container.is_registered::<Arc<dyn RequestRoutingService>>());
        assert!(container.is_registered::<Arc<dyn LlmCommunicationService>>());
        assert!(container.is_registered::<Arc<dyn ResilienceService>>());
        assert!(container.is_registered::<Arc<dyn ServiceOrchestrator>>());
        
        // Проверяем что можем их разрешить
        let _intent_service = container.resolve::<Arc<dyn IntentAnalysisService>>()?;
        let _routing_service = container.resolve::<Arc<dyn RequestRoutingService>>()?;
        let _llm_service = container.resolve::<Arc<dyn LlmCommunicationService>>()?;
        let _resilience_service = container.resolve::<Arc<dyn ResilienceService>>()?;
        let _orchestrator = container.resolve::<Arc<dyn ServiceOrchestrator>>()?;
        
        Ok(())
    }
    
    #[tokio::test] 
    async fn test_orchestrator_dependencies() -> Result<()> {
        let container = test_helpers::create_test_container().await?;
        
        // Разрешаем оркестратор - должен получить все свои зависимости
        let orchestrator = container.resolve::<Arc<dyn ServiceOrchestrator>>()?;
        
        // Проверяем базовую функциональность
        let health = orchestrator.health_check().await;
        assert!(!health.service_statuses.is_empty());
        
        let stats = orchestrator.get_orchestrator_stats().await;
        assert_eq!(stats.total_requests, 0);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_no_circular_dependencies() -> Result<()> {
        let container = test_helpers::create_test_container().await?;
        
        // Валидация должна пройти без ошибок циркулярных зависимостей
        container.validate_dependencies()?;
        
        // Проверяем что циклов нет
        let cycles = container.get_dependency_cycles();
        assert!(cycles.is_empty(), "Обнаружены циклические зависимости: {:?}", cycles);
        
        Ok(())
    }
}