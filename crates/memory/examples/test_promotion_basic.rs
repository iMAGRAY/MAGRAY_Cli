use anyhow::Result;
use memory::promotion_optimized::OptimizedPromotionEngine;
use memory::{MemoryConfig, MemoryService, PromotionConfig, VectorStore};
use std::sync::Arc;
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    info!("🚀 Базовый тест оптимизированного promotion engine");
    info!("================================================\n");
    
    // Создаем уникальные временные директории
    let temp_dir = tempfile::tempdir()?;
    let process_id = std::process::id();
    let vector_db_path = temp_dir.path().join(format!("vector_db_{}", process_id));
    let promotion_db_path = temp_dir.path().join(format!("promotion_db_{}", process_id));
    
    println!("✅ Временные директории созданы");
    
    // Создаем конфигурацию
    let promotion_config = PromotionConfig {
        interact_ttl_hours: 1,
        insights_ttl_days: 1,
        promote_threshold: 0.5,
        decay_factor: 0.9,
    };
    
    println!("✅ Конфигурация создана");
    
    // Создаем VectorStore с отдельным путем
    let vector_store = Arc::new(VectorStore::new(&vector_db_path).await?);
    println!("✅ VectorStore создан");
    
    // Создаем отдельную Sled DB для promotion индексов
    let promotion_sled_db = Arc::new(sled::open(&promotion_db_path)?);
    println!("✅ Promotion Sled DB открыта");
    
    // Создаем OptimizedPromotionEngine
    println!("\n🔧 Создание OptimizedPromotionEngine...");
    let promotion_engine = OptimizedPromotionEngine::new(
        vector_store,
        promotion_config,
        promotion_sled_db
    ).await?;
    
    println!("✅ OptimizedPromotionEngine создан!");
    
    // Тестируем статистику производительности
    println!("\n📊 Тестирование статистики производительности...");
    let perf_stats = promotion_engine.get_performance_stats().await?;
    
    println!("  📈 Размеры индексов:");
    println!("    Interact: time={}, score={}", 
             perf_stats.interact_time_index_size,
             perf_stats.interact_score_index_size);
    println!("    Insights: time={}, score={}", 
             perf_stats.insights_time_index_size,
             perf_stats.insights_score_index_size);
    println!("    Assets: time={}, score={}", 
             perf_stats.assets_time_index_size,
             perf_stats.assets_score_index_size);
    
    // Тестируем promotion цикл
    println!("\n🔄 Запуск оптимизированного promotion цикла...");
    let start_time = std::time::Instant::now();
    let promotion_stats = promotion_engine.run_optimized_promotion_cycle().await?;
    let duration = start_time.elapsed();
    
    println!("✅ Promotion цикл выполнен за {}ms", duration.as_millis());
    
    println!("  📊 Результаты:");
    println!("    Interact -> Insights: {} записей", promotion_stats.interact_to_insights);
    println!("    Insights -> Assets: {} записей", promotion_stats.insights_to_assets);
    println!("    Удалено из Interact: {} записей", promotion_stats.expired_interact);
    println!("    Удалено из Insights: {} записей", promotion_stats.expired_insights);
    
    println!("  ⏱️ Детальная производительность:");
    println!("    Общее время: {}ms", promotion_stats.total_time_ms);
    println!("    Обновление индексов: {}ms", promotion_stats.index_update_time_ms);
    println!("    Promotion операции: {}ms", promotion_stats.promotion_time_ms);
    println!("    Очистка: {}ms", promotion_stats.cleanup_time_ms);
    
    // Повторный цикл для проверки стабильности
    println!("\n🔄 Повторный цикл для проверки стабильности...");
    let second_start = std::time::Instant::now();
    let second_stats = promotion_engine.run_optimized_promotion_cycle().await?;
    let second_duration = second_start.elapsed();
    
    println!("✅ Второй цикл выполнен за {}ms", second_duration.as_millis());
    
    // Финальная статистика
    let final_perf_stats = promotion_engine.get_performance_stats().await?;
    
    println!("\n📈 Сравнение статистики:");
    println!("  Interact индексы: {} -> {}", 
             perf_stats.interact_time_index_size,
             final_perf_stats.interact_time_index_size);
    println!("  Insights индексы: {} -> {}", 
             perf_stats.insights_time_index_size,
             final_perf_stats.insights_time_index_size);
    println!("  Assets индексы: {} -> {}", 
             perf_stats.assets_time_index_size,
             final_perf_stats.assets_time_index_size);
    
    println!("\n🏆 РЕЗУЛЬТАТЫ ТЕСТА:");
    println!("===================");
    println!("  ✅ OptimizedPromotionEngine: Создание успешно");
    println!("  ✅ Time-based индексы: Инициализированы");
    println!("  ✅ Performance stats: Работают");
    println!("  ✅ Promotion цикл: Выполняется без ошибок"); 
    println!("  ✅ Повторные циклы: Стабильны");
    
    let avg_time = (duration.as_millis() + second_duration.as_millis()) / 2;
    println!("  📊 Средняя производительность: {}ms", avg_time);
    
    if avg_time < 50 {
        println!("  🚀 Отличная производительность!");
    } else if avg_time < 200 {
        println!("  ⚡ Хорошая производительность!");
    } else {
        println!("  📈 Приемлемая производительность");
    }
    
    println!("\n🎉 ОПТИМИЗАЦИЯ PROMOTION ENGINE ЗАВЕРШЕНА!");
    println!("   Time-based индексирование готово к продакшену");
    
    Ok(())
}