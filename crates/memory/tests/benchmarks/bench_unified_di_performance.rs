//! Performance Benchmarks for Unified DI System
//! 
//! Comprehensive performance testing including:
//! - Container creation and initialization benchmarks
//! - Service resolution performance under different loads
//! - Concurrent access and thread safety performance
//! - Memory allocation and cleanup efficiency
//! - Factory pattern performance across different configurations
//! - Large-scale container performance with many services

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::sync::Arc;
use std::time::Duration;
use tokio::runtime::Runtime;

use crate::{
    di::{
        unified_container::UnifiedDIContainer,
        unified_config::UnifiedDIConfiguration,
        errors::DIResult,
        traits::ServiceLifetime,
    },
    services::{
        unified_factory::{UnifiedServiceFactory, FactoryPreset},
        monitoring_service::MonitoringService,
        cache_service::CacheService,
    },
};

// Mock services for benchmarking
#[derive(Debug, Clone)]
struct BenchmarkService {
    id: String,
    data: Vec<u8>,
}

impl BenchmarkService {
    fn new(id: String, size_kb: usize) -> Self {
        Self {
            id,
            data: vec![0u8; size_kb * 1024], // Simulate memory usage
        }
    }
    
    fn get_id(&self) -> &str {
        &self.id
    }
    
    fn process_data(&self) -> usize {
        // Simulate some work
        self.data.iter().sum::<u8>() as usize
    }
}

#[derive(Debug)]
struct HeavyBenchmarkService {
    dependencies: Vec<Arc<BenchmarkService>>,
    computation_result: u64,
}

impl HeavyBenchmarkService {
    fn new(dependencies: Vec<Arc<BenchmarkService>>) -> Self {
        // Simulate expensive initialization
        let computation_result = dependencies.iter()
            .map(|dep| dep.process_data() as u64)
            .sum();
        
        Self {
            dependencies,
            computation_result,
        }
    }
    
    fn get_result(&self) -> u64 {
        self.computation_result
    }
}

// Benchmark container creation with different configurations
fn bench_container_creation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("container_creation");
    
    let configs = vec![
        ("minimal", UnifiedDIConfiguration::minimal_config().unwrap()),
        ("test", UnifiedDIConfiguration::test_config().unwrap()),
        ("development", UnifiedDIConfiguration::development_config().unwrap()),
        ("production", UnifiedDIConfiguration::production_config().unwrap()),
    ];
    
    for (config_name, config) in configs {
        group.bench_with_input(
            BenchmarkId::new("create_container", config_name),
            &config,
            |b, config| {
                b.to_async(&rt).iter(|| async {
                    let container = black_box(UnifiedDIContainer::new(config.clone()).await.unwrap());
                    container.shutdown().await.unwrap();
                });
            },
        );
    }
    
    group.finish();
}

// Benchmark factory-based container creation
fn bench_factory_container_building(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("factory_container_building");
    
    let factory_presets = vec![
        ("minimal", FactoryPreset::Minimal),
        ("test", FactoryPreset::Test),
        ("development", FactoryPreset::Development),
        ("production", FactoryPreset::Production),
    ];
    
    for (preset_name, preset) in factory_presets {
        group.bench_with_input(
            BenchmarkId::new("build_with_factory", preset_name),
            &preset,
            |b, preset| {
                b.to_async(&rt).iter(|| async {
                    let factory = UnifiedServiceFactory::with_preset(*preset).unwrap();
                    let config = match preset {
                        FactoryPreset::Minimal => UnifiedDIConfiguration::minimal_config().unwrap(),
                        FactoryPreset::Test => UnifiedDIConfiguration::test_config().unwrap(),
                        FactoryPreset::Development => UnifiedDIConfiguration::development_config().unwrap(),
                        FactoryPreset::Production => UnifiedDIConfiguration::production_config().unwrap(),
                        _ => UnifiedDIConfiguration::test_config().unwrap(),
                    };
                    
                    let container = black_box(factory.build_container(&config).await.unwrap());
                    container.shutdown().await.unwrap();
                });
            },
        );
    }
    
    group.finish();
}

// Benchmark service resolution performance
fn bench_service_resolution(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("service_resolution");
    
    // Setup container with pre-registered services
    let container = rt.block_on(async {
        let config = UnifiedDIConfiguration::test_config().unwrap();
        let mut container = UnifiedDIContainer::new(config).await.unwrap();
        
        // Register various services
        container.register_singleton::<BenchmarkService, _>(
            "SingletonService",
            || Ok(Arc::new(BenchmarkService::new("singleton".to_string(), 1)))
        ).unwrap();
        
        container.register_transient::<BenchmarkService, _>(
            "TransientService", 
            || Ok(Arc::new(BenchmarkService::new("transient".to_string(), 1)))
        ).unwrap();
        
        Arc::new(container)
    });
    
    // Benchmark singleton resolution
    group.bench_function("resolve_singleton", |b| {
        b.to_async(&rt).iter(|| async {
            let service = black_box(
                container.resolve_named::<BenchmarkService>("SingletonService").await.unwrap()
            );
            black_box(service.get_id());
        });
    });
    
    // Benchmark transient resolution
    group.bench_function("resolve_transient", |b| {
        b.to_async(&rt).iter(|| async {
            let service = black_box(
                container.resolve_named::<BenchmarkService>("TransientService").await.unwrap()
            );
            black_box(service.get_id());
        });
    });
    
    group.finish();
}

// Benchmark concurrent service resolution
fn bench_concurrent_resolution(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("concurrent_resolution");
    
    let container = rt.block_on(async {
        let config = UnifiedDIConfiguration::test_config().unwrap();
        let mut container = UnifiedDIContainer::new(config).await.unwrap();
        
        container.register_singleton::<BenchmarkService, _>(
            "ConcurrentService",
            || Ok(Arc::new(BenchmarkService::new("concurrent".to_string(), 4)))
        ).unwrap();
        
        Arc::new(container)
    });
    
    for thread_count in [1, 2, 4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_resolution", thread_count),
            thread_count,
            |b, &thread_count| {
                b.to_async(&rt).iter(|| async {
                    let tasks: Vec<_> = (0..thread_count)
                        .map(|_| {
                            let container = container.clone();
                            tokio::spawn(async move {
                                let service = container
                                    .resolve_named::<BenchmarkService>("ConcurrentService")
                                    .await
                                    .unwrap();
                                black_box(service.process_data())
                            })
                        })
                        .collect();
                    
                    let results: Vec<_> = futures::future::join_all(tasks).await;
                    black_box(results);
                });
            },
        );
    }
    
    group.finish();
}

// Benchmark large-scale container with many services
fn bench_large_scale_container(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("large_scale_container");
    
    for service_count in [10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("many_services", service_count),
            service_count,
            |b, &service_count| {
                b.to_async(&rt).iter(|| async {
                    let config = UnifiedDIConfiguration::production_config().unwrap();
                    let mut container = UnifiedDIContainer::new(config).await.unwrap();
                    
                    // Register many services
                    for i in 0..service_count {
                        let service_name = format!("Service_{}", i);
                        container.register_singleton::<BenchmarkService, _>(
                            &service_name,
                            move || Ok(Arc::new(BenchmarkService::new(format!("service_{}", i), 1)))
                        ).unwrap();
                    }
                    
                    // Resolve all services
                    for i in 0..service_count {
                        let service_name = format!("Service_{}", i);
                        let service = container
                            .resolve_named::<BenchmarkService>(&service_name)
                            .await
                            .unwrap();
                        black_box(service.get_id());
                    }
                    
                    container.shutdown().await.unwrap();
                });
            },
        );
    }
    
    group.finish();
}

// Benchmark dependency injection performance
fn bench_dependency_injection(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("dependency_injection");
    
    let container = rt.block_on(async {
        let config = UnifiedDIConfiguration::test_config().unwrap();
        let mut container = UnifiedDIContainer::new(config).await.unwrap();
        
        // Register dependencies
        for i in 0..5 {
            let dep_name = format!("Dependency_{}", i);
            container.register_singleton::<BenchmarkService, _>(
                &dep_name,
                move || Ok(Arc::new(BenchmarkService::new(format!("dep_{}", i), 2)))
            ).unwrap();
        }
        
        // Register service with dependencies
        let container_ref = &container;
        container.register_singleton::<HeavyBenchmarkService, _>(
            "HeavyService",
            move || {
                let rt = Runtime::new().unwrap();
                rt.block_on(async {
                    let mut deps = Vec::new();
                    for i in 0..5 {
                        let dep_name = format!("Dependency_{}", i);
                        let dep = container_ref
                            .resolve_named::<BenchmarkService>(&dep_name)
                            .await
                            .unwrap();
                        deps.push(dep);
                    }
                    Ok(Arc::new(HeavyBenchmarkService::new(deps)))
                })
            }
        ).unwrap();
        
        Arc::new(container)
    });
    
    group.bench_function("resolve_with_dependencies", |b| {
        b.to_async(&rt).iter(|| async {
            let service = black_box(
                container.resolve_named::<HeavyBenchmarkService>("HeavyService").await.unwrap()
            );
            black_box(service.get_result());
        });
    });
    
    group.finish();
}

// Benchmark memory allocation and cleanup
fn bench_memory_management(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("memory_management");
    
    group.bench_function("container_lifecycle", |b| {
        b.to_async(&rt).iter(|| async {
            let config = UnifiedDIConfiguration::test_config().unwrap();
            let mut container = UnifiedDIContainer::new(config).await.unwrap();
            
            // Register memory-intensive services
            for i in 0..10 {
                let service_name = format!("MemoryService_{}", i);
                container.register_singleton::<BenchmarkService, _>(
                    &service_name,
                    move || Ok(Arc::new(BenchmarkService::new(format!("memory_{}", i), 100))) // 100KB each
                ).unwrap();
            }
            
            // Use services
            for i in 0..10 {
                let service_name = format!("MemoryService_{}", i);
                let service = container
                    .resolve_named::<BenchmarkService>(&service_name)
                    .await
                    .unwrap();
                black_box(service.process_data());
            }
            
            // Cleanup
            container.shutdown().await.unwrap();
        });
    });
    
    group.finish();
}

// Benchmark configuration validation performance
fn bench_configuration_validation(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    let mut group = c.benchmark_group("configuration_validation");
    
    let configs = vec![
        ("simple", UnifiedDIConfiguration::test_config().unwrap()),
        ("complex", UnifiedDIConfiguration::production_config().unwrap()),
    ];
    
    for (config_name, config) in configs {
        group.bench_with_input(
            BenchmarkId::new("validate_config", config_name),
            &config,
            |b, config| {
                b.iter(|| {
                    let validation_result = black_box(config.validate().unwrap());
                    black_box(validation_result.is_valid);
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    benches,
    bench_container_creation,
    bench_factory_container_building,
    bench_service_resolution,
    bench_concurrent_resolution,
    bench_large_scale_container,
    bench_dependency_injection,
    bench_memory_management,
    bench_configuration_validation
);

criterion_main!(benches);