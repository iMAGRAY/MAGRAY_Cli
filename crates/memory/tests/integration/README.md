# MAGRAY CLI Integration Tests

Comprehensive integration test suite для валидации полной функциональности MAGRAY CLI системы после архитектурных улучшений.

## 🎯 Цель тестирования

Валидация production-ready готовности всех компонентов системы:

- **DIMemoryService** (95% готовности) - Production DI memory service
- **MemoryOrchestrator** (95% готовности) - Главный системный координатор  
- **UnifiedAgent** (90% готовности) - Clean Architecture agent
- **Orchestration Coordinators** (95% готовности) - Все 6+ coordinators

## 📋 Структура тестов

### 1. Full System Tests (`full_system_test.rs`)

**Назначение**: End-to-end workflow валидация

**Тестовые сценарии**:
- `test_complete_end_to_end_workflow()` - Полный пользовательский workflow
- `test_concurrent_user_sessions()` - 10 concurrent user sessions
- `test_production_workload_simulation()` - Production нагрузочное тестирование
- `test_memory_lifecycle_integration()` - Lifecycle записей через все слои
- `test_error_recovery_resilience()` - Recovery после failures

**SLA требования**:
- Sub-5ms search performance
- 100+ concurrent operations
- > 90% success rate под нагрузкой

### 2. Orchestration Tests (`orchestration_test.rs`)

**Назначение**: Валидация orchestration coordinators integration

**Тестовые сценарии**:
- `test_memory_orchestrator_integration()` - Главный координатор
- `test_embedding_coordinator_integration()` - AI embeddings с adaptive batching
- `test_search_coordinator_integration()` - Sub-5ms HNSW поиск  
- `test_health_manager_integration()` - Production monitoring
- `test_resource_controller_integration()` - Auto-scaling
- `test_cross_coordinator_integration()` - Взаимодействие coordinators

**SLA требования**:
- Circuit breaker activation/recovery < 2s
- Coordinator sync time < 100ms
- Health monitoring SLA compliance

### 3. Performance Tests (`performance_test.rs`)

**Назначение**: Валидация производительности и SLA compliance

**Тестовые сценарии**:
- `test_sub_5ms_search_sla_validation()` - Строгая валидация search SLA
- `test_concurrent_operations_performance()` - 150 concurrent operations
- `test_memory_efficiency_sustained_load()` - 30s sustained load
- `test_production_throughput_benchmarks()` - Production benchmarks

**SLA требования**:
- Average search: < 5ms
- P95 search: < 8ms  
- P99 search: < 15ms
- Throughput: > 50 ops/sec (search-heavy), > 20 ops/sec (balanced)
- Cache efficiency: > 50% hit rate

### 4. Resilience Tests (`resilience_test.rs`)

**Назначение**: Fault tolerance и error recovery валидация

**Тестовые сценарии**:
- `test_circuit_breaker_activation_recovery()` - Circuit breaker patterns
- `test_component_failure_scenarios()` - Component failures
- `test_graceful_degradation()` - Graceful degradation под stress
- `test_data_consistency_under_failures()` - Data consistency

**SLA требования**:
- > 95% data consistency под failures
- Graceful degradation: > 70% success rate
- Recovery time: < 5s для full recovery

### 5. DI Container Tests (`di_container_test.rs`)

**Назначение**: Dependency injection система валидация

**Тестовые сценарии**:
- `test_full_di_container_integration()` - Полная DI integration
- `test_di_container_performance_under_load()` - DI performance под нагрузкой
- `test_di_lifecycle_management()` - Component lifecycle через DI
- `test_error_propagation_through_di_chain()` - Error handling через DI

**SLA требования**:
- DI resolution: < 1ms average
- DI throughput: > 1000 resolutions/sec
- DI cache efficiency: > 70% hit rate

## 🛠️ Common Test Utilities (`common/mod.rs`)

**Shared functionality**:
- `TestConfigBuilder` - Test service creation
- `TestRecordBuilder` - Test data generation
- `PerformanceMeasurement` - Performance tracking
- `SlaValidator` - SLA compliance validation
- `TestDataGenerator` - Realistic test data
- `TestEnvironment` - Environment setup/teardown

## 🚀 Запуск тестов

### Запуск всех integration тестов
```bash
cd crates/memory
cargo test --test integration --features=integration-tests
```

### Запуск конкретного test suite
```bash
# Full system tests
cargo test --test integration::full_system_test

# Performance tests
cargo test --test integration::performance_test

# Resilience tests  
cargo test --test integration::resilience_test
```

### Запуск с детальным выводом
```bash
cargo test --test integration -- --nocapture --test-threads=1
```

## 📊 Покрытие SLA

| Метрика | Цель | Validation |
|---------|------|------------|
| **Search Latency** | < 5ms avg | ✅ Multiple test scenarios |
| **Throughput** | > 50 ops/sec | ✅ Production benchmarks |
| **Concurrent Ops** | 100+ support | ✅ Stress testing |
| **Circuit Breaker** | < 10% error rate | ✅ Failure scenarios |
| **Data Consistency** | > 95% preservation | ✅ Failure resilience |
| **Cache Efficiency** | > 50% hit rate | ✅ Performance monitoring |
| **DI Performance** | < 1ms resolution | ✅ Container testing |
| **Recovery Time** | < 5s full recovery | ✅ Error scenarios |

## 🔧 Конфигурация тестов

### Environment variables
```bash
# Test timeouts
INTEGRATION_TEST_TIMEOUT=300  # 5 minutes per test
PERFORMANCE_TEST_DURATION=30  # 30 seconds load tests

# Test scale
CONCURRENT_OPERATIONS=150     # Concurrent test scale
TEST_DATA_SIZE=2000          # Records for performance tests

# SLA thresholds
SEARCH_SLA_MS=5              # Search latency SLA
THROUGHPUT_SLA=50            # Operations per second SLA
```

### Features flags
```toml
[features]
integration-tests = ["dep:tempfile", "dep:futures"]
performance-tests = ["integration-tests", "dep:criterion"] 
resilience-tests = ["integration-tests"]
```

## 📈 Мониторинг результатов

Тесты генерируют comprehensive metrics:

- **Performance metrics**: latency P50/P95/P99, throughput
- **Reliability metrics**: success rates, error counts  
- **Resource metrics**: memory usage, cache efficiency
- **SLA compliance**: pass/fail для каждого SLA requirement

## ⚠️ Troubleshooting

### Общие проблемы:

1. **Timeout errors**: Увеличить `INTEGRATION_TEST_TIMEOUT`
2. **Memory pressure**: Уменьшить `TEST_DATA_SIZE` или `CONCURRENT_OPERATIONS`
3. **SLA violations**: Проверить system load и hardware specs
4. **DI resolution errors**: Проверить dependency configuration

### Debug режим:
```bash
RUST_LOG=debug cargo test --test integration -- --nocapture
```

## 🎖️ Success Criteria

Для production readiness все тесты должны проходить с следующими результатами:

- ✅ **All test suites pass**: 100% test pass rate
- ✅ **SLA compliance**: Все SLA requirements выполнены
- ✅ **Performance targets**: Sub-5ms search, 50+ ops/sec throughput
- ✅ **Resilience validation**: Graceful degradation под failures
- ✅ **Consistency guarantee**: Data integrity под stress

---

*Created by @rust-quality-guardian agent (agent_202508060258_q7k4)*  
*Last updated: 2025-08-06 04:15 UTC*