# System Overview - Обзор всей системы

> Лист архитектурного одуванчика - общий обзор архитектуры MAGRAY CLI

[[_Architecture Hub - Центр архитектурной информации]] → System Overview

## 🎯 Что такое MAGRAY?

MAGRAY CLI - это высокопроизводительный AI агент на Rust с трёхслойной системой памяти и векторным поиском. **Статус: 95% production ready** с HNSW индексами, ONNX моделями и GPU ускорением.

```mermaid
mindmap
  root((MAGRAY CLI))
    Core Features
      Vector Search(<5ms)
      3-Layer Memory
        Interact(24h)
        Insights(90d)
        Assets(∞)
      GPU Acceleration
        CUDA Support
        Auto Fallback
      Single Binary(16MB)
    
    Architecture
      8 Rust Crates
        cli
        memory
        ai
        llm
        router
        tools
        todo
        common
      HNSW Algorithm
      ONNX Models
        Qwen3(1024D)
        BGE-M3(1024D)
        Reranker
    
    Capabilities
      Multi-LLM Support
        OpenAI
        Anthropic
        Local Models
      Tool Execution
        File Ops
        Git Ops
        Web Ops
        Shell Ops
      Smart Routing
        Intent Analysis
        Task Planning
        DAG Execution
```

## 🏗️ Архитектурный обзор

MAGRAY CLI - это модульная система, построенная на принципах высокой производительности и отказоустойчивости. Система состоит из 8 специализированных Rust crates, каждый из которых отвечает за свою область функциональности.

```mermaid
graph TB
    subgraph "User Interface"
        CLI[CLI Layer]
        API[REST API*]
    end
    
    subgraph "Core Logic"
        UA[UnifiedAgent]
        Router[Smart Router]
        LLM[LLM Client]
        Tools[Tool Registry]
    end
    
    subgraph "Data Layer"
        Memory[Memory Service]
        VS[Vector Store]
        Cache[Embedding Cache]
    end
    
    subgraph "AI/ML Layer"
        Embed[Embedding Service]
        GPU[GPU Pipeline]
        Rerank[Reranker]
    end
    
    CLI --> UA
    API -.-> UA
    UA --> Router
    Router --> LLM
    Router --> Tools
    LLM --> Memory
    Tools --> Memory
    Memory --> VS
    Memory --> Cache
    Memory --> Embed
    Embed --> GPU
    Embed --> Rerank
```

## 📦 Основные компоненты

### 1. CLI Layer
**Ответственность**: Интерфейс командной строки
- Парсинг команд
- Отображение прогресса
- Обработка ввода/вывода

### 2. UnifiedAgent
**Ответственность**: Главный оркестратор
- Координация всех компонентов
- Управление жизненным циклом запроса
- Error handling и retry логика

### 3. Memory System
**Ответственность**: Управление памятью и контекстом
- 3-слойная архитектура (Interact → Insights → Assets)
- Векторный поиск через HNSW
- Автоматическое продвижение между слоями

### 4. AI Pipeline
**Ответственность**: Обработка AI/ML
- Генерация embeddings (BGE-M3)
- GPU ускорение с fallback на CPU
- Reranking результатов

## 🔄 Поток данных

### Типичный запрос пользователя

1. **CLI** принимает команду
2. **UnifiedAgent** анализирует intent
3. **Router** выбирает стратегию выполнения
4. **LLM/Tools** выполняют задачу
5. **Memory** сохраняет контекст
6. **CLI** отображает результат

### Поток векторного поиска

1. **Query** → текстовый запрос
2. **Embedding Service** → генерация вектора
3. **Vector Store** → HNSW поиск
4. **Reranker** → улучшение результатов
5. **Results** → отсортированные записи

## 🎯 Архитектурные принципы

### 1. Модульность
- Независимые crates с чёткими границами
- Минимальные зависимости между модулями
- Возможность замены компонентов

### 2. Производительность
- Zero-copy где возможно
- SIMD оптимизации для векторных операций
- Async/await для I/O операций

### 3. Отказоустойчивость
- Graceful degradation на всех уровнях
- Retry механизмы с exponential backoff
- Health monitoring и self-healing

### 4. Масштабируемость
- Горизонтальное масштабирование через sharding
- Вертикальное через GPU и оптимизации
- От embedded до cloud deployments

## 📊 Ключевые метрики

| Характеристика | Значение | Статус | Описание |
|---------------|----------|--------|----------|
| Production Ready | 95% | 🟢 | Готовность к продакшену |
| Binary Size | ~16MB | 🟢 | Release build |
| Startup Time | 150ms | 🟡 | Cold start (цель: 100ms) |
| Vector Search | <5ms | 🟢 | HNSW O(log n) поиск |
| Test Coverage | 35.4% | 🔴 | Текущее (цель: 80%) |
| GPU Support | Готово | 🟢 | CUDA + автофallback |

## 🏷️ Теги

#architecture #overview #system-design #leaf

---
[[_Architecture Hub - Центр архитектурной информации|← К центру архитектурного одуванчика]]