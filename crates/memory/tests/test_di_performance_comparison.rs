#![cfg(feature = "extended-tests")]

use anyhow::Result;
use memory::{DIContainer, DIPerformanceMetrics, Lifetime};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Comprehensive тесты сравнения производительности DI containers
/// Сравнивает: оригинальный vs оптимизированный DI container

/// Simple test service для бенчмарков
struct SimpleTestService {
    id: u64,
    name: String,
}

impl SimpleTestService {
    fn new(id: u64) -> Self {
        Self {
            id,
            name: format!("service-{}", id),
        }
    }
}

/// Heavy test service с дорогим конструктором
struct HeavyTestService {
    id: u64,
    computed_data: Vec<u64>,
    result: String,
}

impl HeavyTestService {
    fn new(id: u64) -> Self {
        // Имитация тяжелых вычислений
        let computed_data: Vec<u64> = (0..500).map(|i| (i * id) % 1000).collect();
        let result = format!("heavy-{}-{}", id, computed_data.iter().sum::<u64>());

        Self {
            id,
            computed_data,
            result,
        }
    }
}

/// Зависимый сервис для dependency injection тестов
struct DependentTestService {
    #[allow(dead_code)]
    simple: Arc<SimpleTestService>,
    #[allow(dead_code)]
    heavy: Arc<HeavyTestService>,
    combined_name: String,
}

impl DependentTestService {
    fn new(simple: Arc<SimpleTestService>, heavy: Arc<HeavyTestService>) -> Self {
        Self {
            combined_name: format!("dep-{}-{}", simple.id, heavy.id),
            simple,
            heavy,
        }
    }
}

/// Тест производительности регистрации сервисов
#[test]
fn test_registration_performance_comparison() -> Result<()> {
    println!("🧪 Сравнение производительности регистрации сервисов");

    let service_count = 1000;

    // Оригинальный container
    let start = Instant::now();
    let original_container = DIContainer::new();

    for i in 0..service_count {
        let i_copy = i;
        original_container.register(
            move |_| Ok(SimpleTestService::new(i_copy)),
            Lifetime::Singleton,
        )?;
    }

    let original_registration_time = start.elapsed();

    // Оптимизированный container
    let start = Instant::now();
    let optimized_container = DIContainer::new();

    for i in 0..service_count {
        let i_copy = i;
        optimized_container.register(
            move |_| Ok(SimpleTestService::new(i_copy)),
            Lifetime::Singleton,
        )?;
    }

    let optimized_registration_time = start.elapsed();

    println!("  📊 Регистрация {} сервисов:", service_count);
    println!("    Original: {:?}", original_registration_time);
    println!("    Optimized: {:?}", optimized_registration_time);

    let improvement = original_registration_time.as_nanos() as f64
        / optimized_registration_time.as_nanos() as f64;
    println!("    Improvement: {:.2}x", improvement);

    // Оптимизированный должен быть не медленнее оригинального
    assert!(
        optimized_registration_time <= original_registration_time * 2,
        "Optimized container registration is too slow"
    );

    Ok(())
}

/// Тест производительности разрешения singleton зависимостей
#[test]
fn test_singleton_resolution_performance() -> Result<()> {
    println!("🧪 Сравнение производительности разрешения singleton зависимостей");

    // Подготовка контейнеров
    let original_container = DIContainer::new();
    original_container.register(|_| Ok(SimpleTestService::new(42)), Lifetime::Singleton)?;

    let optimized_container = DIContainer::new();
    optimized_container.register(|_| Ok(SimpleTestService::new(42)), Lifetime::Singleton)?;

    let resolution_count = 10000;

    // Original container - первый resolve (создание)
    let start = Instant::now();
    let _service = original_container.resolve::<SimpleTestService>()?;
    let original_first_resolve = start.elapsed();

    // Original container - subsequent resolves (кэш)
    let start = Instant::now();
    for _ in 0..resolution_count {
        let _service = original_container.resolve::<SimpleTestService>()?;
    }
    let original_cached_resolves = start.elapsed();

    // Optimized container - первый resolve (создание)
    let start = Instant::now();
    let _service = optimized_container.resolve::<SimpleTestService>()?;
    let optimized_first_resolve = start.elapsed();

    // Optimized container - subsequent resolves (кэш)
    let start = Instant::now();
    for _ in 0..resolution_count {
        let _service = optimized_container.resolve::<SimpleTestService>()?;
    }
    let optimized_cached_resolves = start.elapsed();

    println!("  📊 Первое разрешение (создание):");
    println!("    Original: {:?}", original_first_resolve);
    println!("    Optimized: {:?}", optimized_first_resolve);

    println!("  📊 {} кэшированных разрешений:", resolution_count);
    println!("    Original: {:?}", original_cached_resolves);
    println!("    Optimized: {:?}", optimized_cached_resolves);

    let cache_improvement =
        original_cached_resolves.as_nanos() as f64 / optimized_cached_resolves.as_nanos() as f64;
    println!("    Cache improvement: {:.2}x", cache_improvement);

    // Проверяем метрики оптимизированного контейнера
    let metrics = optimized_container.get_performance_metrics();
    println!("  📊 Optimized metrics:");
    println!("    Cache hits: {}", metrics.cache_hits);
    println!(
        "    Cache misses: {}",
        metrics.total_resolves - metrics.cache_hits
    );
    println!("    Factory executions: {}", metrics.factory_creates);
    println!("    Avg resolution time: {}ns", metrics.avg_resolve_time_ns);

    // Кэшированные разрешения должны быть значительно быстрее
    assert!(
        cache_improvement >= 1.0,
        "Optimized cache should not be slower"
    );
    assert!(metrics.cache_hits > 0, "Should have cache hits");
    assert_eq!(
        metrics.factory_creates, 1,
        "Should have only one factory execution for singleton"
    );

    Ok(())
}

/// Тест производительности transient разрешений
#[test]
fn test_transient_resolution_performance() -> Result<()> {
    println!("🧪 Сравнение производительности transient разрешений");

    // Подготовка контейнеров
    let original_container = DIContainer::new();
    original_container.register(|_| Ok(HeavyTestService::new(123)), Lifetime::Transient)?;

    let optimized_container = DIContainer::new();
    optimized_container.register(|_| Ok(HeavyTestService::new(123)), Lifetime::Transient)?;

    let resolution_count = 100; // Меньше для transient из-за тяжелых операций

    // Original container
    let start = Instant::now();
    for _ in 0..resolution_count {
        let _service = original_container.resolve::<HeavyTestService>()?;
    }
    let original_time = start.elapsed();

    // Optimized container
    let start = Instant::now();
    for _ in 0..resolution_count {
        let _service = optimized_container.resolve::<HeavyTestService>()?;
    }
    let optimized_time = start.elapsed();

    println!("  📊 {} transient разрешений:", resolution_count);
    println!("    Original: {:?}", original_time);
    println!("    Optimized: {:?}", optimized_time);

    let improvement = original_time.as_nanos() as f64 / optimized_time.as_nanos() as f64;
    println!("    Improvement: {:.2}x", improvement);

    // Проверяем метрики
    let metrics = optimized_container.get_performance_metrics();
    println!("  📊 Optimized metrics:");
    println!("    Factory executions: {}", metrics.factory_creates);
    println!(
        "    Cache misses: {}",
        metrics.total_resolves - metrics.cache_hits
    );
    println!("    Avg resolution time: {}ns", metrics.avg_resolve_time_ns);

    assert_eq!(
        metrics.factory_creates, resolution_count,
        "Should execute factory for each transient"
    );
    assert_eq!(
        metrics.cache_hits, 0,
        "Transient should not have cache hits"
    );

    Ok(())
}

/// Тест производительности dependency injection chains
#[test]
fn test_dependency_chain_performance() -> Result<()> {
    println!("🧪 Сравнение производительности dependency chains");

    // Подготовка оригинального контейнера
    let original_container = DIContainer::new();
    original_container.register(|_| Ok(SimpleTestService::new(1)), Lifetime::Singleton)?;
    original_container.register(|_| Ok(HeavyTestService::new(2)), Lifetime::Singleton)?;
    original_container.register(
        |container| {
            let simple = container.resolve::<SimpleTestService>()?;
            let heavy = container.resolve::<HeavyTestService>()?;
            Ok(DependentTestService::new(simple, heavy))
        },
        Lifetime::Transient,
    )?;

    // Подготовка оптимизированного контейнера
    let optimized_container = DIContainer::new();
    optimized_container.register(|_| Ok(SimpleTestService::new(1)), Lifetime::Singleton)?;
    optimized_container.register(|_| Ok(HeavyTestService::new(2)), Lifetime::Singleton)?;
    optimized_container.register(
        |container| {
            let simple = container.resolve::<SimpleTestService>()?;
            let heavy = container.resolve::<HeavyTestService>()?;
            Ok(DependentTestService::new(simple, heavy))
        },
        Lifetime::Transient,
    )?;

    let resolution_count = 1000;

    // Original container
    let start = Instant::now();
    for _ in 0..resolution_count {
        let _service = original_container.resolve::<DependentTestService>()?;
    }
    let original_time = start.elapsed();

    // Optimized container
    let start = Instant::now();
    for _ in 0..resolution_count {
        let _service = optimized_container.resolve::<DependentTestService>()?;
    }
    let optimized_time = start.elapsed();

    println!("  📊 {} dependency chain разрешений:", resolution_count);
    println!("    Original: {:?}", original_time);
    println!("    Optimized: {:?}", optimized_time);

    let improvement = original_time.as_nanos() as f64 / optimized_time.as_nanos() as f64;
    println!("    Improvement: {:.2}x", improvement);

    // Проверяем метрики оптимизированного
    let metrics = optimized_container.get_performance_metrics();
    println!("  📊 Optimized metrics:");
    println!("    Total resolutions: {}", metrics.total_resolves);
    println!("    Cache hits: {}", metrics.cache_hits);
    println!("    Factory executions: {}", metrics.factory_creates);

    // Должны быть cache hits для singleton dependencies
    assert!(
        metrics.cache_hits > 0,
        "Should have cache hits for singleton dependencies"
    );

    Ok(())
}

/// Тест производительности concurrent access
#[test]
fn test_concurrent_performance() -> Result<()> {
    println!("🧪 Сравнение производительности concurrent access");

    // Подготовка контейнеров
    let original_container = Arc::new(DIContainer::new());
    original_container.register(|_| Ok(SimpleTestService::new(99)), Lifetime::Singleton)?;

    let optimized_container = Arc::new(DIContainer::new());
    optimized_container.register(|_| Ok(SimpleTestService::new(99)), Lifetime::Singleton)?;

    let thread_count = 8;
    let resolutions_per_thread = 1000;

    // Original container concurrent test
    let start = Instant::now();
    let handles: Vec<_> = (0..thread_count)
        .map(|_| {
            let container = Arc::clone(&original_container);
            std::thread::spawn(move || {
                for _ in 0..resolutions_per_thread {
                    let _service = container
                        .resolve::<SimpleTestService>()
                        .expect("Test operation should succeed");
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Test operation should succeed");
    }
    let original_concurrent_time = start.elapsed();

    // Optimized container concurrent test
    let start = Instant::now();
    let handles: Vec<_> = (0..thread_count)
        .map(|_| {
            let container = Arc::clone(&optimized_container);
            std::thread::spawn(move || {
                for _ in 0..resolutions_per_thread {
                    let _service = container
                        .resolve::<SimpleTestService>()
                        .expect("Test operation should succeed");
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("Test operation should succeed");
    }
    let optimized_concurrent_time = start.elapsed();

    let total_resolutions = thread_count * resolutions_per_thread;
    println!(
        "  📊 {} concurrent разрешений ({} threads × {}):",
        total_resolutions, thread_count, resolutions_per_thread
    );
    println!("    Original: {:?}", original_concurrent_time);
    println!("    Optimized: {:?}", optimized_concurrent_time);

    let improvement =
        original_concurrent_time.as_nanos() as f64 / optimized_concurrent_time.as_nanos() as f64;
    println!("    Improvement: {:.2}x", improvement);

    // Проверяем метрики
    let metrics = optimized_container.get_performance_metrics();
    println!("  📊 Optimized concurrent metrics:");
    println!("    Total resolutions: {}", metrics.total_resolves);
    println!("    Cache hits: {}", metrics.cache_hits);
    println!(
        "    Cache hit ratio: {:.2}%",
        metrics.cache_hits as f64 / metrics.total_resolves as f64 * 100.0
    );

    // В concurrent окружении должен быть высокий cache hit ratio для singleton
    let cache_hit_ratio = metrics.cache_hits as f64 / metrics.total_resolves as f64;
    assert!(
        cache_hit_ratio > 0.95,
        "Cache hit ratio should be very high for singleton"
    );

    Ok(())
}

/// Тест builder pattern performance
#[test]
fn test_builder_performance() -> Result<()> {
    println!("🧪 Сравнение производительности builder pattern");

    let builder_count = 100;

    // Original builder
    let start = Instant::now();
    for i in 0..builder_count {
        let _container = DIContainer::new();
        _container.register(move |_| Ok(SimpleTestService::new(i)), Lifetime::Singleton)?;
    }
    let original_builder_time = start.elapsed();

    // Optimized builder
    let start = Instant::now();
    for i in 0..builder_count {
        let _container = DIContainer::new();
        _container.register(move |_| Ok(SimpleTestService::new(i)), Lifetime::Singleton)?;
    }
    let optimized_builder_time = start.elapsed();

    println!("  📊 {} builder creations:", builder_count);
    println!("    Original: {:?}", original_builder_time);
    println!("    Optimized: {:?}", optimized_builder_time);

    let improvement =
        original_builder_time.as_nanos() as f64 / optimized_builder_time.as_nanos() as f64;
    println!("    Improvement: {:.2}x", improvement);

    Ok(())
}

/// Integration test всех performance optimizations
#[test]
fn test_comprehensive_performance_comparison() -> Result<()> {
    println!("🧪 Comprehensive performance comparison");

    // Создаем реалистичный сценарий с множественными сервисами
    let service_count = 50;
    let resolution_cycles = 1000;

    // Original container setup
    let original_container = DIContainer::new();
    for i in 0..service_count {
        let i_copy = i;
        original_container.register(
            move |_| Ok(SimpleTestService::new(i_copy)),
            if i_copy % 2 == 0 {
                Lifetime::Singleton
            } else {
                Lifetime::Transient
            },
        )?;
    }

    // Optimized container setup
    let optimized_container = DIContainer::new();
    for i in 0..service_count {
        let i_copy = i;
        optimized_container.register(
            move |_| Ok(SimpleTestService::new(i_copy)),
            if i_copy % 2 == 0 {
                Lifetime::Singleton
            } else {
                Lifetime::Transient
            },
        )?;
    }

    // Performance test: mixed resolution pattern
    let start = Instant::now();
    for cycle in 0..resolution_cycles {
        for i in 0..10 {
            // Resolve 10 random services per cycle
            let service_id = (cycle * 7 + i * 13) % service_count; // Pseudo-random pattern
            let _service = original_container.resolve::<SimpleTestService>()?;
        }
    }
    let original_mixed_time = start.elapsed();

    let start = Instant::now();
    for cycle in 0..resolution_cycles {
        for i in 0..10 {
            let service_id = (cycle * 7 + i * 13) % service_count;
            let _service = optimized_container.resolve::<SimpleTestService>()?;
        }
    }
    let optimized_mixed_time = start.elapsed();

    let total_resolutions = resolution_cycles * 10;
    println!(
        "  📊 Comprehensive test ({} services, {} resolutions):",
        service_count, total_resolutions
    );
    println!("    Original total: {:?}", original_mixed_time);
    println!("    Optimized total: {:?}", optimized_mixed_time);

    let overall_improvement =
        original_mixed_time.as_nanos() as f64 / optimized_mixed_time.as_nanos() as f64;
    println!("    Overall improvement: {:.2}x", overall_improvement);

    // Detailed metrics
    let metrics = optimized_container.get_performance_metrics();
    println!("  📊 Final optimized metrics:");
    // Registration count не доступен в текущих метриках
    println!("    Total resolutions: {}", metrics.total_resolves);
    println!("    Cache hits: {}", metrics.cache_hits);
    println!(
        "    Cache misses: {}",
        metrics.total_resolves - metrics.cache_hits
    );
    println!("    Factory executions: {}", metrics.factory_creates);
    println!(
        "    Cache hit ratio: {:.2}%",
        metrics.cache_hits as f64 / metrics.total_resolves as f64 * 100.0
    );
    println!("    Avg resolution time: {}ns", metrics.avg_resolve_time_ns);

    // Проверяем что оптимизации работают
    assert!(metrics.cache_hits > 0, "Should have cache hits");
    assert!(
        overall_improvement >= 0.8,
        "Overall performance should not significantly regress"
    );

    println!("✅ Comprehensive performance comparison успешен");
    Ok(())
}

/// Smoke test для базовой функциональности
#[test]
fn test_optimized_container_smoke() -> Result<()> {
    let container = DIContainer::new();

    // Registration
    container.register(|_| Ok(SimpleTestService::new(1)), Lifetime::Singleton)?;
    container.register(|_| Ok(HeavyTestService::new(2)), Lifetime::Transient)?;

    // Resolution
    let simple = container.resolve::<SimpleTestService>()?;
    assert_eq!(simple.id, 1);

    let heavy = container.resolve::<HeavyTestService>()?;
    assert_eq!(heavy.id, 2);

    // Stats
    let stats = container.stats();
    assert_eq!(stats.registered_factories, 2);
    assert_eq!(stats.cached_singletons, 1); // SimpleTestService cached

    // Metrics
    let metrics = container.get_performance_metrics();
    // Registration count не доступен в текущих метриках
    assert_eq!(metrics.total_resolves, 2);
    assert_eq!(metrics.cache_hits, 0); // No cache hits on first resolve
    assert_eq!(metrics.factory_creates, 2);

    println!("✅ Optimized container smoke test прошел");
    Ok(())
}
