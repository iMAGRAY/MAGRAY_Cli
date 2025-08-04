# Core Concepts - Ключевые концепции проекта

> Лист архитектурного одуванчика - центральные концепции и ментальная модель MAGRAY CLI

[[_Architecture Hub - Центр архитектурной информации]] → Core Concepts

## 🧠 Ментальная модель MAGRAY

```mermaid
mindmap
  root((MAGRAY))
    Memory
      3 Layers
        Interact[Session Memory 24h]
        Insights[Knowledge 90d]
        Assets[Permanent]
      Vector Search
        HNSW Index
        <5ms latency
        Semantic similarity
      Promotion
        Time-based
        ML-based
        Access patterns
    
    Intelligence
      Embeddings
        BGE-M3 768D
        GPU/CPU
        Batch processing
      LLM Integration
        Multi-provider
        Retry logic
        Context window
      Reranking
        Neural scoring
        Relevance boost
    
    Execution
      Tool System
        File ops
        Git integration
        Web tools
      Agent Router
        Intent detection
        Multi-step planning
        Tool selection
      Safety
        Sandboxing
        Rate limiting
        Validation
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

Текст преобразуется в 768-мерные векторы для семантического поиска:

```mermaid
graph LR
    T["'How to auth?'"] --> Tok[Tokenizer]
    Tok --> E[BGE-M3 Model]
    E --> V["[0.23, -0.15, ...]"]
    V --> S[Semantic Space]
    
    style V fill:#f9f,stroke:#333,stroke-width:4px
```

**Принцип**: Близкие по смыслу тексты имеют близкие векторы.

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

**Принцип**: Иерархическая навигация от грубого к точному.

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

**Принцип**: Ценность определяется использованием и ML-оценкой.

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

**Принцип**: Максимальная производительность на доступном железе.

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

**Принцип**: От простого к сложному через декомпозицию.

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

## 🎯 Ключевые инварианты

1. **Данные всегда имеют embedding** - нет текста без вектора
2. **Слои изолированы** - прямой переход только вверх
3. **TTL строго соблюдается** - автоматическая очистка
4. **GPU fallback гарантирован** - работа на любом железе
5. **Контекст ограничен** - окно памяти для LLM

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