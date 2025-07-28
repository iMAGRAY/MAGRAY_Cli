# Task memory-system-implementation: Реализация 5-слойной системы памяти

## Goal
Реализовать полную 5-слойную систему памяти (M0-M4) с семантическим роутингом и координацией между слоями согласно архитектуре из ARCHITECTURE.md

## Priority
CRITICAL - разблокирует executor, agent loop, и семантический поиск

## Source
- Architecture: implementation_plan/docs/ARCHITECTURE.md
- Schema: implementation_plan/db/schema.sql
- INCOMPLETE.md: критические недоработки memory координации
- Created: 2025-07-28 15:30:00 UTC
- Assignee: BackgroundAgent

## Constraints
- Все данные локально в ~/.ourcli/projects/<id>/
- M4 (семантический слой) - фронт-дверь ко всем остальным
- Автоматический промоушен M0→M1→M2→M3→M4
- BGE3 для эмбеддингов, BG3 для reranking
- SQLite для M1/M2, файловая система для M3, векторная БД для M4

## Plan (DAG)
- [x] Step 1: Создать memory crate с базовыми трейтами
- [ ] Step 2: Реализовать M0 (Ephemeral) - RAM хранилище
- [ ] Step 3: Реализовать M1 (ShortTerm) - SQLite KV
- [ ] Step 4: Реализовать M2 (MediumTerm) - SQLite таблицы
- [ ] Step 5: Реализовать M3 (LongTerm) - файловое хранилище
- [ ] Step 6: Реализовать M4 (Semantic) - векторный индекс
- [ ] Step 7: Создать MemoryCoordinator для управления слоями
- [ ] Step 8: Реализовать SemanticRouter с BGE3/BG3
- [ ] Step 9: Добавить автоматический промоушен данных
- [ ] Step 10: Интегрировать с executor и остальной системой
- [ ] Step 11: Добавить тесты и документацию

## Execution Log
### Step 1: Create Memory Crate Structure
Action: Создаю базовую структуру memory crate с трейтами и типами
Result: Создан memory crate с основными интерфейсами
Checks: ✅ compilation ✅ basic structure
Next: Реализовать M0 Ephemeral слой