# MAGRAY CLI — Быстрый старт

## Установка
```bash
git clone https://github.com/yourusername/MAGRAY_Cli
cd MAGRAY_Cli
cargo build --release
# бинарник: ./target/release/magray
```

## Настройка
```bash
cp .env.example .env
# Укажите LLM провайдера и ключ (или используйте локальные модели)
```

Пример `~/.magray/config.toml`:
```toml
[ai]
embed_model = "qwen3-0.6b"
rerank_model = "qwen3-0.6b-reranker"
use_gpu = false

[ai.llm]
provider = "openai"      # или "ollama", "lmstudio", "groq", "anthropic"
model    = "gpt-4o-mini"
```

## Первый запуск
```bash
magray health
magray chat
magray smart "проанализируй src/ и предложи рефакторинг"
```

## Полезные команды
```bash
# Инструменты
magray tool "создай файл hello.rs"
magray tool "покажи git статус"
magray tool "запусти cargo test"

# Память
magray memory add "Важный эндпоинт: POST /api/users" --layer insights
magray memory search "эндпоинты"
```

## Подсказки
- Используйте переменные окружения (`.env`) для ключей провайдеров
- Для локальных моделей настройте провайдер `ollama` или `lmstudio`
- Память поможет быстро находить факты проекта (эмбеддинг и реранкинг на qwen3 0.6b)