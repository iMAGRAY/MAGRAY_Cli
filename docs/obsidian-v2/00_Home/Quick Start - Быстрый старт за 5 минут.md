# Quick Start

> Быстрое руководство по установке и первому запуску MAGRAY CLI

[[Home]] → Quick Start

## 📦 Установка

### Вариант 1: Скачать готовый бинарник

```bash
# Linux/macOS
curl -L https://github.com/your-org/magray-cli/releases/latest/download/magray-linux-amd64 -o magray
chmod +x magray
sudo mv magray /usr/local/bin/

# Windows
# Скачайте magray.exe из GitHub Releases
```

### Вариант 2: Установка через Cargo

```bash
# Требуется Rust 1.75+
cargo install magray-cli
```

### Вариант 3: Сборка из исходников

```bash
git clone https://github.com/your-org/magray-cli
cd magray-cli
cargo build --release
# Бинарник будет в target/release/magray
```

## ⚙️ Первоначальная настройка

### 1. Создайте файл конфигурации

```bash
# Создаст ~/.magray/config.toml
magray init
```

### 2. Настройте LLM провайдера

```toml
# ~/.magray/config.toml
[llm]
provider = "openai"  # или "anthropic", "local"
api_key = "sk-..."   # Ваш API ключ

[llm.model]
name = "gpt-4"
max_tokens = 2000
temperature = 0.7
```

### 3. Проверьте установку

```bash
# Проверка статуса системы
magray status

# Вывод:
# ✅ Memory system: healthy (3 layers active)
# ✅ Vector search: ready (HNSW index loaded)
# ✅ GPU support: available (CUDA 12.1)
# ✅ LLM provider: connected (OpenAI)
```

## 🚀 Первые команды

### Простой чат

```bash
magray chat "Привет! Как дела?"
```

### Поиск по контексту

```bash
# Добавить файлы в память
magray add ./src --recursive

# Искать по содержимому
magray search "функция авторизации"
```

### Умные задачи

```bash
# Многошаговая задача с планированием
magray smart "проанализируй этот код и предложи улучшения"
```

### Выполнение инструментов

```bash
# Прямое выполнение инструмента
magray tool shell "ls -la"
magray tool file read "./README.md"
```

## 🎯 Типичные сценарии использования

### Сценарий 1: Анализ кода

```bash
# Добавить проект в память
magray add . --project myapp

# Найти похожие паттерны
magray search "error handling" --project myapp

# Получить рекомендации
magray chat "как улучшить обработку ошибок в этом проекте?"
```

### Сценарий 2: Документирование

```bash
# Сгенерировать документацию
magray smart "создай README.md для этого проекта"

# Обновить существующую
magray smart "обнови документацию API в docs/"
```

### Сценарий 3: Рефакторинг

```bash
# Анализ и планирование
magray smart "проанализируй module.rs и предложи рефакторинг"

# Выполнение изменений
magray smart "примени предложенный рефакторинг"
```

## ⚡ Производительность

### GPU ускорение

```bash
# Проверить доступность GPU
magray gpu status

# Принудительно использовать CPU
MAGRAY_FORCE_CPU=1 magray chat "..."
```

### Настройка памяти

```toml
# ~/.magray/config.toml
[memory]
max_vectors = 1_000_000
cache_size_mb = 1024

[memory.layers]
interact_ttl = "24h"
insights_ttl = "7d"
assets_ttl = "permanent"
```

## 🔧 Troubleshooting

### Проблема: Медленный поиск

```bash
# Пересоздать индексы
magray maintenance reindex

# Оптимизировать память
magray maintenance optimize
```

### Проблема: Ошибки GPU

```bash
# Проверить CUDA
nvidia-smi

# Использовать CPU fallback
export MAGRAY_GPU_ENABLED=false
```

### Проблема: Нет соединения с LLM

```bash
# Проверить конфигурацию
magray config validate

# Тестовое соединение
magray llm test
```

## 📚 Следующие шаги

- **Возможности** → Через HOME → FEATURES одуванчик - что ещё умеет MAGRAY
- **Архитектура** → Через HOME → ARCHITECTURE одуванчик - понять как работает система

## 🏷️ Теги

#quickstart #installation #setup #getting-started

---
[[Home|← На главную]]