# MAGRAY CLI - TODO Структура

## 🎯 ЧТО ЗДЕСЬ
- **302 микро-задачи** разделены по папкам
- **Каждая задача ≤10 минут** для управляемости  
- **4 критических блокера** требуют немедленного внимания

## 📂 СТРУКТУРА

### 🚨 НАЧАТЬ ЗДЕСЬ
**[blockers/](blockers/)** - Критические блокеры MVP
- **[critical-blockers.md](blockers/critical-blockers.md)** - 4 блокера (29ч)
- **[integration-buffers.md](blockers/integration-buffers.md)** - Буферы

### 📋 ОСНОВНЫЕ ФАЗЫ  
**[phases/](phases/)** - Архитектурные фазы
- **[p0-security.md](phases/p0-security.md)** - Security: 31 задача (85%)
- **[p1-core.md](phases/p1-core.md)** - Core: 42 задачи (55%)  
- **[p1-plus-ux.md](phases/p1-plus-ux.md)** - UX: 22 задачи (5%)
- **[p2-enhancement.md](phases/p2-enhancement.md)** - Polish: 24 задачи (10%)

### 📊 ОТСЛЕЖИВАНИЕ
**[progress/](progress/)** - Метрики и результаты
- **[metrics.json](progress/metrics.json)** - Числовые метрики
- **[completed-tasks.md](progress/completed-tasks.md)** - Завершённые
- **[audit-results.md](progress/audit-results.md)** - Аудиты

### 🏗️ СПРАВОЧНИКИ  
**[architecture/](architecture/)** - Принципы разработки
- **[principles.md](architecture/principles.md)** - Микро-декомпозиция
- **[success-criteria.md](architecture/success-criteria.md)** - Критерии  
- **[time-estimates.md](architecture/time-estimates.md)** - Оценки времени

### 🗃️ ИНДЕКСЫ
**[metadata/](metadata/)** - Связи и индексы
- **[task-index.json](metadata/task-index.json)** - Все 302 задачи
- **[dependencies.json](metadata/dependencies.json)** - Зависимости

## ⚡ БЫСТРЫЙ СТАРТ

1. **БЛОКЕРЫ** → [critical-blockers.md](blockers/critical-blockers.md) (29ч до MVP)
2. **SECURITY** → [p0-security.md](phases/p0-security.md) (осталось 15%)  
3. **CORE** → [p1-core.md](phases/p1-core.md) (нужно 45%)

## 📖 ЛЕГЕНДА
- **🚨** = критический блокер
- **✅** = завершено  
- **🔄** = в работе
- **❌** = не начато
- **⚠️** = частично

## 🎯 ПРАВИЛА
1. **Блокеры первыми** - разблокируют функциональность
2. **≤10 минут на задачу** - управляемые куски
3. **Проверять зависимости** - см. metadata/
4. **Отмечать прогресс** - обновлять в progress/

---
*302 задачи • MVP за 29 часов • 35% готово*