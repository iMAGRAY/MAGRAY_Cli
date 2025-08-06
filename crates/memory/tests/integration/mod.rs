//! Integration Tests Module
//! 
//! Comprehensive integration tests для MAGRAY CLI системы после архитектурных улучшений.
//! 
//! ## Структура тестов:
//! 
//! ### 1. Full System Tests (`full_system_test.rs`)
//! - Complete end-to-end workflow testing
//! - Concurrent user sessions simulation  
//! - Production workload simulation
//! - Memory lifecycle integration
//! - Error recovery and resilience
//! 
//! ### 2. Orchestration Tests (`orchestration_test.rs`)
//! - MemoryOrchestrator integration (95% готовности)
//! - EmbeddingCoordinator integration (95% готовности) 
//! - SearchCoordinator integration (95% готовности)
//! - HealthManager integration (95% готовности)
//! - ResourceController integration (95% готовности)
//! - Cross-coordinator integration
//! 
//! ### 3. Performance Tests (`performance_test.rs`)
//! - Sub-5ms search SLA validation
//! - 100+ concurrent operations support
//! - Memory efficiency under sustained load
//! - Production throughput benchmarks
//! 
//! ### 4. Resilience Tests (`resilience_test.rs`) 
//! - Circuit breaker activation и recovery
//! - Component failure scenarios
//! - Graceful degradation testing
//! - Data consistency under failures
//! 
//! ### 5. DI Container Tests (`di_container_test.rs`)
//! - Full DI container с всеми зависимостями
//! - Performance metrics для DI operations
//! - Lifecycle management через DI
//! - Error propagation через DI chain
//! 
//! ## Компоненты под тестированием:
//! 
//! - **DIMemoryService** (95% готовности): Production DI memory service
//! - **MemoryOrchestrator** (95% готовности): Главный координатор системы
//! - **UnifiedAgent** (90% готовности): Clean Architecture agent
//! - **Orchestration Coordinators** (95% готовности): Все 6+ coordinators
//! 
//! ## SLA требования:
//! 
//! - **Search Latency**: < 5ms average, < 8ms P95, < 15ms P99
//! - **Throughput**: > 50 ops/sec search-heavy, > 20 ops/sec balanced
//! - **Concurrent Operations**: 100+ operations with > 90% success rate
//! - **Circuit Breaker**: Activation < 10% error rate, Recovery < 2s
//! - **Data Consistency**: > 95% preservation under failures
//! - **Cache Efficiency**: > 50% hit rate, > 70% under load

pub mod full_system_test;
pub mod orchestration_test;
pub mod performance_test;
pub mod resilience_test;
pub mod di_container_test;

