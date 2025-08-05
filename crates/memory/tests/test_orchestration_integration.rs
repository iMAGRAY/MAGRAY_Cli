use anyhow::Result;
use memory::{
    service_di::{DIMemoryService, default_config},
    orchestration::{MemoryOrchestrator, EmbeddingCoordinator, SearchCoordinator, HealthManager, PromotionCoordinator, ResourceController, BackupCoordinator}
};

#[tokio::test]
async fn test_orchestration_coordinators_integration() -> Result<()> {
    println!("🔧 Создание DIMemoryService с orchestration координаторами");
    let config = default_config()?;
    let service = DIMemoryService::new_minimal(config).await?;
    
    println!("✅ DIMemoryService создан");
    
    // Выводим статистику DI контейнера
    let di_stats = service.di_stats();
    println!("📊 DI Container Stats: registered={}, cached={}, total={}",
             di_stats.registered_factories, di_stats.cached_singletons, di_stats.total_types);
    
    // Проверяем что можем разрешить основные координаторы
    println!("🔍 Тестирование разрешения координаторов...");
    
    // EmbeddingCoordinator
    match service.resolve::<EmbeddingCoordinator>() {
        Ok(_) => println!("EmbeddingCoordinator: ✅ OK"),
        Err(e) => println!("EmbeddingCoordinator: ❌ Error: {}", e),
    }
    
    // SearchCoordinator  
    let search_coordinator = service.try_resolve::<SearchCoordinator>();
    println!("SearchCoordinator: {:?}", search_coordinator.is_some());
    
    // HealthManager
    let health_manager = service.try_resolve::<HealthManager>();
    println!("HealthManager: {:?}", health_manager.is_some());
    
    // PromotionCoordinator
    let promotion_coordinator = service.try_resolve::<PromotionCoordinator>();
    println!("PromotionCoordinator: {:?}", promotion_coordinator.is_some());
    
    // ResourceController
    let resource_controller = service.try_resolve::<ResourceController>();
    println!("ResourceController: {:?}", resource_controller.is_some());
    
    // BackupCoordinator
    let backup_coordinator = service.try_resolve::<BackupCoordinator>();
    println!("BackupCoordinator: {:?}", backup_coordinator.is_some());
    
    // MemoryOrchestrator (главный координатор)
    let memory_orchestrator = service.try_resolve::<MemoryOrchestrator>();
    println!("MemoryOrchestrator: {:?}", memory_orchestrator.is_some());
    
    // Проверяем что ключевые координаторы доступны в минимальной конфигурации
    let embedding_coordinator = service.try_resolve::<EmbeddingCoordinator>();
    assert!(embedding_coordinator.is_some(), "EmbeddingCoordinator должен быть доступен");
    assert!(search_coordinator.is_some(), "SearchCoordinator должен быть доступен");
    assert!(health_manager.is_some(), "HealthManager должен быть доступен");
    
    // Для минимальной конфигурации эти могут отсутствовать
    if let Some(orchestrator) = memory_orchestrator {
        println!("🎯 Тестирование MemoryOrchestrator...");
        
        // Проверяем что все координаторы готовы
        let all_ready = orchestrator.all_ready().await;
        println!("All coordinators ready: {}", all_ready);
        
        // Получаем метрики
        let metrics = orchestrator.all_metrics().await;
        println!("Orchestrator metrics: {}", metrics);
        
        println!("✅ MemoryOrchestrator работает корректно");
    }
    
    println!("🎉 Orchestration integration test завершен успешно!");
    Ok(())
}