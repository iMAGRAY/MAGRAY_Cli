use memory::{migration::MigrationManager, VectorStore};
use anyhow::Result;
use clap::{Arg, Command};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    
    let matches = Command::new("migrate_to_hnsw_simple")
        .about("Миграция на максимальную эффективность с VectorIndexHnswSimple")
        .arg(
            Arg::new("db_path")
                .help("Путь к базе данных")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("command")
                .help("Команда: stats | migrate | clear | benchmark")
                .required(true)
                .index(2),
        )
        .get_matches();

    let db_path = matches.get_one::<String>("db_path").unwrap();
    let command = matches.get_one::<String>("command").unwrap();

    match command.as_str() {
        "stats" => show_stats(db_path).await?,
        "migrate" => run_migration(db_path).await?,
        "clear" => clear_all_data(db_path).await?,
        "benchmark" => run_benchmark(db_path).await?,
        _ => {
            eprintln!("Неизвестная команда: {}", command);
            eprintln!("Доступные команды: stats, migrate, clear, benchmark");
        }
    }

    Ok(())
}

async fn show_stats(db_path: &str) -> Result<()> {
    info!("Получение статистики базы данных...");
    
    let migration_manager = MigrationManager::new(db_path)?;
    let stats = migration_manager.get_stats().await?;
    
    println!("📊 Статистика базы данных:");
    println!("  Версия схемы: {}", stats.schema_version);
    println!("  Общий размер: {:.2} MB", stats.total_size_bytes as f64 / 1_048_576.0);
    println!();
    
    println!("📝 Слой Interact:");
    println!("  Записей: {}", stats.interact.record_count);
    println!("  Размер: {:.2} KB", stats.interact.total_size_bytes as f64 / 1024.0);
    println!("  Повреждённых: {}", stats.interact.corrupted_count);
    println!("  Средняя размерность: {:.1}", stats.interact.avg_embedding_dim);
    println!();
    
    println!("💡 Слой Insights:");
    println!("  Записей: {}", stats.insights.record_count);
    println!("  Размер: {:.2} KB", stats.insights.total_size_bytes as f64 / 1024.0);
    println!("  Повреждённых: {}", stats.insights.corrupted_count);
    println!("  Средняя размерность: {:.1}", stats.insights.avg_embedding_dim);
    println!();
    
    println!("🗃️ Слой Assets:");
    println!("  Записей: {}", stats.assets.record_count);
    println!("  Размер: {:.2} KB", stats.assets.total_size_bytes as f64 / 1024.0);
    println!("  Повреждённых: {}", stats.assets.corrupted_count);
    println!("  Средняя размерность: {:.1}", stats.assets.avg_embedding_dim);
    
    Ok(())
}

async fn run_migration(db_path: &str) -> Result<()> {
    info!("🚀 Начинаем миграцию для максимальной эффективности...");
    
    let migration_manager = MigrationManager::new(db_path)?;
    
    // Выполняем стандартную миграцию
    migration_manager.migrate().await?;
    
    // Создаём векторное хранилище с новым индексом
    info!("📊 Создание нового векторного хранилища с HnswSimple...");
    let vector_store = VectorStore::new(db_path).await?;
    
    // Инициализируем все слои для теста производительности
    use memory::Layer;
    for layer in [Layer::Interact, Layer::Insights, Layer::Assets] {
        vector_store.init_layer(layer).await?;
        
        // Получаем статистику слоя
        let mut count = 0;
        let iter = vector_store.iter_layer(layer).await?;
        for result in iter {
            if result.is_ok() {
                count += 1;
            }
        }
        
        info!("✅ Слой {:?}: {} записей", layer, count);
    }
    
    info!("🎉 Миграция завершена успешно!");
    info!("💡 Теперь используется VectorIndexHnswSimple для максимальной производительности");
    
    Ok(())
}

async fn clear_all_data(db_path: &str) -> Result<()> {
    warn!("⚠️ Очистка всех данных из базы...");
    
    let migration_manager = MigrationManager::new(db_path)?;
    migration_manager.clear_all_data().await?;
    
    info!("🗑️ Все данные очищены");
    
    Ok(())
}

async fn run_benchmark(db_path: &str) -> Result<()> {
    use std::time::Instant;
    use memory::{Layer, Record};
    use uuid::Uuid;
    use chrono::Utc;
    
    info!("🏃 Запуск бенчмарка производительности...");
    
    let vector_store = VectorStore::new(db_path).await?;
    
    // Генерируем тестовые данные
    info!("📝 Генерация тестовых данных...");
    let mut test_records = Vec::new();
    
    for i in 0..1000 {
        let embedding: Vec<f32> = (0..1024)
            .map(|j| ((i + j) as f32 * 0.1).sin())
            .collect();
            
        let record = Record {
            id: Uuid::new_v4(),
            text: format!("Тестовая запись {}", i),
            embedding,
            layer: Layer::Interact,
            kind: "test".to_string(),
            tags: vec!["benchmark".to_string()],
            project: "test_project".to_string(),
            session: "test_session".to_string(),
            ts: Utc::now(),
            last_access: Utc::now(),
            score: 0.0,
            access_count: 0,
        };
        
        test_records.push(record);
    }
    
    // Тест пакетной вставки
    info!("📥 Тест пакетной вставки...");
    let start = Instant::now();
    
    let record_refs: Vec<&Record> = test_records.iter().collect();
    vector_store.insert_batch(&record_refs).await?;
    
    let insert_time = start.elapsed();
    info!("✅ Вставка 1000 записей: {:?}", insert_time);
    
    // Тест поиска
    info!("🔍 Тест поиска...");
    let query_embedding: Vec<f32> = (0..1024)
        .map(|i| (i as f32 * 0.05).sin())
        .collect();
    
    let start = Instant::now();
    let search_results = vector_store.search(&query_embedding, Layer::Interact, 10).await?;
    let search_time = start.elapsed();
    
    info!("✅ Поиск top-10: {:?} (найдено {} результатов)", 
          search_time, search_results.len());
    
    // Multiple searches для проверки кэширования
    info!("🔄 Тест множественного поиска...");
    let start = Instant::now();
    
    for _ in 0..100 {
        let query: Vec<f32> = (0..1024)
            .map(|i| (i as f32 * 0.01).sin())
            .collect();
        vector_store.search(&query, Layer::Interact, 5).await?;
    }
    
    let multi_search_time = start.elapsed();
    info!("✅ 100 поисков: {:?} (среднее: {:?})", 
          multi_search_time, multi_search_time / 100);
    
    // Статистика производительности
    println!();
    println!("📊 Результаты бенчмарка:");
    println!("  Пакетная вставка (1000 записей): {:?}", insert_time);
    println!("  Поиск top-10: {:?}", search_time);
    println!("  Среднее время поиска: {:?}", multi_search_time / 100);
    println!("  Пропускная способность: {:.0} поисков/сек", 
             100.0 / multi_search_time.as_secs_f64());
    
    Ok(())
}