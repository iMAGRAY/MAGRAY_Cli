# CLI Mind Map - Визуальная карта CLI crate

> Лист компонентного одуванчика - визуальная карта CLI crate и его компонентов

[[_Components Hub - Центр всех компонентов системы]] → CLI Mind Map

## 🧠 Полная карта CLI System

```mermaid
mindmap
  root((CLI System))
    Core Components
      UnifiedAgent[60%]
        Intent Analysis
        Router Integration
        LLM Orchestration
      StatusCommand[100%]
        System Diagnostics
        Health Checks
        Graceful Degradation
      AdaptiveProgress[95%]
        Smart Indicators
        Context-aware
        Performance
    
    Command System
      ChatCommand
        Interactive Mode
        Context Handling
        Response Formatting
      SmartCommand
        Multi-step Planning
        DAG Execution
        Progress Tracking
      ToolCommand
        Direct Execution
        Parameter Parsing
        Safety Checks
      GpuCommand[95%]
        GPU Info
        CUDA Detection
        Fallback Status
      ModelsCommand[100%]
        Model Management
        Download Control
        Registry Access
    
    User Interface
      AnimatedSpinner
        Progress States
        Smooth Updates
        Resource Aware
      ColoredOutput
        Syntax Highlighting
        Error Emphasis
        Success Indicators
      InteractiveMode
        Command History
        Auto-completion
        Context Preservation
    
    Infrastructure
      HealthChecks[100%]
        Component Status
        Dependency Checks
        Performance Metrics
      ErrorHandling
        Graceful Recovery
        User-friendly Messages
        Debug Information
      Configuration
        Environment Vars
        Config Files
        Defaults Management
```

## 🔗 Взаимосвязи компонентов

```mermaid
graph TB
    subgraph "Entry Layer"
        MAIN[main.rs]
        ARGS[CLI Args]
    end
    
    subgraph "Command Layer"
        CMD[Command Router]
        CHAT[Chat Handler]
        SMART[Smart Handler]
        TOOL[Tool Handler]
        GPU[GPU Handler]
        STATUS[Status Handler]
    end
    
    subgraph "Core Layer"
        UA[UnifiedAgent]
        PROG[Progress Manager]
        HEALTH[Health Monitor]
    end
    
    subgraph "Integration Layer"
        LLM[LLM Client]
        ROUTER[Smart Router]
        TOOLS[Tool Registry]
        MEM[Memory Service]
    end
    
    MAIN --> ARGS
    ARGS --> CMD
    
    CMD --> CHAT
    CMD --> SMART
    CMD --> TOOL
    CMD --> GPU
    CMD --> STATUS
    
    CHAT --> UA
    SMART --> UA
    TOOL --> UA
    
    UA --> LLM
    UA --> ROUTER
    UA --> TOOLS
    UA --> MEM
    
    STATUS --> HEALTH
    GPU --> HEALTH
    
    PROG -.-> CHAT
    PROG -.-> SMART
    PROG -.-> TOOL
    
    style MAIN fill:#f96,stroke:#333,stroke-width:4px
    style UA fill:#69f,stroke:#333,stroke-width:4px
    style HEALTH fill:#9f6,stroke:#333,stroke-width:4px
```

## 📊 Готовность компонентов

```mermaid
pie title "CLI Component Readiness"
    "Production Ready (90-100%)" : 5
    "High Ready (80-89%)" : 0
    "In Progress (60-79%)" : 1
    "Needs Work (<60%)" : 0
```

### Детальный статус

| Компонент | Готовность | Статус |
|-----------|------------|--------|
| StatusCommand | 100% | 🟢 Production |
| ModelsCommand | 100% | 🟢 Production |
| HealthChecks | 100% | 🟢 Production |
| GpuCommand | 95% | 🟢 Production |
| AdaptiveProgress | 95% | 🟢 Production |
| UnifiedAgent | 60% | 🟠 Active Dev |

## 🎯 Критические пути выполнения

### Path 1: Интерактивный чат

```mermaid
sequenceDiagram
    participant User
    participant CLI
    participant Agent
    participant LLM
    participant Memory
    
    User->>CLI: magray chat "question"
    CLI->>Agent: ParseIntent(question)
    Agent->>Memory: LoadContext()
    Memory-->>Agent: Context
    Agent->>LLM: Generate(question, context)
    LLM-->>Agent: Response
    Agent->>Memory: SaveInteraction()
    Agent-->>CLI: FormattedResponse
    CLI-->>User: ColoredOutput
```

### Path 2: Smart команда

```mermaid
flowchart LR
    INPUT[Smart Command] --> PARSE[Parse Task]
    PARSE --> PLAN[Create Plan]
    PLAN --> DAG[Build DAG]
    
    DAG --> EXEC{Execute Steps}
    EXEC --> S1[Step 1]
    EXEC --> S2[Step 2]
    EXEC --> S3[Step 3]
    
    S1 --> PROG1[Update Progress]
    S2 --> PROG2[Update Progress]
    S3 --> PROG3[Update Progress]
    
    PROG1 --> RESULT[Aggregate Results]
    PROG2 --> RESULT
    PROG3 --> RESULT
    
    RESULT --> OUTPUT[Format Output]
    
    style INPUT fill:#f96
    style DAG fill:#9f6
    style RESULT fill:#69f
```

### Path 3: Диагностика системы

```mermaid
graph TD
    STATUS[status command] --> COLLECT{Collect Info}
    
    COLLECT --> SYS[System Info]
    COLLECT --> COMP[Components]
    COLLECT --> PERF[Performance]
    COLLECT --> GPU[GPU Status]
    
    subgraph "Health Checks"
        COMP --> HC1[LLM Health]
        COMP --> HC2[Memory Health]
        COMP --> HC3[Vector Health]
        COMP --> HC4[Tool Health]
    end
    
    subgraph "Metrics"
        PERF --> M1[Response Time]
        PERF --> M2[Memory Usage]
        PERF --> M3[CPU Usage]
    end
    
    SYS --> REPORT[Status Report]
    HC1 --> REPORT
    HC2 --> REPORT
    HC3 --> REPORT
    HC4 --> REPORT
    M1 --> REPORT
    M2 --> REPORT
    M3 --> REPORT
    GPU --> REPORT
    
    REPORT --> FORMAT[Pretty Print]
    FORMAT --> USER[Display to User]
    
    style STATUS fill:#f9f
    style REPORT fill:#9ff
```

## 🎨 UI/UX компоненты

### Система прогресса

```mermaid
mindmap
  root((Progress System))
    Indicators
      Spinner
        ASCII art
        Smooth animation
        Context-aware
      Progress Bar
        Percentage
        ETA calculation
        Dynamic width
      Status Text
        Current operation
        Substeps
        Error messages
    
    Adaptivity
      Performance
        FPS limiting
        Resource monitoring
        Auto-disable
      Context
        Task complexity
        Terminal size
        User preferences
```

### Цветовая схема

```mermaid
graph LR
    subgraph "Output Types"
        SUCCESS[✓ Success] 
        ERROR[✗ Error]
        WARNING[⚠ Warning]
        INFO[ℹ Info]
        DEBUG[🔍 Debug]
    end
    
    subgraph "Colors"
        GREEN[Green]
        RED[Red]
        YELLOW[Yellow]
        BLUE[Blue]
        GRAY[Gray]
    end
    
    SUCCESS --> GREEN
    ERROR --> RED
    WARNING --> YELLOW
    INFO --> BLUE
    DEBUG --> GRAY
    
    style SUCCESS fill:#4f4
    style ERROR fill:#f44
    style WARNING fill:#ff4
    style INFO fill:#44f
    style DEBUG fill:#888
```

## 🚀 Паттерны использования

### Основные команды

```bash
# Интерактивный режим
magray

# Одиночный чат
magray chat "explain vector search"

# Умное выполнение
magray smart "refactor auth module and add tests"

# Прямой инструмент
magray tool "git status"

# Системная информация
magray status

# GPU информация
magray gpu info

# Управление моделями
magray models list
```

### Продвинутые сценарии

```mermaid
graph TD
    subgraph "Complex Task"
        T1[Analyze Code] --> T2[Plan Refactor]
        T2 --> T3[Execute Changes]
        T3 --> T4[Run Tests]
        T4 --> T5[Generate Report]
    end
    
    subgraph "Progress Tracking"
        P1[Start: 0%] --> P2[Analysis: 20%]
        P2 --> P3[Planning: 40%]
        P3 --> P4[Executing: 60%]
        P4 --> P5[Testing: 80%]
        P5 --> P6[Complete: 100%]
    end
    
    T1 -.-> P2
    T2 -.-> P3
    T3 -.-> P4
    T4 -.-> P5
    T5 -.-> P6
```

## 🔧 Точки расширения

### Добавление новых команд

```rust
// Структура для новой команды
pub struct NewCommand {
    agent: UnifiedAgent,
    progress: ProgressManager,
}

impl Execute for NewCommand {
    async fn run(&self, args: Args) -> Result<()> {
        // Implementation
    }
}
```

### Интеграция с агентом

```mermaid
flowchart LR
    NC[New Command] --> REG[Register in CLI]
    REG --> ROUTE[Add to Router]
    ROUTE --> AGENT[Connect to Agent]
    AGENT --> IMPL[Implement Logic]
    
    style NC fill:#f96
    style AGENT fill:#69f
```

## 🏷️ Теги

#cli #interface #mindmap #components #leaf

---
[[_Components Hub - Центр всех компонентов системы|← К центру компонентного одуванчика]]