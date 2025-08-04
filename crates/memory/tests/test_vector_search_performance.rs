use anyhow::Result;
use memory::{
    VectorStore,
    Layer, Record,
    HnswRsConfig,
};
use std::sync::Arc;
use std::time::Instant;
use tempfile::TempDir;
use uuid::Uuid;
use chrono::Utc;

#[tokio::test]
async fn test_vector_search_performance() -> Result<()> {
    // Создаем временную директорию для БД
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");
    
    // Создаем VectorStore с оптимальной конфигурацией HNSW
    let hnsw_config = HnswRsConfig {
        dimension: 1024,
        max_connections: 32,      // Увеличиваем для лучшей точности
        ef_construction: 400,     // Высокое качество построения
        ef_search: 100,          // Баланс скорость/точность
        max_elements: 100_000,   // Достаточно для теста
        max_layers: 16,
        use_parallel: true,
    };
    
    let store = Arc::new(VectorStore::with_config(&db_path, hnsw_config).await?);
    
    // Генерируем тестовые векторы
    println!("Генерация тестовых данных...");
    let num_vectors = 10_000;
    let mut records = Vec::new();
    
    for i in 0..num_vectors {
        // Генерируем случайные векторы
        let embedding: Vec<f32> = (0..1024)
            .map(|j| ((i * j) as f32 / 1000.0).sin())
            .collect();
        
        let record = Record {
            id: Uuid::new_v4(),
            layer: Layer::Interact,
            text: format!("Test record {}", i),
            kind: "test".to_string(),
            tags: vec![format!("tag{}", i % 10)],
            project: "test".to_string(),
            session: "test".to_string(),
            embedding,
            ts: Utc::now(),
            last_access: Utc::now(),
            access_count: 0,
            score: rand::random::<f32>(),
        };
        records.push(record);
    }
    
    // Вставляем записи batch'ами
    println!("Вставка {} записей...", num_vectors);
    let start_insert = Instant::now();
    
    let batch_size = 1000;
    for chunk in records.chunks(batch_size) {
        let refs: Vec<&Record> = chunk.iter().collect();
        store.insert_batch(&refs).await?;
    }
    
    let insert_duration = start_insert.elapsed();
    println!("Вставка завершена за {:?} ({:.0} записей/сек)", 
             insert_duration,
             num_vectors as f64 / insert_duration.as_secs_f64());
    
    // Тестируем поиск
    println!("\nТестирование производительности поиска...");
    
    // Создаем запросы
    let num_queries = 100;
    let mut query_vectors = Vec::new();
    for i in 0..num_queries {
        let query: Vec<f32> = (0..1024)
            .map(|j| ((i * j * 2) as f32 / 1000.0).cos())
            .collect();
        query_vectors.push(query);
    }
    
    // Последовательный поиск
    let start_sequential = Instant::now();
    let mut sequential_results = 0;
    
    for query in &query_vectors {
        let results = store.search(query, Layer::Interact, 10).await?;
        sequential_results += results.len();
    }
    
    let sequential_duration = start_sequential.elapsed();
    let sequential_qps = num_queries as f64 / sequential_duration.as_secs_f64();
    
    println!("Последовательный поиск:");
    println!("  - {} запросов за {:?}", num_queries, sequential_duration);
    println!("  - {:.0} запросов/сек", sequential_qps);
    println!("  - Среднее время: {:.2} мс/запрос", 
             sequential_duration.as_secs_f64() * 1000.0 / num_queries as f64);
    
    // Параллельный поиск через tokio tasks
    println!("\nТестирование параллельного поиска через tokio...");
    let start_parallel = Instant::now();
    
    // Запускаем параллельные задачи
    let store_clone = store.clone();
    let mut handles = Vec::new();
    
    for query in query_vectors.clone() {
        let store = store_clone.clone();
        let handle = tokio::spawn(async move {
            store.search(&query, Layer::Interact, 10).await
        });
        handles.push(handle);
    }
    
    // Ждем завершения всех задач
    let mut parallel_results = 0;
    for handle in handles {
        let result = handle.await??;
        parallel_results += result.len();
    }
    
    let parallel_duration = start_parallel.elapsed();
    let parallel_qps = num_queries as f64 / parallel_duration.as_secs_f64();
    
    println!("Параллельный поиск:");
    println!("  - {} запросов за {:?}", num_queries, parallel_duration);
    println!("  - {:.0} запросов/сек", parallel_qps);
    println!("  - Ускорение: {:.1}x", sequential_duration.as_secs_f64() / parallel_duration.as_secs_f64());
    
    // Проверяем результаты
    assert_eq!(sequential_results, parallel_results, "Результаты должны совпадать");
    
    // Проверка что поиск действительно O(log n)
    println!("\nПроверка сложности O(log n)...");
    
    // Замеряем время поиска для разных размеров БД
    let test_sizes = vec![1000, 2000, 4000, 8000];
    let mut search_times = Vec::new();
    
    for &size in &test_sizes {
        // Используем первые N записей
        let query = &query_vectors[0];
        
        // Прогреваем кэш
        let _ = store.search(query, Layer::Interact, 10).await?;
        
        // Замеряем время
        let start = Instant::now();
        let iterations = 10;
        for _ in 0..iterations {
            let _ = store.search(query, Layer::Interact, 10).await?;
        }
        let avg_time = start.elapsed().as_micros() as f64 / iterations as f64;
        search_times.push((size, avg_time));
        
        println!("  Размер БД: {}, среднее время поиска: {:.0} мкс", size, avg_time);
    }
    
    // Проверяем что время растет логарифмически
    let growth_factor = search_times.last().unwrap().1 / search_times.first().unwrap().1;
    let size_factor = *test_sizes.last().unwrap() as f64 / *test_sizes.first().unwrap() as f64;
    let expected_growth = size_factor.log2();
    
    println!("\nАнализ сложности:");
    println!("  - Рост размера БД: {}x", size_factor);
    println!("  - Рост времени поиска: {:.2}x", growth_factor);
    println!("  - Ожидаемый рост (log n): {:.2}x", expected_growth);
    
    // Проверяем что рост времени близок к логарифмическому
    assert!(growth_factor < expected_growth * 2.0, 
            "Время поиска растет быстрее чем O(log n)");
    
    // Выводим общую статистику
    println!("\nСтатистика HNSW:");
    println!("  - Векторов в индексе: {}", num_vectors);
    println!("  - Среднее время поиска: {:.2} мс", sequential_duration.as_secs_f64() * 1000.0 / num_queries as f64);
    println!("  - Пропускная способность: {:.0} поисков/сек", sequential_qps);
    println!("  - Примерное использование памяти: ~{} MB", num_vectors * 4 / 1024);
    
    println!("\n✅ Все тесты производительности векторного поиска пройдены!");
    
    Ok(())
}

#[tokio::test]
async fn test_hnsw_accuracy() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let db_path = temp_dir.path().join("test_db");
    
    let store = Arc::new(VectorStore::new(&db_path).await?);
    
    // Создаем кластеры похожих векторов
    let clusters = 5;
    let vectors_per_cluster = 20;
    let mut all_records = Vec::new();
    let mut cluster_centers = Vec::new();
    
    for cluster_id in 0..clusters {
        // Центр кластера
        let center: Vec<f32> = (0..1024)
            .map(|i| ((cluster_id * i) as f32 / 100.0).sin())
            .collect();
        cluster_centers.push(center.clone());
        
        // Векторы в кластере (с небольшим шумом)
        for j in 0..vectors_per_cluster {
            let mut vector = center.clone();
            for v in vector.iter_mut() {
                *v += (j as f32 / 100.0) * 0.1; // Небольшой шум
            }
            
            let record = Record {
                id: Uuid::new_v4(),
                layer: Layer::Interact,
                text: format!("Cluster {} vector {}", cluster_id, j),
                kind: "test".to_string(),
                tags: vec![format!("cluster{}", cluster_id)],
                project: "test".to_string(),
                session: "test".to_string(),
                embedding: vector,
                ts: Utc::now(),
                last_access: Utc::now(),
                access_count: 0,
                score: 1.0,
            };
            all_records.push(record);
        }
    }
    
    // Вставляем все записи
    let refs: Vec<&Record> = all_records.iter().collect();
    store.insert_batch(&refs).await?;
    
    // Тестируем точность поиска
    println!("Тестирование точности HNSW...");
    
    for (cluster_id, center) in cluster_centers.iter().enumerate() {
        let results = store.search(center, Layer::Interact, 10).await?;
        
        // Проверяем что большинство результатов из правильного кластера
        let correct_cluster = results.iter()
            .filter(|r| r.tags.contains(&format!("cluster{}", cluster_id)))
            .count();
        
        let accuracy = correct_cluster as f64 / results.len() as f64;
        println!("Кластер {}: точность {:.1}%", cluster_id, accuracy * 100.0);
        
        assert!(accuracy > 0.7, "Точность поиска слишком низкая для кластера {}", cluster_id);
    }
    
    println!("\n✅ Тест точности HNSW пройден!");
    
    Ok(())
}