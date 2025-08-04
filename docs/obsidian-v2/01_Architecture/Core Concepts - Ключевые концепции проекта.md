# Core Concepts - Ключевые концепции проекта

> Лист архитектурного одуванчика - центральные концепции и ментальная модель MAGRAY CLI

[[_Architecture Hub - Центр архитектурной информации]] → Core Concepts

## 🧠 Ментальная модель MAGRAY

**Реальный статус: 95% production ready, 35.4% test coverage**

```mermaid
mindmap
  root((MAGRAY CLI))
    Memory System[70%]
      3 Layers
        Interact[Session Memory 24h]
        Insights[Knowledge 90d]
        Assets[Permanent]
      Vector Search[85%]
        HNSW Index
        5ms latency
        hnsw_rs library
      Promotion[75%]
        Time-based rules
        ML scoring engine
        BTreeMap indices
    
    AI Pipeline[90%]
      Embeddings[100%]
        Qwen3 1024D
        GPU/CPU fallback
        ONNX runtime
      LLM Integration[80%]
        OpenAI/Anthropic
        Retry logic
        Error handling
      Reranking[90%]
        BGE reranker v2-m3
        Neural scoring
    
    Execution[60%]
      Tool System[90%]
        File operations
        Git integration
        Web tools
      Agent Router[70%]
        Intent detection
        Basic planning
        UnifiedAgent
      Safety[80%]
        Parameter validation
        Error boundaries
        Timeout handling
```

## 🔑 Ключевые концепции

### 1. Трёхслойная память

Память в MAGRAY организована как иерархия с разными TTL:

```mermaid
graph TB
    subgraph "User Interaction"
        U[User Query] --> I[Interact Layer]
    end
    
    subgraph "Memory Hierarchy"
        I -->|24h TTL| I
        I -->|Promotion| IN[Insights Layer]
        IN -->|90d TTL| IN
        IN -->|Promotion| A[Assets Layer]
        A -->|Permanent| A
    end
    
    subgraph "Lifecycle"
        I -.->|Expire| X[Deletion]
        IN -.->|Expire| X
    end
```

**Принцип**: Информация продвигается вверх по мере важности и использования.

### 2. Векторные представления (Embeddings)

Текст преобразуется в 1024-мерные векторы через ONNX модель Qwen3:

```mermaid
graph LR
    T["'How to auth?'"] --> Tok[ONNX Tokenizer]
    Tok --> E[Qwen3 Model]
    E --> V["[0.23, -0.15, ...1024D]"]
    V --> GPU{GPU Available?}
    GPU -->|Yes| CUDA[GPU Processing]
    GPU -->|No| CPU[CPU Fallback]
    CUDA --> Cache[Embedding Cache]
    CPU --> Cache
    
    style V fill:#f9f,stroke:#333,stroke-width:4px
    style CUDA fill:#0f0,stroke:#333,stroke-width:2px
```

**Принцип**: Близкие по смыслу тексты имеют близкие векторы. Автоматический fallback CPU↔GPU.

### 3. Граф навигации (HNSW Algorithm)

HNSW создаёт многослойный граф для быстрого поиска:

```mermaid
graph TD
    subgraph "Layer 2 (Highway)"
        L2A[Node] -.-> L2B[Node]
        L2B -.-> L2C[Node]
    end
    
    subgraph "Layer 1 (Regional)"
        L1A[Node] --> L1B[Node]
        L1B --> L1C[Node]
        L1C --> L1D[Node]
    end
    
    subgraph "Layer 0 (Local)"
        L0A[Node] --> L0B[Node]
        L0B --> L0C[Node]
        L0C --> L0D[Node]
        L0D --> L0E[Node]
    end
    
    L2A -.-> L1A
    L2B -.-> L1C
    L1A -.-> L0A
    L1B -.-> L0C
    L1D -.-> L0E
```

**Принцип**: Иерархическая навигация O(log n) через hnsw_rs библиотеку. Реальная производительность: <5ms.

### 4. Механизм продвижения (Promotion Engine)

Автоматическое продвижение ценной информации:

```mermaid
stateDiagram-v2
    [*] --> Interact: New Record
    
    Interact --> Insights: High Access Count
    Interact --> Insights: ML Score > 0.8
    Interact --> [*]: TTL Expired
    
    Insights --> Assets: Critical Info
    Insights --> Assets: User Tagged
    Insights --> [*]: TTL Expired
    
    Assets --> Assets: Permanent
```

**Принцип**: Ценность определяется использованием и ML-оценкой. Статус: 75% готовности, используются BTreeMap индексы.

### 5. Адаптивное ускорение (GPU Acceleration)

Автоматический выбор оптимального устройства:

```mermaid
flowchart LR
    R[Request] --> D{Detect GPU}
    D -->|Available| G[GPU Pipeline]
    D -->|Not Available| C[CPU Pipeline]
    
    G --> B1[Batch Queue]
    B1 --> GP[GPU Process]
    GP --> O[Output]
    
    C --> MT[Multi-thread]
    MT --> SIMD[SIMD Optimize]
    SIMD --> O
    
    style G fill:#0f0,stroke:#333,stroke-width:2px
    style C fill:#ff0,stroke:#333,stroke-width:2px
```

**Принцип**: Максимальная производительность на доступном железе. Статус: 100% готово с automatic fallback.

### 6. Агентная архитектура (Agent Intelligence)

Умная маршрутизация и планирование:

```mermaid
graph TD
    Q[User Query] --> IA[Intent Analysis]
    
    IA --> Simple{Simple?}
    Simple -->|Yes| Direct[Direct Response]
    Simple -->|No| Plan[Multi-step Planning]
    
    Plan --> DAG[Task DAG]
    DAG --> T1[Task 1]
    DAG --> T2[Task 2]
    DAG --> T3[Task 3]
    
    T1 --> Tools[Tool Execution]
    T2 --> LLM[LLM Processing]
    T3 --> Memory[Memory Search]
    
    Tools --> Combine[Combine Results]
    LLM --> Combine
    Memory --> Combine
    
    Combine --> Response[Final Response]
```

**Принцип**: От простого к сложному через декомпозицию. Статус: 60% готовности, базовая маршрутизация работает.

## 🔄 Жизненный цикл данных

### От ввода к постоянному хранению

```mermaid
journey
    title Путь информации в MAGRAY
    section Input
      User Query: 5: User
      Parse Intent: 3: Agent
    section Processing
      Generate Embedding: 3: AI
      Search Similar: 4: Memory
      Execute Tools: 3: Tools
    section Storage
      Save to Interact: 5: Memory
      Analyze Value: 3: ML
      Promote to Insights: 4: Engine
    section Long-term
      Tag as Asset: 2: User
      Permanent Storage: 5: Memory
```

## 🎯 Ключевые инварианты (реальное состояние)

1. **Данные всегда имеют embedding** - Qwen3 1024D векторы ✅
2. **Слои изолированы** - прямой переход только вверх ✅
3. **TTL строго соблюдается** - автоматическая очистка ⚠️ (в разработке)
4. **GPU fallback гарантирован** - CUDA→CPU автоматически ✅
5. **Контекст ограничен** - окно памяти для LLM ⚠️ (базовая реализация)
6. **Тестовое покрытие 35.4%** - критически низкое 🔴

## 🔗 Углубление в концепции

### Связанные темы

**Все детали доступны через центры одуванчиков:**
- **Архитектурные детали** → Через ARCHITECTURE Hub → Memory Layers
- **Компоненты реализации** → Через HOME → COMPONENTS одуванчик
- **Практические возможности** → Через HOME → FEATURES одуванчик

## 🏷️ Теги

#concepts #architecture #mental-model #leaf

---
[[_Architecture Hub - Центр архитектурной информации|← К центру архитектурного одуванчика]]