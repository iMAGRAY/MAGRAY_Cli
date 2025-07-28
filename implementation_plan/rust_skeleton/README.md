# MAGRAY CLI

Простой CLI чат с LLM моделями. Поддерживает OpenAI, Anthropic и локальные модели.

## 🚀 Быстрый старт

### 1. Сборка

```bash
cargo build --release
```

### 2. Настройка

Скопируйте пример конфигурации и настройте свой API ключ:

```bash
cp .env.example .env
# Отредактируйте .env файл
```

Пример `.env` для OpenAI:
```env
LLM_PROVIDER=openai
OPENAI_API_KEY=sk-your-api-key-here
OPENAI_MODEL=gpt-4o-mini
MAX_TOKENS=1000
TEMPERATURE=0.7
```

### 3. Использование

**Одиночное сообщение:**
```bash
./target/release/magray chat "Привет, как дела?"
```

**Интерактивный чат:**
```bash
./target/release/magray chat
```

## ⚙️ Поддерживаемые провайдеры

### OpenAI
```env
LLM_PROVIDER=openai
OPENAI_API_KEY=sk-your-key
OPENAI_MODEL=gpt-4o-mini  # или gpt-4, gpt-3.5-turbo
```

### Anthropic (Claude)
```env
LLM_PROVIDER=anthropic
ANTHROPIC_API_KEY=sk-ant-your-key
ANTHROPIC_MODEL=claude-3-haiku-20240307  # или claude-3-sonnet-20240229
```

### Локальные модели (LM Studio, Ollama с OpenAI API)
```env
LLM_PROVIDER=local
LOCAL_LLM_URL=http://localhost:1234/v1
LOCAL_LLM_MODEL=llama-3.2-3b-instruct
```

## 📝 Команды

- `magray chat [сообщение]` - отправить сообщение LLM
- `magray chat` - интерактивный режим (введите `exit` для выхода)
- `magray --help` - показать справку

## 🛠️ Настройки

В `.env` файле можно настроить:

- `MAX_TOKENS` - максимальное количество токенов в ответе (по умолчанию: 1000)
- `TEMPERATURE` - креативность модели 0.0-2.0 (по умолчанию: 0.7)

## 🔍 Логирование

Для отладки установите уровень логирования:
```bash
RUST_LOG=debug ./target/release/magray chat "тест"
```

## 🏗️ Архитектура (упрощенная версия)

Текущая версия сфокусирована на простоте:

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│    CLI      │───▶│    LLM      │───▶│  Provider   │
│  (clap)     │    │  (client)   │    │ (OpenAI/    │
│             │    │             │    │ Anthropic/  │
│             │    │             │    │ Local)      │
└─────────────┘    └─────────────┘    └─────────────┘
```

## 🎯 Что работает

✅ **CLI интерфейс** - команды чата  
✅ **LLM интеграция** - OpenAI, Anthropic, локальные модели  
✅ **Конфигурация** - через .env файл  
✅ **Интерактивный чат** - с выходом по `exit`  
✅ **Обработка ошибок** - корректные сообщения об ошибках  

## 🔧 Что НЕ реализовано (будущие версии)

❌ Память и контекст беседы  
❌ Планировщик задач  
❌ Векторный поиск  
❌ Инструменты (file_read, web_search, etc.)  
❌ 5-слойная архитектура памяти  

---

**Версия:** 0.1.0 - Минимальная рабочая версия  
**Лицензия:** MIT