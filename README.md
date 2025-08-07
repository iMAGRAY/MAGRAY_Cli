# MAGRAY CLI — умный CLI-помощник для программирования

Многоплатформенная CLI‑программа, помогающая писать и поддерживать код. Работает как с облачными LLM API, так и с локальными моделями. Имеет гибкую систему инструментов (file/git/shell и плагины), умную память и использует qwen3 0.6b для эмбеддингов и реранкинга.

## Возможности
- Гибкие инструменты: чтение/запись файлов, git‑операции, shell, собственные плагины
- LLM: OpenAI/Groq/Anthropic или локальные (Ollama/LM Studio) — выбирайте провайдера
- Умная память: контекст проекта и сессий, быстрый поиск по эмбеддингам
- Эмбеддинги и реранкинг: qwen3 0.6b по умолчанию

## Установка
```bash
# Клонировать и собрать
git clone https://github.com/yourusername/MAGRAY_Cli
cd MAGRAY_Cli
cargo build --release
# бинарник: ./target/release/magray
```

## Настройка
```bash
cp .env.example .env
# укажите LLM провайдера и ключи, либо настройте локальные модели
```

Пример конфигурации (`~/.magray/config.toml`):
```toml
[ai]
embed_model = "qwen3-0.6b"
rerank_model = "qwen3-0.6b-reranker"
use_gpu = false

[ai.llm]
provider = "openai"     # или "ollama", "lmstudio", "groq", "anthropic"
model = "gpt-4o-mini"

[memory]
interact_ttl_hours = 24
insights_ttl_days  = 90
```

## Быстрый старт
```bash
# Проверка
./target/release/magray health

# Интерактивный чат
./target/release/magray chat

# Умная команда на естественном языке
./target/release/magray smart "проанализируй src/ и предложи рефакторинг"
```

## Основные команды
- `chat` — интерактивный диалог с ИИ
- `smart "…"` — умное выполнение задач (планирование шагов + инструменты)
- `tool "…"` — файловые/гит/шелл‑операции по описанию
- `memory …` — поиск/добавление/статистика памяти
- `models …` — управление локальными моделями
- `health` — быстрая самодиагностика

## Архитектура (кратко)
- `crates/cli` — команды и UX поверх инструментов/LLM/памяти
- `crates/llm` — абстракции над провайдерами LLM (API/локальные)
- `crates/ai` — эмбеддинги и реранкинг (qwen3 0.6b)
- `crates/memory` — умная память (векторный поиск, быстрая индексация)
- `crates/tools` — инструменты (file/git/shell) и расширения
- `crates/common` — общие утилиты, ошибки, трейты

## Примеры
```bash
# Работа с файлами
magray tool "создай файл hello.rs"
magray tool "покажи содержимое src/main.rs"

# Git и shell
magray tool "покажи git статус"
magray tool "запусти cargo test"

# Память
magray memory add "Важный эндпоинт: POST /api/users" --layer insights
magray memory search "эндпоинты"
```

## Документация
- docs/QUICKSTART.md — установка и первые шаги
- docs/MEMORY_SYSTEM_ARCHITECTURE.md — устройство памяти
- docs/conditional-compilation.md — фичи и сборочные профили

## Лицензия
MIT — см. файл LICENSE
