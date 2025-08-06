# 🚀 MAGRAY CLI - Быстрый старт

**5 минут до первого запуска!**

---

## 📦 Установка (1 минута)

### Вариант 1: Скачать готовый бинарник

```bash
# Linux/macOS
curl -L https://github.com/yourusername/MAGRAY_Cli/releases/latest/download/magray-$(uname -s)-$(uname -m) -o magray
chmod +x magray
sudo mv magray /usr/local/bin/

# Windows (PowerShell)
Invoke-WebRequest -Uri "https://github.com/yourusername/MAGRAY_Cli/releases/latest/download/magray-windows-amd64.exe" -OutFile "magray.exe"
```

### Вариант 2: Сборка из исходников

```bash
git clone https://github.com/yourusername/MAGRAY_Cli
cd MAGRAY_Cli
cargo build --release
# Бинарник будет в ./target/release/magray
```

---

## ⚙️ Настройка (2 минуты)

### Шаг 1: Создайте .env файл

```bash
# Скопируйте пример
cp .env.example .env

# Или создайте вручную
cat > .env << EOF
# Обязательно (выберите один)
LLM_PROVIDER=openai
OPENAI_API_KEY=your-api-key-here

# Опционально для multi-provider
# ANTHROPIC_API_KEY=your-anthropic-key
# GROQ_API_KEY=your-groq-key
EOF
```

### Шаг 2: Загрузите модели (автоматически)

```bash
# Модели загрузятся автоматически при первом запуске
# Или загрузите заранее:
magray models download qwen3-embeddings
```

---

## 🎯 Первый запуск (2 минуты)

### 1. Проверка системы

```bash
magray health
```

Вы должны увидеть:
```
✓ LLM Service: Connected
✓ Memory Service: Healthy
✓ Models: Loaded
✓ Binary: v0.2.0 (16.2 MB)
```

### 2. Первый чат

```bash
magray chat "Привет! Расскажи что ты умеешь?"
```

### 3. Работа с файлами

```bash
# Создать файл
magray tool "создай файл hello.rs с простым hello world"

# Прочитать файл
magray tool "покажи содержимое hello.rs"

# Анализ кода
magray smart "проанализируй hello.rs и предложи улучшения"
```

---

## 💡 Основные команды

### Чат режим
```bash
# Интерактивный чат (выход: exit)
magray chat

# Одиночный вопрос
magray chat "Как оптимизировать Rust код?"
```

### Умные задачи
```bash
# Анализ проекта
magray smart "проанализируй архитектуру проекта"

# Рефакторинг
magray smart "улучши код в src/main.rs"

# Генерация
magray smart "создай REST API для управления задачами"
```

### Инструменты
```bash
# Git операции
magray tool "покажи git статус"
magray tool "создай коммит с описанием изменений"

# Файловые операции
magray tool "найди все .rs файлы"
magray tool "создай структуру папок для нового проекта"

# Shell команды
magray tool "запусти cargo test"
```

### Память
```bash
# Сохранить важную информацию
magray memory add "API endpoint: POST /api/users" --layer insights

# Поиск в памяти
magray memory search "API endpoints"
```

---

## 🔥 Продвинутые примеры

### Пайплайн обработки
```bash
# Анализ → Документация → Тесты
magray smart "проанализируй main.rs" | \
magray tool "создай документацию на основе анализа" | \
magray tool "сгенерируй тесты для документированных функций"
```

### Интеграция в скрипты
```bash
#!/bin/bash
# deploy.sh

# Проверка перед деплоем
if magray smart "проверь готовность к production"; then
    echo "✓ Проверки пройдены"
    cargo build --release
    ./deploy-to-server.sh
else
    echo "✗ Найдены проблемы"
    exit 1
fi
```

### Использование в CI/CD
```yaml
# .github/workflows/ai-review.yml
- name: AI Code Review
  run: |
    magray smart "проверь PR на best practices" > review.md
    cat review.md >> $GITHUB_STEP_SUMMARY
```

---

## 🆘 Частые проблемы

### "LLM Service not configured"
```bash
# Проверьте .env файл
cat .env | grep API_KEY

# Проверьте переменные окружения
echo $OPENAI_API_KEY
```

### "Model not found"
```bash
# Загрузите модели
magray models download qwen3-embeddings

# Проверьте загруженные модели
magray models list
```

### "Memory service error"
```bash
# Очистите кэш
rm -rf ~/.magray/cache

# Переинициализируйте БД
magray memory init --force
```

---

## 📚 Что дальше?

1. **Изучите команды**: `magray --help`
2. **Настройте под себя**: Отредактируйте `~/.magray/config.toml`
3. **Прочитайте документацию**:
   - [API Reference](API.md) - Все команды и параметры
   - [Архитектура](ARCHITECTURE.md) - Как всё работает
   - [Примеры](../examples/) - Готовые скрипты

---

## 🎓 Полезные советы

1. **Используйте Tab-completion**:
   ```bash
   # Bash/Zsh
   eval "$(magray completions bash)"
   ```

2. **Алиасы для частых команд**:
   ```bash
   alias mc="magray chat"
   alias mt="magray tool"
   alias ms="magray smart"
   ```

3. **История команд**:
   ```bash
   # Все команды сохраняются в
   ~/.magray/history.log
   ```

4. **Debug режим**:
   ```bash
   RUST_LOG=debug magray status
   ```

---

**Готовы к работе?** Запустите `magray chat` и начните! 🚀

**Нужна помощь?** 
- Discord: [discord.gg/magray](https://discord.gg/magray)
- GitHub Issues: [Сообщить о проблеме](https://github.com/yourusername/MAGRAY_Cli/issues)

---

*Создано с ❤️ на Rust* | [⭐ Star на GitHub](https://github.com/yourusername/MAGRAY_Cli)