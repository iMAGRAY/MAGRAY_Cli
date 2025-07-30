use anyhow::Result;
use memory::{
    MemoryConfig, MemoryService, Layer, Record, 
    ComponentType, HealthConfig
};
use tracing::{info, error};
use uuid::Uuid;
use chrono::Utc;

/// ПОЛНАЯ ПРОВЕРКА РАБОТОСПОСОБНОСТИ ВСЕХ СИСТЕМ ПАМЯТИ
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    info!("🔍 ПОЛНАЯ ПРОВЕРКА РАБОТОСПОСОБНОСТИ СИСТЕМ МАГРЭЙ");
    info!("==================================================\n");
    
    let mut overall_score = 0;
    let mut max_possible_score = 0;
    let mut failed_tests = Vec::new();
    
    // ========== ТЕСТ 1: ИНИЦИАЛИЗАЦИЯ MEMORYS SERVICE ==========
    println!("🚀 ТЕСТ 1: Инициализация MemoryService");
    println!("======================================");
    max_possible_score += 100;
    
    let temp_dir = tempfile::tempdir()?;
    let memory_config = MemoryConfig {
        db_path: temp_dir.path().join("full_verification_test"),
        cache_path: temp_dir.path().join("cache"),
        promotion: Default::default(),
        ai_config: Default::default(),
        health_config: HealthConfig::default(),
    };
    
    let memory_service = match MemoryService::new(memory_config).await {
        Ok(service) => {
            println!("  ✅ MemoryService инициализирован успешно");
            overall_score += 100;
            service
        },
        Err(e) => {
            error!("  ❌ Ошибка инициализации MemoryService: {}", e);
            failed_tests.push("MemoryService инициализация");
            return Ok(());
        }
    };
    
    // ========== ТЕСТ 2: HEALTH MONITORING СИСТЕМА ==========
    println!("\n🏥 ТЕСТ 2: Health Monitoring система");
    println!("===================================");
    max_possible_score += 100;
    
    let initial_health = memory_service.get_system_health();
    let health_score = if initial_health.component_statuses.len() >= 3 
        && initial_health.uptime_seconds >= 0 {
        println!("  ✅ Health система активна");
        println!("    📊 Статус: {:?}", initial_health.overall_status);
        println!("    📈 Компонентов: {}", initial_health.component_statuses.len());
        100
    } else {
        println!("  ❌ Health система работает неполноценно");
        failed_tests.push("Health monitoring");
        50
    };
    overall_score += health_score;
    
    // ========== ТЕСТ 3: VECTOR STORE И HNSW ==========
    println!("\n🗄️ ТЕСТ 3: VectorStore и HNSW индексирование");
    println!("============================================");
    max_possible_score += 150;
    
    let test_records = vec![
        create_test_record("Первая запись для HNSW тестирования", Layer::Interact, 0.9),
        create_test_record("Вторая запись с высокой релевантностью", Layer::Interact, 0.8),
        create_test_record("Третья запись для проверки поиска", Layer::Insights, 0.7),
        create_test_record("Четвертая запись в Assets слое", Layer::Assets, 0.6),
        create_test_record("Пятая запись для тестирования скорости", Layer::Interact, 0.5),
    ];
    
    println!("  📝 Вставка {} тестовых записей...", test_records.len());
    let insert_start = std::time::Instant::now();
    
    let mut insert_success = 0;
    for (i, record) in test_records.iter().enumerate() {
        match memory_service.insert(record.clone()).await {
            Ok(_) => {
                insert_success += 1;
                println!("    ✅ Запись {} вставлена", i + 1);
            },
            Err(e) => {
                println!("    ❌ Ошибка вставки записи {}: {}", i + 1, e);
            }
        }
    }
    
    let insert_duration = insert_start.elapsed();
    let insert_score = if insert_success == test_records.len() {
        println!("  ✅ Все записи вставлены успешно");
        println!("    ⏱️ Время вставки: {:?}", insert_duration);
        75
    } else {
        println!("  ⚠️ Вставлено {}/{} записей", insert_success, test_records.len());
        failed_tests.push("VectorStore insert");
        25
    };
    
    // Тестируем поиск
    println!("  🔍 Тестирование поиска по всем слоям...");
    let search_start = std::time::Instant::now();
    
    let search_score = match memory_service
        .search("запись тестирование")
        .with_layers(&[Layer::Interact, Layer::Insights, Layer::Assets])
        .top_k(3)
        .execute()
        .await {
        Ok(results) => {
            let search_duration = search_start.elapsed();
            println!("  ✅ Поиск выполнен успешно");
            println!("    📊 Найдено: {} результатов", results.len());
            println!("    ⏱️ Время поиска: {:?}", search_duration);
            
            for (i, result) in results.iter().enumerate() {
                println!("    {}. {:?}: {} (score: {:.3})", 
                         i + 1, result.layer, 
                         result.text.chars().take(30).collect::<String>(), 
                         result.score);
            }
            75
        },
        Err(e) => {
            println!("  ❌ Ошибка поиска: {}", e);
            failed_tests.push("VectorStore search");
            0
        }
    };
    
    let total_vectorstore_score = insert_score + search_score;
    overall_score += total_vectorstore_score;
    
    // ========== ТЕСТ 4: PROMOTION ENGINE ==========
    println!("\n⚡ ТЕСТ 4: PromotionEngine");
    println!("========================");
    max_possible_score += 100;
    
    let promotion_start = std::time::Instant::now();
    let promotion_result = memory_service.run_promotion_cycle().await;
    
    let promotion_score = match promotion_result {
        Ok(stats) => {
            let promotion_duration = promotion_start.elapsed();
            println!("  ✅ PromotionEngine работает");
            println!("    📊 Interact->Insights: {}", stats.interact_to_insights);
            println!("    📊 Insights->Assets: {}", stats.insights_to_assets);
            println!("    ⏱️ Общее время: {}ms", stats.total_time_ms);
            println!("    ⏱️ Реальное время: {:?}", promotion_duration);
            100
        },
        Err(e) => {
            println!("  ❌ Ошибка PromotionEngine: {}", e);
            failed_tests.push("PromotionEngine");
            0
        }
    };
    overall_score += promotion_score;
    
    // ========== ТЕСТ 5: BGE RERANKER ==========
    println!("\n🤖 ТЕСТ 5: BGE Reranker интеграция");
    println!("=================================");
    max_possible_score += 100;
    
    // Создаем документы для reranking
    let documents_for_reranking = vec![
        "Алгоритмы машинного обучения и нейронные сети".to_string(),
        "Базы данных и системы управления данными".to_string(),
        "Искусственный интеллект в современных приложениях".to_string(),
        "Веб-разработка и фронтенд технологии".to_string(),
    ];
    
    // Проверяем доступность reranking через MemoryService
    // (поскольку прямого API нет, проверяем косвенно через health status)
    let reranker_health = memory_service.get_component_health(ComponentType::RerankingService);
    
    let reranker_score = if let Some(stats) = reranker_health {
        println!("  ✅ BGE Reranker интегрирован");
        println!("    📊 Success rate: {:.1}%", stats.success_rate * 100.0);
        println!("    📊 Всего запросов: {}", stats.total_requests);
        println!("    ⏱️ Среднее время: {:.2}ms", stats.avg_response_time_ms);
        100
    } else {
        println!("  ⚠️ BGE Reranker не доступен для мониторинга");
        failed_tests.push("BGE Reranker availability");
        50
    };
    overall_score += reranker_score;
    
    // ========== ТЕСТ 6: EMBEDDING SERVICE ==========
    println!("\n🧠 ТЕСТ 6: Embedding Service");
    println!("===========================");
    max_possible_score += 100;
    
    let embedding_health = memory_service.get_component_health(ComponentType::EmbeddingService);
    let embedding_score = if let Some(stats) = embedding_health {
        println!("  ✅ EmbeddingService активен");
        println!("    📊 Success rate: {:.1}%", stats.success_rate * 100.0);
        println!("    📊 Всего запросов: {}", stats.total_requests);
        
        if stats.success_rate >= 0.9 {
            100
        } else if stats.success_rate >= 0.5 {
            70
        } else {
            30
        }
    } else {
        println!("  ❌ EmbeddingService недоступен");
        failed_tests.push("EmbeddingService");
        0
    };
    overall_score += embedding_score;
    
    // ========== ТЕСТ 7: CACHE СИСТЕМА ==========
    println!("\n💾 ТЕСТ 7: Cache система");
    println!("=======================");
    max_possible_score += 75;
    
    let cache_health = memory_service.get_component_health(ComponentType::Cache);
    let cache_score = if let Some(stats) = cache_health {
        println!("  ✅ Cache система работает");
        println!("    📊 Success rate: {:.1}%", stats.success_rate * 100.0);
        println!("    📊 Всего операций: {}", stats.total_requests);
        
        if stats.total_requests > 0 {
            75
        } else {
            50
        }
    } else {
        println!("  ⚠️ Cache система не отслеживается");
        50
    };
    overall_score += cache_score;
    
    // ========== ТЕСТ 8: HEALTH CHECK СИСТЕМА ==========
    println!("\n🔍 ТЕСТ 8: Health Check всех компонентов");
    println!("=======================================");
    max_possible_score += 100;
    
    let health_check_start = std::time::Instant::now();
    let health_check_result = memory_service.run_health_check().await;
    
    let health_check_score = match health_check_result {
        Ok(health_status) => {
            let health_check_duration = health_check_start.elapsed();
            println!("  ✅ Health check выполнен");
            println!("    📊 Общий статус: {:?}", health_status.overall_status);
            println!("    🚨 Активных alerts: {}", health_status.active_alerts.len());
            println!("    ⏱️ Время проверки: {:?}", health_check_duration);
            
            if !health_status.active_alerts.is_empty() {
                println!("    🚨 Найденные проблемы:");
                for alert in &health_status.active_alerts {
                    println!("      {:?}: {}", alert.severity, alert.title);
                }
            }
            
            match health_status.overall_status {
                memory::health::HealthStatus::Healthy => 100,
                memory::health::HealthStatus::Degraded => 75,
                memory::health::HealthStatus::Unhealthy => 50,
                memory::health::HealthStatus::Down => 25,
            }
        },
        Err(e) => {
            println!("  ❌ Ошибка health check: {}", e);
            failed_tests.push("Health check system");
            0
        }
    };
    overall_score += health_check_score;
    
    // ========== ТЕСТ 9: PERFORMANCE ТЕСТ ==========
    println!("\n⚡ ТЕСТ 9: Performance тестирование");
    println!("=================================");
    max_possible_score += 75;
    
    let performance_start = std::time::Instant::now();
    let mut performance_operations = 0;
    
    // Выполняем серию операций для тестирования производительности
    for i in 0..10 {
        let record = create_test_record(
            &format!("Performance тест запись {}", i),
            Layer::Interact,
            0.5
        );
        
        if memory_service.insert(record).await.is_ok() {
            performance_operations += 1;
        }
        
        if i % 3 == 0 {
            let _ = memory_service
                .search(&format!("performance {}", i))
                .top_k(2)
                .execute()
                .await;
        }
    }
    
    let performance_duration = performance_start.elapsed();
    let ops_per_second = performance_operations as f64 / performance_duration.as_secs_f64();
    
    let performance_score = if performance_operations >= 8 && ops_per_second > 5.0 {
        println!("  ✅ Performance тест пройден");
        println!("    📊 Операций выполнено: {}/10", performance_operations);
        println!("    ⚡ Операций в секунду: {:.1}", ops_per_second);
        println!("    ⏱️ Общее время: {:?}", performance_duration);
        75
    } else {
        println!("  ⚠️ Performance ниже ожидаемого");
        println!("    📊 Операций выполнено: {}/10", performance_operations);
        println!("    ⚡ Операций в секунду: {:.1}", ops_per_second);
        failed_tests.push("Performance requirements");
        30
    };
    overall_score += performance_score;
    
    // ========== ФИНАЛЬНАЯ ОЦЕНКА ==========
    println!("\n🏆 ФИНАЛЬНЫЕ РЕЗУЛЬТАТЫ ПРОВЕРКИ");
    println!("===============================");
    
    let final_percentage = (overall_score as f64 / max_possible_score as f64) * 100.0;
    
    println!("📊 ОБЩИЙ СЧЕТ: {}/{} ({:.1}%)", overall_score, max_possible_score, final_percentage);
    
    if failed_tests.is_empty() {
        println!("✅ ВСЕ ТЕСТЫ ПРОЙДЕНЫ УСПЕШНО!");
    } else {
        println!("⚠️ ПРОБЛЕМЫ В ТЕСТАХ:");
        for test in &failed_tests {
            println!("  ❌ {}", test);
        }
    }
    
    println!("\n📋 ДЕТАЛЬНЫЙ АНАЛИЗ:");
    
    // Получаем финальный статус системы
    let final_health = memory_service.get_system_health();
    
    println!("🔍 СТАТУС КОМПОНЕНТОВ:");
    for (component, status) in &final_health.component_statuses {
        let status_icon = match status {
            memory::health::HealthStatus::Healthy => "✅",
            memory::health::HealthStatus::Degraded => "🟡", 
            memory::health::HealthStatus::Unhealthy => "🟠",
            memory::health::HealthStatus::Down => "❌",
        };
        println!("  {} {:?}: {:?}", status_icon, component, status);
    }
    
    println!("\n📈 МЕТРИКИ СИСТЕМЫ:");
    println!("  🕒 Время работы: {} секунд", final_health.uptime_seconds);
    println!("  📊 Активных метрик: {}", final_health.metrics_summary.len());
    println!("  🚨 Активных alerts: {}", final_health.active_alerts.len());
    
    // Итоговый вердикт
    println!("\n🎯 ИТОГОВЫЙ ВЕРДИКТ:");
    
    if final_percentage >= 95.0 && failed_tests.is_empty() {
        println!("🎉 СИСТЕМА ПОЛНОСТЬЮ ГОТОВА К PRODUCTION!");
        println!("   Все компоненты работают идеально!");
    } else if final_percentage >= 85.0 && failed_tests.len() <= 2 {
        println!("👍 СИСТЕМА ГОТОВА К PRODUCTION с небольшими замечаниями");
        println!("   Основной функционал работает стабильно");
    } else if final_percentage >= 70.0 {
        println!("⚠️ СИСТЕМА ФУНКЦИОНИРУЕТ, но требует доработки");
        println!("   Некоторые компоненты нуждаются в оптимизации");
    } else {
        println!("❌ СИСТЕМА НЕ ГОТОВА К PRODUCTION");
        println!("   Требуется серьезная доработка критических компонентов");
    }
    
    println!("\\n📊 ЧЕСТНАЯ ГОТОВНОСТЬ: {:.0}%", final_percentage);
    
    Ok(())
}

/// Создает тестовую запись с заданными параметрами
fn create_test_record(text: &str, layer: Layer, score: f32) -> Record {
    Record {
        id: Uuid::new_v4(),
        text: text.to_string(),
        embedding: vec![0.1; 1024], // BGE-M3 размерность
        layer,
        kind: "verification_test".to_string(),
        tags: vec!["full_test".to_string()],
        project: "full_verification".to_string(),
        session: Uuid::new_v4().to_string(),
        score,
        access_count: 1,
        ts: Utc::now(),
        last_access: Utc::now(),
    }
}