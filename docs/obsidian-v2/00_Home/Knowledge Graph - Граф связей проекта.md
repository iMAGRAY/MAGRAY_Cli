# Knowledge Graph

> Граф знаний и связей всего проекта MAGRAY

[[Home]] → Knowledge Graph

## 🌐 Полный граф знаний проекта

```mermaid
graph TB
    subgraph "🎯 Core Concepts"
        MAGRAY[MAGRAY CLI]
        MEMORY[3-Layer Memory]
        VECTOR[Vector Search]
        GPU[GPU Acceleration]
    end
    
    subgraph "🏗️ Architecture"
        CRATES[8 Rust Crates]
        LAYERS[System Layers]
        FLOW[Data Flow]
        PATTERNS[Design Patterns]
    end
    
    subgraph "🧩 Components"
        CLI_C[CLI Components]
        MEM_C[Memory Components]
        AI_C[AI Components]
        LLM_C[LLM Components]
        ROUTER_C[Router Components]
        TOOLS_C[Tools Components]
        TODO_C[Todo Components]
        COMMON_C[Common Components]
    end
    
    subgraph "💡 Features"
        SEARCH_F[Vector Search]
        GPU_F[GPU Acceleration]
        MEMORY_F[Memory Management]
        LLM_F[Multi-LLM Support]
        TOOLS_F[Tool Execution]
    end
    
    subgraph "📐 Concepts"
        EMBED[Embeddings]
        HNSW[HNSW Algorithm]
        PROMO[Promotion Logic]
        FALLBACK[Graceful Fallback]
    end
    
    %% Core connections
    MAGRAY --> MEMORY
    MAGRAY --> VECTOR
    MAGRAY --> GPU
    MAGRAY --> CRATES
    
    %% Architecture connections
    CRATES --> CLI_C
    CRATES --> MEM_C
    CRATES --> AI_C
    CRATES --> LLM_C
    CRATES --> ROUTER_C
    CRATES --> TOOLS_C
    CRATES --> TODO_C
    CRATES --> COMMON_C
    
    %% Component to Feature
    MEM_C --> SEARCH_F
    MEM_C --> MEMORY_F
    AI_C --> GPU_F
    AI_C --> EMBED
    LLM_C --> LLM_F
    TOOLS_C --> TOOLS_F
    
    %% Concept connections
    VECTOR --> HNSW
    VECTOR --> EMBED
    MEMORY --> PROMO
    GPU --> FALLBACK
    
    %% Cross connections
    SEARCH_F --> HNSW
    MEMORY_F --> PROMO
    GPU_F --> FALLBACK
    
    %% Flow connections
    LAYERS --> FLOW
    FLOW --> PATTERNS
    
    %% Highlighting critical paths
    style MAGRAY fill:#f96,stroke:#333,stroke-width:4px
    style MEMORY fill:#69f,stroke:#333,stroke-width:4px
    style VECTOR fill:#9f6,stroke:#333,stroke-width:4px
    style GPU fill:#f9f,stroke:#333,stroke-width:4px
```

## 🗺️ Навигационные пути

### Path 1: Понимание системы
```mermaid
flowchart LR
    START[Home] --> OVERVIEW[System Overview]
    OVERVIEW --> CONCEPTS[Core Concepts]
    CONCEPTS --> LAYERS[Memory Layers]
    LAYERS --> COMPONENTS[Memory Components]
    COMPONENTS --> IMPL[MemoryService]
    
    style START fill:#f96
    style CONCEPTS fill:#69f
    style IMPL fill:#9f6
```

### Path 2: Реализация функции
```mermaid
flowchart LR
    FEATURE[Vector Search Feature] --> CONCEPT[HNSW Concept]
    CONCEPT --> COMPONENT[VectorStore Component]
    COMPONENT --> CODE[Implementation]
    CODE --> TEST[Testing]
    CODE --> PERF[Performance]
    
    style FEATURE fill:#f9f
    style COMPONENT fill:#9ff
```

### Path 3: Решение проблем
```mermaid
flowchart LR
    ISSUE[Common Issues] --> CATEGORY{Problem Type}
    CATEGORY -->|Performance| PERF[Performance Guide]
    CATEGORY -->|GPU| GPU[GPU Issues]
    CATEGORY -->|Memory| MEM[Memory Issues]
    
    PERF --> BENCH[Benchmarks]
    GPU --> FALLBACK[Fallback Logic]
    MEM --> CONFIG[Configuration]
    
    style ISSUE fill:#f66
    style CATEGORY fill:#ff9
```

## 🧠 Ментальные связи

### Концептуальный граф

```mermaid
mindmap
  root((MAGRAY))
    Understanding
      Concepts
        Memory Layers
        Vector Search
        Embeddings
      Architecture
        8 Crates
        Data Flow
        Patterns
    
    Building
      Components
        Services
        Processors
        Managers
      Features
        Search
        GPU
        Tools
    
    Operating
      Configuration
      Deployment
      Monitoring
      Troubleshooting
    
    Extending
      Development
      Contributing
      Testing
      Documentation
```

## 📊 Статистика связей

### Структура одуванчиков

| Одуванчик | Листьев | Роль |
|-----------|---------|------|
| HOME | 2 | Главный центр навигации |
| ARCHITECTURE | 4 | Архитектурные концепции |
| COMPONENTS | 8 | Mind Map'ы всех crates |
| FEATURES | 6 | Ключевые возможности |

### Узловые точки знаний

```mermaid
graph TD
    subgraph "Primary Hubs"
        HOME[Home]
        MAP[Map of Content]
        ARCH[Architecture Hub]
        COMP[Components Hub]
        FEAT[Features Hub]
    end
    
    subgraph "Secondary Hubs"
        MEM[Memory Components]
        AI[AI Components]
        CLI[CLI Components]
    end
    
    subgraph "Concept Pages"
        CORE[Core Concepts]
        LAYERS[Memory Layers]
        FLOW[Data Flow]
    end
    
    HOME --> MAP
    MAP --> ARCH
    MAP --> COMP
    MAP --> FEAT
    
    COMP --> MEM
    COMP --> AI
    COMP --> CLI
    
    ARCH --> CORE
    CORE --> LAYERS
    ARCH --> FLOW
    
    style HOME fill:#f96,stroke:#333,stroke-width:4px
    style MAP fill:#69f,stroke:#333,stroke-width:4px
    style CORE fill:#9f6,stroke:#333,stroke-width:4px
```

## 🔍 Поисковые паттерны

### По типу информации

- **Концепции** → Через HOME найти ARCHITECTURE одуванчик
- **Компоненты** → Через HOME найти COMPONENTS одуванчик  
- **Возможности** → Через HOME найти FEATURES одуванчик

### По задаче

- **Начать работу** → [[Quick Start - Быстрый старт за 5 минут]]
- **Понять систему** → Через HOME → ARCHITECTURE → System Overview
- **Найти компонент** → Через HOME → COMPONENTS → нужный Mind Map
- **Изучить возможности** → Через HOME → FEATURES → конкретная возможность

## 🏷️ Теги знаний

### Основные категории
- `#concept` - концептуальные страницы
- `#component` - описания компонентов
- `#feature` - функциональность
- `#guide` - руководства
- `#reference` - справочная информация

### По уровню
- `#beginner` - для начинающих
- `#intermediate` - средний уровень
- `#advanced` - продвинутый уровень
- `#expert` - экспертный уровень

---
[[Home|← На главную]]