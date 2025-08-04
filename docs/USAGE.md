# MAGRAY CLI - Руководство пользователя

## 🚀 Быстрый старт

### Установка
```bash
# Клонировать репозиторий
git clone https://github.com/yourusername/MAGRAY_Cli.git
cd MAGRAY_Cli

# Собрать проект
cargo build --release

# Или использовать Makefile
make build
```

### Настройка окружения
Создайте файл `.env` в корне проекта:

```env
# LLM провайдер (openai, anthropic, local)
LLM_PROVIDER=openai

# OpenAI настройки
OPENAI_API_KEY=sk-proj-...
OPENAI_MODEL=gpt-4o-mini

# Или Anthropic
ANTHROPIC_API_KEY=sk-ant-...
ANTHROPIC_MODEL=claude-3-haiku-20240307

# Или локальная модель
LOCAL_LLM_URL=http://localhost:1234/v1
LOCAL_LLM_MODEL=llama-3.2-3b-instruct

# Общие настройки
MAX_TOKENS=1000
TEMPERATURE=0.7
RUST_LOG=info
```

## 📖 Основные команды

### Интерактивный режим
```bash
magray
# Запускает интерактивную сессию с анимированным интерфейсом
```

### Простой чат
```bash
magray chat "Что такое Rust?"
# Отправляет одиночный запрос в LLM
```

### Умный режим (многошаговое планирование)
```bash
magray smart "Создай веб-сервер на Rust с базой данных"
# Анализирует задачу и выполняет несколько шагов
```

### Работа с инструментами
```bash
# Чтение файла
magray tool file read src/main.rs

# Создание файла
magray tool file write test.txt "Hello, World!"

# Просмотр директории
magray tool dir list ./src

# Git операции
magray tool git status
magray tool git commit "Add new feature"

# Выполнение команд
magray tool shell exec "cargo test"

# Поиск в интернете
magray tool web search "Rust async programming"
```

### GPU управление
```bash
# Проверка статуса GPU
magray gpu status

# Информация о GPU
magray gpu info

# Бенчмарк GPU
magray gpu benchmark
```

### Управление моделями
```bash
# Список установленных моделей
magray models list

# Проверка моделей
magray models check

# Загрузка моделей
magray models download
```

### Системная информация
```bash
# Полный статус системы
magray status

# Версия
magray version
```

## 🎯 Продвинутые возможности

### Флаги командной строки
```bash
# Режим отладки
magray --debug chat "test"

# Тихий режим (минимум вывода)
magray --quiet status

# Без цветов (для CI/CD)
magray --no-color version
```

### Естественный язык для инструментов
MAGRAY понимает естественные команды:

```bash
# Вместо: magray tool file read test.txt
magray smart "покажи содержимое test.txt"

# Вместо: magray tool dir list ./src
magray smart "какие файлы в папке src?"

# Вместо: magray tool shell exec "mkdir new_folder"
magray smart "создай папку new_folder"
```

### Сложные задачи
```bash
# Многошаговые операции
magray smart "создай проект TODO приложения с README"

# Анализ и рефакторинг
magray smart "проанализируй код в src/ и предложи улучшения"

# Работа с Git
magray smart "проверь статус git и сделай коммит с описанием изменений"
```

## 💾 Система памяти

### Три уровня памяти:
1. **Interact** (24ч) - текущая сессия и контекст
2. **Insights** (90д) - извлеченные знания и паттерны  
3. **Assets** (∞) - код, документация, важные данные

### Просмотр памяти
```bash
# Статистика памяти (в разработке)
magray memory stats

# Поиск в памяти (в разработке)
magray memory search "async function"
```

## 🐳 Docker

### CPU версия
```bash
docker run -it --rm \
  -v $(pwd):/workspace \
  -e OPENAI_API_KEY=$OPENAI_API_KEY \
  magray:cpu chat "Hello"
```

### GPU версия (требует NVIDIA Docker)
```bash
docker run -it --rm --gpus all \
  -v $(pwd):/workspace \
  -e OPENAI_API_KEY=$OPENAI_API_KEY \
  magray:gpu smart "analyze performance"
```

## 🔧 Конфигурация

### Переменные окружения
- `RUST_LOG` - уровень логирования (trace, debug, info, warn, error)
- `MAGRAY_MODELS_DIR` - путь к моделям (по умолчанию ./models)
- `MAGRAY_CACHE_DIR` - путь к кэшу (по умолчанию ./cache)
- `NO_COLOR` - отключить цвета в выводе
- `MAGRAY_TIMEOUT` - таймаут операций в секундах (по умолчанию 300)

### Настройки моделей
Модели автоматически загружаются при первом использовании:
- BGE-M3 embeddings (768D)
- BGE reranker v2-m3
- Qwen3 embeddings (опционально)

## 📊 Мониторинг и отладка

### Логирование
```bash
# Подробные логи
RUST_LOG=debug magray status

# Только ошибки
RUST_LOG=error magray chat "test"

# Трассировка для отладки
RUST_LOG=trace magray smart "complex task"
```

### Производительность
```bash
# Бенчмарк векторного поиска
cargo bench --bench vector_search

# Профилирование памяти
RUST_LOG=info magray status --verbose
```

## ❓ Частые вопросы

### Ошибка "OPENAI_API_KEY not set"
Убедитесь что:
1. Файл `.env` существует в корне проекта
2. Переменная `OPENAI_API_KEY` установлена
3. Ключ начинается с `sk-proj-`

### Ошибка загрузки моделей
```bash
# Ручная загрузка моделей
make download-models

# Или через cargo
cargo run --bin download_models
```

### GPU не определяется
1. Проверьте драйверы NVIDIA
2. Установите CUDA toolkit
3. Запустите: `magray gpu status`

### Медленная работа на CPU
- Используйте меньшие модели
- Включите кэширование
- Рассмотрите GPU версию

## 🐛 Решение проблем

### Сброс кэша
```bash
rm -rf cache/
magray status
```

### Переустановка моделей
```bash
rm -rf models/
make download-models
```

### Отладочная информация
```bash
magray --debug status > debug.log 2>&1
```

## 📚 Дополнительные ресурсы

- [Архитектура проекта](ARCHITECTURE.md)
- [Руководство разработчика](DEVELOPER.md)
- [API документация](https://docs.rs/magray)
- [Примеры использования](../examples/)

## 🤝 Поддержка

- GitHub Issues: [github.com/yourusername/MAGRAY_Cli/issues](https://github.com/yourusername/MAGRAY_Cli/issues)
- Документация: [docs/](../docs/)
- Discord: [Присоединиться к сообществу](https://discord.gg/magray)