# ⚡ ПРИНЦИПЫ МИКРО-ДЕКОМПОЗИЦИИ

> **Методология разбиения сложных задач на управляемые 10-минутные блоки**

**🎯 Цель**: Трансформировать 74 больших задачи в **302 микро-задачи ≤10 минут** для psychologically manageable прогресса

---

## 🔧 ОСНОВНЫЕ ПРИНЦИПЫ

### ⚡ 10-МИНУТНОЕ ПРАВИЛО

**Максимум 10 минут** на одну задачу - основа всей методологии

**Почему 10 минут?**
- **Психологически комфортно** - любую задачу можно начать
- **Measurable progress** - четкие milestones каждые 10 минут  
- **Low mental overhead** - легко планировать и отслеживать
- **Risk mitigation** - минимальные потери при переключении контекста
- **Quality assurance** - достаточно времени для проверки результата

**Структура 10-минутной задачи**:
```
- [ ] **P1.2.3.a** [8м] Конкретное описание задачи  
  Шаги: Действие (2м) → Действие (3м) → Проверка (3м)
  Критерий: Конкретный измеримый результат
```

### 📋 АТОМАРНОСТЬ ЗАДАЧ

**Один задача = одно конкретное изменение**

**Принципы атомарности**:
- **Единственная ответственность**: задача решает одну проблему
- **Четкая границы**: понятно что входит и что НЕ входит
- **Независимость**: можно выполнить без других задач (кроме явных зависимостей)  
- **Проверяемость**: есть однозначный критерий завершения
- **Rollback capability**: можно отменить без влияния на другие компоненты

**Примеры атомарных задач**:
- ✅ ХОРОШО: "Создать IntentAnalyzer struct с analyze_intent() method"  
- ❌ ПЛОХО: "Реализовать полную систему анализа намерений"

### 🎯 ПРОВЕРЯЕМОСТЬ РЕЗУЛЬТАТОВ

**Четкий критерий завершения** для каждой задачи

**Типы критериев**:
- **Compilation**: "Код компилируется без ошибок"
- **Functional**: "Method возвращает ожидаемый результат"  
- **Integration**: "Компонент работает с existing системой"
- **Test**: "Unit test проходит"
- **Performance**: "Операция выполняется за <N мс"

**Формат критерия**:
```
Критерий: [Компонент] [действие] [измеримый результат]  
Пример: "IntentAnalyzer возвращает structured Intent из user input"
```

### 🔄 БУФЕРНОЕ ВРЕМЯ  

**20% времени на отладку/неожиданности** включено в каждую оценку

**Типы буферов**:
- **Задачные буферы**: каждая задача имеет +20% на debugging
- **Блочные буферы**: 15-20 минут на integration testing после блока
- **Фазовые буферы**: 15 минут между major фазами
- **Network буферы**: +50% для network-dependent задач

**Когда использовать буферы**:
- Неожиданные compilation errors
- Integration issues  
- Dependency conflicts
- Learning curve для новых технологий
- External service timeouts

---

## 📊 СТРУКТУРА ДЕКОМПОЗИЦИИ

### 🏗️ ИЕРАРХИЧЕСКАЯ ОРГАНИЗАЦИЯ

**4-уровневая структура** для organization и tracking

```
P0 (Фаза) 
├── P0.1 (Блок) - Policy Engine Security
│   ├── P0.1.1 (Подблок) - Изучение Policy Engine  
│   │   ├── P0.1.1.a [5м] Микро-задача
│   │   ├── P0.1.1.b [5м] Микро-задача  
│   │   └── P0.1.1.c [5м] Микро-задача
│   └── P0.1.BUFFER [15м] - Отладка блока
└── P0→P1 INTEGRATION BUFFER [15м]
```

### 📋 ШАБЛОН МИКРО-ЗАДАЧИ

**Стандартный формат** для consistency

```markdown
#### **P1.2.3.a** [8м] Создать компонент X с функциональностью Y  
- **Шаги**: Step 1 (2м) → Step 2 (3м) → Validation (3м)
- **Критерий**: Конкретный measurable результат
- **Статус**: ❌ NOT_STARTED / 🔄 IN_PROGRESS / ✅ COMPLETED
- **Файлы**: Список файлов которые будут изменены/созданы
- **Зависимости**: Список задач которые должны быть завершены первыми
```

### 🔗 УПРАВЛЕНИЕ ЗАВИСИМОСТЯМИ

**Explicit dependency tracking** для правильной последовательности

**Типы зависимостей**:
- **Архитектурные**: A должно быть создано перед B  
- **Функциональные**: B использует functionality из A
- **Data flow**: B needs data/structures созданные A
- **Integration**: B интегрируется с interface определенным A

**Правила зависимостей**:
- ❌ Реализация без архитектуры
- ❌ Тестирование без кода  
- ❌ Интеграция без компонентов
- ❌ Оптимизация без рабочей версии
- ❌ Деплой без пройденных тестов

---

## ⏰ ВРЕМЕННЫЕ ПРИНЦИПЫ

### 📅 REALISTIC TIME ESTIMATION

**Основанные на experience estimates** с учетом human factors

**Факторы влияющие на время**:
- **Complexity**: Сложность implementing
- **Familiarity**: Знакомство с технологией/domain  
- **Dependencies**: Количество integration points
- **Risk**: Вероятность неожиданных проблем
- **Context switching**: Overhead переключения между задачами

**Временные категории**:
- **5 минут**: Trivial changes (imports, simple config)
- **8 минут**: Standard tasks (struct creation, basic methods)  
- **10 минут**: Complex tasks (algorithm implementation, integration)
- **15+ минут**: РАЗБИТЬ НА БОЛЕЕ МЕЛКИЕ ЗАДАЧИ

### 🔄 INTEGRATION POINTS

**Специальные временные slots** для stability и quality

**Integration Buffer Pattern**:
```
Block Tasks [N minutes] → Block Buffer [15-20m] → Integration Buffer [15m] → Next Block
```

**Что происходит в буферах**:
- **Compilation verification** - убедиться что код компилируется
- **Basic testing** - smoke tests для new functionality  
- **Integration testing** - проверить что new код работает с existing
- **Error handling** - исправить unexpected issues
- **Documentation updates** - обновить docs if needed

---

## 🎯 КАЧЕСТВЕННЫЕ ПРИНЦИПЫ

### ✅ SUCCESS CRITERIA

**Конкретные измеримые** результаты для каждой задачи

**SMART Criteria Application**:
- **Specific**: Что exactly должно быть создано/изменено
- **Measurable**: Как проверить что задача завершена  
- **Achievable**: Реально выполнимо за отведенное время
- **Relevant**: Contributes к overall архитектурной цели  
- **Time-bound**: Четкая temporal граница (≤10 минут)

**Типы Success Criteria**:
- **Creation**: "Component X создан с methods Y и Z"
- **Integration**: "Component X работает с existing system Y"  
- **Functionality**: "Method X возвращает expected результат для input Y"
- **Performance**: "Operation X выполняется за <N milliseconds"
- **Quality**: "Code passes linting без warnings"

### 🔧 EXECUTION RULES

**Обязательные steps** для каждой задачи

1. **Pre-execution**:
   - Проверить зависимости выполнены
   - Прочитать файлы которые будут изменяться
   - Понять integration points

2. **During execution**:  
   - Следовать documented шагам
   - Тестировать incremental changes
   - Document significant decisions

3. **Post-execution**:
   - Verify success criteria met
   - Run compilation/tests где applicable  
   - Update status и dependencies

### 📈 PROGRESS TRACKING

**Granular visibility** в progress для motivation и planning

**Tracking Metrics**:
- **Tasks completed per hour** - productivity measurement
- **Success rate** - percentage of tasks completed on first try
- **Buffer utilization** - как часто нужны буферы  
- **Dependency accuracy** - правильность dependency specification
- **Time estimation accuracy** - actual vs estimated time

**Progress Visualization**:
```
Phase: P1 Core [55%] ████████████▓▓▓▓▓▓▓▓▓▓▓▓ 23/42 tasks
Block: Multi-Agent [12%] ██▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓▓ 2/16 tasks  
Task: P1.1.2.a [IN_PROGRESS] ████████▓▓ 8/10 minutes
```

---

## 🚨 RISK MITIGATION

### ⚠️ ИЗВЕСТНЫЕ РИСКИ И COUNTERMEASURES  

**Systematic approach** к handling common problems

**Risk Categories**:
- **Technical**: Compilation errors, dependency conflicts
- **Architectural**: Design decisions, integration complexity
- **Temporal**: Time estimation errors, scope creep
- **Human**: Context switching, fatigue, motivation loss

**Mitigation Strategies**:
- **Buffer time**: 20% padding для unexpected issues
- **Dependency validation**: Проверка prerequisites before starting  
- **Atomic commits**: Rollback points на task boundaries
- **Documentation**: Decision tracking для future reference
- **Regular breaks**: Avoid fatigue-induced mistakes

### 🔄 ROLLBACK POINTS

**Safe restoration points** на major milestone boundaries

**Rollback Strategy**:
- **Task level**: Each task creates atomic change
- **Block level**: Block buffers provide integration checkpoints  
- **Phase level**: Integration buffers provide major rollback points
- **System level**: Git commits на significant milestones

**When to Rollback**:  
- Task consistently failing после multiple attempts
- Integration issues affecting multiple components
- Architectural decision proven incorrect  
- Time overruns threatening overall timeline

---

## 💡 BEST PRACTICES

### 🎯 EFFECTIVE TASK DESIGN

**Patterns для creating good микро-задачи**

**Good Task Patterns**:
- "Create X with methods Y and Z" - clear scope и deliverables  
- "Integrate X with Y using interface Z" - explicit integration contract
- "Implement algorithm X with performance requirement Y" - measurable outcome
- "Add validation for X with error handling Y" - clear functionality scope

**Avoid These Patterns**:
- "Fix all issues in module X" - scope too broad и undefined
- "Implement better Y" - subjective criteria, no clear completion
- "Optimize performance" - need specific metrics и targets  
- "Improve user experience" - need concrete features и measurements

### 🔧 IMPLEMENTATION GUIDELINES  

**Practical advice** для executing микро-декомпозированные задачи

**Before Starting**:
- Read all dependent task outputs
- Understand integration requirements
- Check current system state  
- Plan step-by-step approach

**During Work**:
- Focus только на current task scope
- Test incremental changes frequently
- Document key decisions briefly
- Ask for help если stuck >5 minutes  

**After Completion**:
- Verify success criteria completely  
- Test integration points
- Update task status immediately  
- Note any lessons learned

---

## 🔗 Связанные разделы

- **Критерии успеха**: [success-criteria.md](success-criteria.md) - detailed success definitions
- **Временные оценки**: [time-estimates.md](time-estimates.md) - comprehensive time planning  
- **Critical блокеры**: [../blockers/critical-blockers.md](../blockers/critical-blockers.md) - applying principles to urgent tasks
- **Progress метрики**: [../progress/metrics.json](../progress/metrics.json) - tracking implementation

---

*⚡ Микро-декомпозиция transforms overwhelming projects into manageable, trackable progress with predictable outcomes*