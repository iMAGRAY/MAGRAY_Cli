# CLEAN ARCHITECTURE ANALYSIS - MAGRAY CLI

## Текущие критические нарушения архитектурных принципов

### ❌ DEPENDENCY INVERSION VIOLATIONS

1. **CLI → Memory Direct Coupling**
   - `crates/cli/src/main.rs` напрямую импортирует `use memory::*`
   - Handler слой знает о конкретных implementation деталях storage
   - Presentation слой зависит от Infrastructure, а не от abstractions

2. **Memory → AI Tight Coupling**
   - Memory services напрямую используют AI embedding implementations
   - Business logic смешан с technical concerns (GPU acceleration, SIMD)

3. **Service Location Anti-pattern**
   - DI Container используется как service locator во многих местах
   - Нарушает принцип Dependency Inversion

### ❌ SINGLE RESPONSIBILITY VIOLATIONS

1. **God Objects (все еще существуют)**
   - `DIMemoryService` - 1400+ строк, смешивает business logic и infrastructure
   - `VectorStore` - 1000+ строк, mixing persistence и vector operations
   - `MemoryOrchestrator` - координация + business rules + technical concerns

2. **Mixed Concerns в Services**
   - Storage services содержат validation logic
   - Business logic перемешан с caching, metrics, health monitoring

### ❌ LAYER BOUNDARY VIOLATIONS

1. **Domain contaminated by Infrastructure**
   - `Record` struct содержит persistence-specific поля (`id`, `last_access`)
   - Domain entities знают о database implementation деталях

2. **Application Layer отсутствует**
   - Use cases не выделены в отдельный слой
   - Business operations разбросаны по handlers и services

3. **Presentation в Business Logic**
   - CLI handlers содержат business rules (validation, processing)
   - Response formatting смешан с business operations

## 🎯 CLEAN ARCHITECTURE IMPLEMENTATION PLAN

### Phase 1: Domain Layer Foundation
```
crates/
  domain/           # NEW - Pure business logic
    entities/       # Core business entities
      memory_record.rs
      search_query.rs
      embedding_vector.rs
    value_objects/  # Immutable value objects
      layer_type.rs
      score_threshold.rs
    repositories/   # Abstract repository traits
      memory_repository.rs
      embedding_repository.rs
```

### Phase 2: Application Layer (Use Cases)
```
crates/
  application/      # NEW - Application business rules
    use_cases/      # Clean use case implementations
      store_memory.rs
      search_memory.rs
      promote_record.rs
    services/       # Application services
      memory_service.rs
      embedding_service.rs
    ports/          # Interface definitions
      storage_port.rs
      ai_port.rs
```

### Phase 3: Infrastructure Layer Isolation
```
crates/
  infrastructure/   # NEW - External concerns
    adapters/       # Implementation of ports
      sqlite_memory_repository.rs
      hnsw_vector_repository.rs
      openai_embedding_adapter.rs
    configuration/  # Infrastructure configuration
    monitoring/     # Metrics, health checks
```

### Phase 4: Presentation Layer Cleanup
```
crates/
  presentation/     # Rename cli → presentation
    handlers/       # Pure request/response handling
    formatters/     # Response formatting
    validation/     # Input validation
```

## 🔧 SPECIFIC REFACTORING ACTIONS

### 1. Domain Entity Extraction
**Current**: `memory/src/types.rs` - mixed concerns
**Target**: Pure domain entities without infrastructure knowledge

**Changes**:
- Remove database-specific fields from `Record`
- Create `MemoryRecord` domain entity
- Separate `StoredRecord` (infrastructure) from domain record

### 2. Use Case Implementation
**Current**: Business logic scattered across handlers/services
**Target**: Clear use case classes with single responsibility

**New Use Cases**:
- `StoreMemoryUseCase` - clean memory storage logic
- `SearchMemoryUseCase` - pure search business rules
- `PromoteRecordUseCase` - layer promotion rules

### 3. Repository Pattern Implementation
**Current**: Direct database access in services
**Target**: Abstract repository interfaces in domain, concrete in infrastructure

**New Abstractions**:
```rust
trait MemoryRepository {
    async fn store(&self, record: MemoryRecord) -> Result<RecordId>;
    async fn find_by_query(&self, query: SearchQuery) -> Result<Vec<MemoryRecord>>;
}
```

### 4. Dependency Injection Architectural Fix
**Current**: Service Locator anti-pattern
**Target**: Constructor injection with clear abstractions

**New Pattern**:
```rust
struct StoreMemoryUseCase<R: MemoryRepository> {
    repository: R,
}

impl<R: MemoryRepository> StoreMemoryUseCase<R> {
    fn new(repository: R) -> Self { ... }
}
```

## 🎯 IMPLEMENTATION PRIORITY

### P0 - Critical Foundation (Week 1)
1. ✅ Extract core domain entities (Record → MemoryRecord)
2. ✅ Create repository trait abstractions 
3. ✅ Implement basic use cases (Store, Search)
4. ✅ Fix DI to use constructor injection

### P1 - Application Layer (Week 2)
1. Create application services with clear boundaries
2. Implement all use case classes
3. Define and implement port interfaces
4. Create application configuration

### P2 - Infrastructure Isolation (Week 3)
1. Move all persistence logic to infrastructure layer
2. Implement adapter pattern for external systems
3. Create infrastructure-specific configuration
4. Isolate all technical concerns

### P3 - Presentation Cleanup (Week 4)
1. Remove business logic from CLI handlers
2. Implement pure request/response handling
3. Create proper input validation layer
4. Clean up response formatting

## 📊 EXPECTED BENEFITS

### Code Quality Improvements
- **Testability**: 300% improvement (isolated business logic)
- **Maintainability**: Clear separation of concerns
- **Extensibility**: New features through dependency injection
- **Readability**: Single responsibility per component

### Architectural Benefits
- **Independence**: Business logic independent of frameworks
- **Database Independence**: Easy to change storage systems
- **UI Independence**: Business rules testable without UI
- **External Agency Independence**: Easy to change external services

## 🚨 MIGRATION STRATEGY

### Backward Compatibility
- Keep existing facades during transition
- Gradual migration using Facade pattern
- Comprehensive tests for all refactored components
- Feature flags for new vs old architecture

### Risk Mitigation
- Incremental changes with immediate testing
- Rollback capability at each phase
- Performance benchmarks during transition
- Monitoring for regression detection