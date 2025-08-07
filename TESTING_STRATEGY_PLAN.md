# TESTING STRATEGY PLAN для MAGRAY CLI

**КРИТИЧЕСКАЯ СИТУАЦИЯ**: Memory crate не компилируется (38 ошибок), что блокирует измерение покрытия тестами.

## 🔥 КРИТИЧЕСКИЕ ПРОБЛЕМЫ (P0)

### Ошибки компиляции в memory crate
- **E0407**: method `is_ready`/`metrics` не найдены в trait `SearchCoordinatorTrait`
- **E0277**: trait bound `OperationExecutor: OperationExecutorTrait` не выполнен
- **E0433**: не найден `common` в crate root - проблемы с импортами
- **E0599**: методы не найдены в Arc wrappers
- **E0046**: неполная реализация trait SearchCoordinatorTrait

### ROOT CAUSE ANALYSIS
1. **Рефакторинг God Objects** привел к break API compatibility
2. **Trait implementations** не синхронизированы между модулями
3. **Import paths** сломаны после декомпозиции модулей
4. **Mock implementations** неполные в тестах

## 📊 ТЕКУЩЕЕ СОСТОЯНИЕ ТЕСТИРОВАНИЯ

### Инфраструктура (✅ ГОТОВО)
- **cargo-tarpaulin**: конфигурирован в tarpaulin.toml
- **cargo-llvm-cov**: настроен в .cargo/config.toml  
- **proptest**: включен в workspace dependencies
- **criterion**: настроен для benchmarking
- **mockall**: доступен для mocking

### Тестовые файлы (🟡 ЧАСТИЧНО)
- **52 тестовых файла** существуют в crates/memory/tests/
- **test_basic_isolated.rs**: создан изолированный тест
- **test_working_unit_tests.rs**: частично рабочие тесты
- **Большинство тестов НЕ РАБОТАЮТ** из-за compilation errors

### Coverage измерение (❌ ЗАБЛОКИРОВАНО)
- **Невозможно измерить покрытие** пока crate не компилируется
- **Baseline недоступен** для tracking прогресса

## 🎯 ПЛАН ИСПРАВЛЕНИЯ (Priority Order)

### ФАЗА 1: Исправление критических ошибок компиляции (P0)
**ETA: 2-3 дня**

1. **Исправить trait implementations**
   - Синхронизировать SearchCoordinatorTrait methods
   - Добавить недостающие implementations для OperationExecutor
   - Исправить Arc wrapper method calls

2. **Исправить import paths**  
   - Обновить пути после декомпозиции common
   - Исправить crate::common:: импорты
   - Синхронизировать module exports

3. **Исправить mock implementations**
   - Дополнить MockSearchCoordinator methods
   - Исправить test trait bounds
   - Обновить test fixtures

### ФАЗА 2: Baseline measurement (P1)
**ETA: 1 день после исправления компиляции**

1. **Измерить текущее покрытие**
   ```bash
   cargo llvm-cov --package memory --html --output-dir target/coverage
   ```

2. **Создать coverage baseline**
   - Документировать текущий уровень покрытия
   - Идентифицировать uncovered критические пути
   - Приоритизировать файлы для тестирования

### ФАЗА 3: Unit Tests для критических модулей (P1)
**ETA: 1-2 недели**

#### Приоритет 1: Core business logic
- **types.rs**: Layer, Record, SearchOptions (должно быть ~100% покрытие)
- **simd_optimized.rs**: Vector operations, cosine_distance функции
- **storage.rs**: VectorStore operations (store, search, delete)
- **cache_lru.rs**: LRU cache logic и eviction policies

#### Приоритет 2: DI и orchestration  
- **di/container_core.rs**: Dependency resolution logic
- **orchestration/memory_orchestrator.rs**: Coordination patterns
- **services/core_memory_service.rs**: Service composition

#### Приоритет 3: Advanced features
- **ml_promotion/**: ML algorithms for promotion decisions
- **hnsw_index/**: Vector indexing algorithms  
- **gpu_accelerated.rs**: GPU fallback mechanisms

### ФАЗА 4: Property-based testing (P2)
**ETA: 1 неделя**

#### HNSW Algorithm Properties
```rust
// Vector operations properties
proptest! {
    #[test]
    fn cosine_distance_triangle_inequality(
        a in vector_strategy(),
        b in vector_strategy(), 
        c in vector_strategy()
    ) {
        // d(a,c) ≤ d(a,b) + d(b,c)
    }
    
    #[test] 
    fn cosine_distance_symmetry(
        a in vector_strategy(),
        b in vector_strategy()
    ) {
        // d(a,b) = d(b,a)
    }
}
```

#### Cache Behavior Properties
```rust
proptest! {
    #[test]
    fn cache_capacity_never_exceeded(
        operations in cache_operation_strategy()
    ) {
        // cache.len() <= cache.capacity()
    }
}
```

### ФАЗА 5: Integration tests (P2)
**ETA: 1-2 недели**

#### End-to-End Workflows
1. **Store -> Search -> Promote**: Полный memory lifecycle
2. **Multi-layer operations**: Cross-layer data movement  
3. **Fallback scenarios**: GPU -> CPU degradation
4. **Circuit breaker**: Resilience patterns

#### Performance regression tests
```rust
criterion_group!(
    memory_benchmarks,
    bench_vector_search_performance,
    bench_batch_operations,
    bench_cache_hit_rates
);
```

### ФАЗА 6: Mock framework и test doubles (P3)
**ETA: 3-5 дней**

#### Service Mocking Strategy
- **External dependencies**: Database, GPU, filesystem
- **Network calls**: Embedding APIs, notification services  
- **System resources**: Memory allocators, thread pools
- **Time-dependent**: Promotion scheduling, cache TTL

## 🎯 COVERAGE TARGETS

### Target Coverage Levels
- **Overall project**: >30% (minimum viable)
- **Core business logic**: >80% (критически важно)
- **Critical paths**: >60% (store, search, promote)  
- **Utilities**: >40% (logging, config, helpers)

### Uncovered Code Strategy
- **Dead code elimination**: Удалить unused code  
- **Integration test coverage**: Покрыть через integration tests
- **Property-based coverage**: Покрыть edge cases через property tests

## 🛠️ TESTING TOOLS INTEGRATION

### Coverage measurement
```toml
# .cargo/config.toml additions
[alias]
test-coverage = "llvm-cov test --html --output-dir target/coverage"
test-unit = "test --lib --package memory" 
test-integration = "test --test '*' --package memory"
test-property = "test --release proptest"
```

### CI/CD Integration
```yaml
# .github/workflows/test-coverage.yml
name: Test Coverage
on: [push, pull_request]
jobs:
  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo llvm-cov --all-features --lcov --output-path lcov.info
      - uses: codecov/codecov-action@v4
        with:
          files: lcov.info
          fail_ci_if_error: true
```

## 🚨 БЛОКИРОВЩИКИ И RISKS

### Critical Blockers
1. **Memory crate compilation**: Все тестирование заблокировано
2. **API breaking changes**: Рефакторинг сломал compatibility  
3. **Missing trait implementations**: Mock objects неполные

### Risk Mitigation
- **Изоляция тестов**: Создавать isolated unit tests где возможно
- **Incremental fixing**: Исправлять по одному модулю за раз
- **API versioning**: Использовать facade pattern для compatibility

## ✅ ACCEPTANCE CRITERIA

### Минимальные требования (MVP)
- ✅ Memory crate компилируется без ошибок
- ✅ >30% overall test coverage достигнуто
- ✅ Core business logic покрыт unit tests
- ✅ Property-based tests для vector operations
- ✅ Integration tests для main workflows

### Желательные требования
- ✅ >50% overall coverage  
- ✅ Performance regression detection
- ✅ Automated coverage reporting в CI
- ✅ Comprehensive mock framework
- ✅ Fuzz testing для parsers

**NEXT STEPS**: 
1. Исправить compilation errors в memory crate (блокировщик #1)
2. Создать working unit tests для core types
3. Измерить baseline coverage после исправления
4. Implement systematic testing strategy по приоритетам