# 🎯 КАК E2E СИСТЕМА ТЕСТИРОВАНИЯ ВЗАИМОДЕЙСТВУЕТ С MAGRAY CLI

## 📋 Реальные команды, которые использует система

### 1️⃣ **РЕЖИМ ОДИНОЧНЫХ КОМАНД** (Simple Mode)
Для простых запросов система запускает CLI с командой `chat` и сообщением:

```bash
# Компиляция MAGRAY CLI
cargo build -p cli --bin magray

# Запуск с простым сообщением
cargo run -p cli --bin magray -- chat "привет"

# Или если бинарник уже скомпилирован
./target/debug/magray.exe chat "привет"
```

**Примеры команд из сценариев:**
```bash
# Приветствие
cargo run -p cli --bin magray -- chat "привет"

# Просьба о помощи  
cargo run -p cli --bin magray -- chat "помоги мне"

# Проверка версии
cargo run -p cli --bin magray -- chat "какая версия?"

# Сложная задача (создание Rust API)
cargo run -p cli --bin magray -- chat "Создай новый Rust проект с API сервером, добавь аутентификацию JWT, создай несколько эндпоинтов для пользователей и продуктов, напиши unit тесты и integration тесты"
```

### 2️⃣ **ИНТЕРАКТИВНЫЙ РЕЖИМ** (Interactive Mode)
Для сложных сценариев система запускает CLI в интерактивном режиме:

```rust
// 1. Запуск CLI без аргументов (автоматически открывается чат)
cargo run -p cli --bin magray

// 2. CLI выводит приветствие:
// [★] Добро пожаловать в интерактивный режим с AgentOrchestrator!
// [►] Напишите ваше сообщение или
//     'exit' для выхода
// 
// 🤖 Вы: _

// 3. Система отправляет сообщение через stdin
stdin.write("привет\n")

// 4. Читает ответ из stdout
let response = stdout.read_line()
// Ответ: "Здравствуйте! Я MAGRAY CLI..."

// 5. Может продолжить диалог
stdin.write("проанализируй структуру проекта\n")
let response = stdout.read_line()

// 6. Завершение
stdin.write("exit\n")
```

### 3️⃣ **ПРОЦЕСС ТЕСТИРОВАНИЯ**

#### **Простой сценарий (basic_greeting):**
```bash
# Система выполняет:
1. cargo build -p cli --bin magray         # Компиляция
2. cargo run -p cli --bin magray -- chat "привет"  # Запуск с сообщением
3. Получает ответ из stdout
4. GPT-5 nano оценивает качество ответа
```

#### **Сложный сценарий (rust_api_creation):**
```bash
# Система выполняет:
1. cargo build -p cli --bin magray         # Компиляция
2. cargo run -p cli --bin magray           # Запуск интерактивного режима
3. Ждет приветствия CLI
4. Отправляет в stdin: "Создай новый Rust проект с API сервером..."
5. Читает многострочный ответ из stdout (с таймаутом 120 сек)
6. GPT-5 nano оценивает по критериям:
   - creates_project_structure
   - implements_jwt_auth
   - provides_multiple_endpoints
   - includes_comprehensive_tests
   - follows_rust_best_practices
```

### 4️⃣ **HEALTH CHECK КОМАНДЫ**
Для проверки работоспособности:

```bash
# Простая проверка
cargo run -p cli --bin magray -- --help

# Проверка версии
cargo run -p cli --bin magray -- --version

# Тестовое сообщение
cargo run -p cli --bin magray -- chat "test"
```

## 📊 ПОЛНЫЙ WORKFLOW E2E ТЕСТИРОВАНИЯ

```mermaid
graph TD
    A[Запуск E2E тестов] --> B[Компиляция MAGRAY CLI]
    B --> C{Тип сценария?}
    
    C -->|Простой| D[cargo run -- chat "message"]
    C -->|Сложный| E[cargo run + stdin/stdout]
    
    D --> F[Получение ответа]
    E --> G[Интерактивный диалог]
    
    F --> H[GPT-5 валидация]
    G --> H
    
    H --> I[Генерация отчета]
    I --> J[HTML/JSON/MD результаты]
```

## 🚀 ЗАПУСК ПОЛНОГО ТЕСТИРОВАНИЯ

```bash
# 1. Проверка готовности системы
cd tests
cargo run --bin magray_testing check

# 2. Запуск всех тестов
cargo run --bin magray_testing

# 3. Запуск конкретного типа тестов
cargo run --bin magray_testing -- --scenario-type complex_task

# 4. Запуск без GPT валидации (быстрее)
cargo run --bin magray_testing -- --skip-evaluation
```

## 📝 ПРИМЕРЫ РЕАЛЬНЫХ ВЗАИМОДЕЙСТВИЙ

### Пример 1: Простое приветствие
```
КОМАНДА: cargo run -p cli --bin magray -- chat "привет"
ОТВЕТ: Здравствуйте! Я MAGRAY CLI - интеллектуальный ассистент для разработчиков...
ОЦЕНКА GPT-5: 
  - relevance: 5/5
  - politeness: 5/5
  - helpful_context: 4/5
```

### Пример 2: Сложная задача
```
КОМАНДА: cargo run -p cli --bin magray
ВВОД: Создай REST API на Rust с JWT
ОТВЕТ: [300+ строк кода с Cargo.toml, main.rs, auth.rs, tests...]
ОЦЕНКА GPT-5:
  - technical_accuracy: 5/5
  - completeness: 5/5
  - best_practices: 4/5
  - documentation: 4/5
```

## ⚙️ ТЕХНИЧЕСКИЕ ДЕТАЛИ

- **Таймауты**: 10-150 секунд в зависимости от сложности
- **Ретраи**: До 3 попыток при сбоях
- **Буферы**: BufReader для stdout, writeln! для stdin
- **Определение конца ответа**: По маркерам "✓", "Done", "Вы:" или пустой строке
- **Windows совместимость**: Использует cmd.exe для запуска

## 🎯 ИТОГО

E2E система полностью автоматизирует:
1. **Компиляцию** MAGRAY CLI
2. **Запуск** в нужном режиме (chat или интерактивный)
3. **Отправку** тестовых сообщений
4. **Получение** и парсинг ответов
5. **Валидацию** качества через GPT-5 nano
6. **Генерацию** детальных отчетов

Система ведет себя как реальный пользователь, запускающий CLI и взаимодействующий с ним через терминал!