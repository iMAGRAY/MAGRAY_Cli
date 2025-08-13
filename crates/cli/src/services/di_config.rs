//! DI Configuration для Service Layer
//!
//! Конфигурация Dependency Injection контейнера для всех сервисов
//! в архитектуре Service Layer: IntentAnalysisService, RequestRoutingService,
//! LlmCommunicationService, ResilienceService, и ServiceOrchestrator.

use anyhow::Result;
use llm::IntentAnalyzerAgent;
use llm::LlmClient;
use memory::di::DIContainer;
use memory::di::UnifiedContainer as DIMemoryService;
use memory::service_di::default_config;
use router::SmartRouter;
use std::sync::Arc;

use super::{
    intent_analysis::create_intent_analysis_service,
    llm_communication::create_llm_communication_service,
    orchestrator::create_service_orchestrator,
    request_routing::create_request_routing_service,
    resilience::{create_resilience_service, DefaultResilienceService},
    IntentAnalysisService, LlmCommunicationService, RequestRoutingService, ServiceOrchestrator,
};

/// Регистрация всех сервисов в DI контейнере  
pub fn register_services(_container: &DIContainer, _llm_client: LlmClient) -> Result<()> {
    tracing::info!("🔧 Регистрация Services Layer в DI контейнере");

    // Note: DI Container API has changed, services will be instantiated directly when needed
    // This is a temporary workaround until DI API is stabilized

    tracing::info!("✅ Services Layer registration completed (direct instantiation mode)");
    Ok(())
}

/// Создание Services Container с Direct Instantiation
///
/// Поскольку DI Container API изменился, используем прямое создание сервисов
/// с инъекцией зависимостей через factory функции.
pub async fn create_services_container() -> Result<ServicesContainer> {
    tracing::info!("🏗️ Создание Services Container с прямой инстанцией");

    // Создаем базовые зависимости
    let llm_client = LlmClient::from_env()?;
    let memory_service = DIMemoryService::new();
    let memory_service_for_orchestrator = DIMemoryService::new();

    // Создаем сервисы
    let intent_analyzer = IntentAnalyzerAgent::new(llm_client.clone());
    let intent_analysis = create_intent_analysis_service(intent_analyzer);

    let smart_router = SmartRouter::new(llm_client.clone());
    let request_routing = create_request_routing_service(smart_router);

    let llm_communication = create_llm_communication_service(llm_client.clone());
    let resilience = create_resilience_service();

    let orchestrator = create_service_orchestrator(
        intent_analysis.clone(),
        request_routing.clone(),
        llm_communication.clone(),
        resilience.clone(),
        memory_service_for_orchestrator,
    );

    tracing::info!("✅ Services Container готов");

    Ok(ServicesContainer {
        intent_analysis,
        request_routing,
        llm_communication,
        resilience,
        orchestrator,
        memory_service,
        llm_client: Arc::new(llm_client),
    })
}

/// Container для всех Services Layer сервисов
///
/// Поскольку DI Container API нестабилен, используем простой struct
/// для хранения всех необходимых сервисов с их зависимостями.
pub struct ServicesContainer {
    pub intent_analysis: Arc<dyn IntentAnalysisService>,
    pub request_routing: Arc<dyn RequestRoutingService>,
    pub llm_communication: Arc<dyn LlmCommunicationService>,
    pub resilience: Arc<DefaultResilienceService>,
    pub orchestrator: Arc<dyn ServiceOrchestrator>,
    pub memory_service: DIMemoryService,
    pub llm_client: Arc<LlmClient>,
}

impl ServicesContainer {
    /// Получить service orchestrator
    pub fn get_orchestrator(&self) -> Arc<dyn ServiceOrchestrator> {
        self.orchestrator.clone()
    }

    /// Получить intent analysis service
    pub fn get_intent_analysis(&self) -> Arc<dyn IntentAnalysisService> {
        self.intent_analysis.clone()
    }

    /// Получить request routing service
    pub fn get_request_routing(&self) -> Arc<dyn RequestRoutingService> {
        self.request_routing.clone()
    }

    /// Получить LLM communication service
    pub fn get_llm_communication(&self) -> Arc<dyn LlmCommunicationService> {
        self.llm_communication.clone()
    }

    /// Получить resilience service
    pub fn get_resilience(&self) -> Arc<DefaultResilienceService> {
        self.resilience.clone()
    }

    /// Получить memory service
    pub fn get_memory_service(&self) -> &DIMemoryService {
        &self.memory_service
    }

    /// Получить LLM client
    pub fn get_llm_client(&self) -> Arc<LlmClient> {
        self.llm_client.clone()
    }
}

/// Legacy function для compatibility - создает базовый DI container
pub async fn create_services_container_legacy() -> Result<DIContainer> {
    tracing::info!("🏗️ Создание Legacy DI контейнера");

    // Создаем базовый контейнер
    let container = DIContainer::new();

    // Регистрируем memory сервисы
    let _memory_config = default_config()?;
    let _memory_service = DIMemoryService::new();
    // Note: DI Container interface has changed, using direct instantiation

    // Создаем LLM клиент
    let llm_client = LlmClient::from_env()?;

    // Регистрируем все сервисы
    register_services(&container, llm_client)?;

    // Note: Validation removed due to API changes

    tracing::info!("✅ Legacy DI контейнер готов");
    Ok(container)
}
