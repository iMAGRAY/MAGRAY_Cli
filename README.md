# MAGRAY_Cli Workspace

## Команда
- **Амир** - Developer
- **Альтаир** - AI Assistant (Claude 4 Opus)

## Структура проекта
```
.
├── .cursor/
│   └── rules/              # Правила для Cursor IDE
│       ├── general/        # Общие правила работы
│       ├── rust/           # Rust-специфичные правила
│       └── cli-development/# Правила для CLI разработки
├── docs/
│   └── CLI_ARCHITECTURE.md # Архитектурные решения для CLI
├── templates/
│   └── cli-starter/        # Шаблон для новых CLI проектов
├── scripts/
│   └── new-cli.sh          # Скрипт создания нового CLI
├── PROJECT_CONTEXT.md      # Контекст и статус проекта
├── README.md               # Этот файл
└── .gitignore              # Git игнор файлы
```

## 🚀 Быстрый старт CLI проекта

### Создать новый CLI проект:
```bash
./scripts/new-cli.sh my-awesome-cli "Your Name" "Description of your CLI"
```

### Что включено в шаблон:
- ✅ Полная структура Rust CLI проекта
- ✅ Настроенный Cargo.toml с оптимальными зависимостями
- ✅ Базовая архитектура команд (init, create, analyze, config)
- ✅ Интерактивное меню
- ✅ Цветной вывод и progress bars
- ✅ CI/CD через GitHub Actions
- ✅ Готовая система логирования

## 📚 Документация по CLI разработке

См. [docs/CLI_ARCHITECTURE.md](docs/CLI_ARCHITECTURE.md) для:
- Архитектурных паттернов
- UX/UI принципов
- Технического стека
- Системы плагинов
- Best practices

## 🛠️ Рекомендуемый стек для CLI

### Rust (основной выбор)
- **clap** - парсинг аргументов
- **tokio** - async runtime
- **indicatif** - progress bars
- **dialoguer** - интерактивные промпты
- **colored** - цветной вывод

### Альтернативы
- **Go**: cobra + bubbletea
- **Python**: typer + rich
- **Node.js**: oclif + ink

## Как работаем

### 1. Описание задачи
Амир описывает, что нужно сделать

### 2. Обсуждение решений
Альтаир предлагает варианты с анализом плюсов/минусов

### 3. Выбор подхода
Вместе выбираем лучшее решение

### 4. Реализация
Пишем код с тестами и документацией

## Текущий статус
См. [PROJECT_CONTEXT.md](PROJECT_CONTEXT.md)

---
*Начало: 2025-01-07*