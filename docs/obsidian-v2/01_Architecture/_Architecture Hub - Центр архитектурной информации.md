# Architecture Hub - Центр архитектурной информации

> Центральный узел одуванчика архитектуры MAGRAY CLI

[[Home]] → Architecture Hub

## Одуванчик ARCHITECTURE

### Листья архитектурного одуванчика

- [[System Overview - Обзор всей системы]] - Общий обзор архитектуры
- [[Core Concepts - Ключевые концепции проекта]] - Ключевые концепции и термины  
- [[Memory Layers - Трёхслойная архитектура памяти]] - Детали слоёв памяти
- [[Data Flow - Потоки данных через систему]] - Поток данных в системе

## 🏗️ Архитектурный обзор

MAGRAY CLI построен на принципах:

1. **Модульность** - 8 независимых crates в workspace
2. **Производительность** - Zero-copy где возможно, SIMD оптимизации
3. **Отказоустойчивость** - Graceful fallback на всех уровнях
4. **Масштабируемость** - От embedded до cloud deployments

```mermaid
graph TD
    CLI[CLI Layer] --> Router[Smart Router]
    Router --> LLM[LLM Client]
    Router --> Tools[Tool Registry]
    LLM --> Memory[Memory System]
    Memory --> AI[AI/Embeddings]
    AI --> GPU[GPU Pipeline]
    GPU -.->|fallback| CPU[CPU Backend]
```

## 🔑 Ключевые компоненты

### Ключевые архитектурные элементы

**Архитектурная информация сосредоточена в листьях этого одуванчика:**
- Детальные компоненты и их реализация описаны в концептах
- Практическое применение архитектуры доступно через HOME → другие одуванчики

## 📊 Реальные метрики архитектуры

| Характеристика | Значение | Статус | Описание |
|---------------|----------|--------|----------|
| Production Ready | 95% | 🟢 | Готовность к продакшену |
| Test Coverage | 35.4% | 🔴 | Тестовое покрытие (цель: 80%) |
| Crates | 8 | 🟢 | cli, memory, ai, llm, router, tools, todo, common |
| Слои памяти | 3 | 🟡 | Interact(24h), Insights(90d), Assets(∞) |
| Vector Search | <5ms | 🟢 | HNSW O(log n) через hnsw_rs |
| GPU Support | 100% | 🟢 | CUDA + автоматический CPU fallback |
| Binary size | ~16MB | 🟢 | Release build с оптимизациями |

## Навигация

Для перехода к другим областям используйте главный центр:
**HOME** → Выберите нужный одуванчик (Components, Features, или останьтесь в Architecture)

## 🏷️ Теги

#architecture #hub #center

---
[[Home|← К главному центру]]