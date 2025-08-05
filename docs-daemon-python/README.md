# CTL v3.0 Tensor Sync Daemon (Python)

Быстрая и гибкая реализация синхронизации CTL аннотаций с максимальной адаптивностью к изменениям в языке CTL.

## 🎯 Особенности

- **Максимальная адаптивность** - простая настройка через `settings.json`
- **Поддержка CTL v2.0** - JSON формат `// @component: {...}`
- **Поддержка CTL v3.0** - Tensor формат `// @ctl3: Ⱦ[id:type] := {...}`
- **Unicode + ASCII** - тензорные символы и ASCII альтернативы
- **Модульная архитектура** - легко расширять новыми парсерами
- **Автоматическая валидация** - jsonschema + pydantic проверки
- **Файловый мониторинг** - отслеживание изменений в реальном времени

## 🚀 Быстрый старт

### Установка
```bash
cd docs-daemon-python
pip install -e .
```

### Использование
```bash
# Одноразовая синхронизация
ctl-sync once

# Режим наблюдения за файлами  
ctl-sync watch

# Показать статистику
ctl-sync stats
```

## 🔄 Постоянный запуск демона

### 1. Автоматический startup (Windows)
```bash
# Запуск через удобный скрипт
start_daemon.bat

# Или напрямую
python -m ctl_sync.main watch
```

### 2. Управление через daemon_manager
```bash
# Полная синхронизация настроек + запуск
python daemon_manager.py sync

# Только запуск
python daemon_manager.py start

# Остановка
python daemon_manager.py stop

# Перезапуск
python daemon_manager.py restart

# Проверка статуса
python daemon_manager.py validate
```

### 3. Автоматическая интеграция с claude-md-orchestrator
Агент `claude-md-orchestrator` автоматически:
- Мониторит изменения CTL правил в CLAUDE.md
- Обновляет settings.json с новыми паттернами
- Перезапускает демон при изменениях
- Валидирует корректность работы

Никаких ручных действий не требуется!

## ⚙️ Настройка через settings.json

**Главная особенность** - вся конфигурация в одном JSON файле:

```json
{
  "project": {
    "name": "MAGRAY_CLI",
    "crates_dir": "crates",
    "claude_md_file": "CLAUDE.md"
  },
  "parsing": {
    "ctl2": {
      "enabled": true,
      "patterns": ["//\\s*@component:\\s*(\\{.*\\})"]
    },
    "ctl3": {
      "enabled": true,
      "patterns": ["//\\s*@ctl3:\\s*([ⱧD]\\[.*?\\]\\s*:=\\s*\\{[^}]*\\})"],
      "tensor_symbols": {
        "unicode": ["⊗", "⊕", "∇"],
        "ascii": ["compose", "parallel", "grad"]
      }
    }
  },
  "validation": {
    "component": {
      "valid_kinds": ["T", "A", "B", "F", "M", "S", "R", "P", "D", "C", "E"],
      "required_fields": ["k", "id", "t"]
    }
  },
  "features": {
    "colored_output": true,
    "statistics": true
  }
}
```

### Интерактивный редактор настроек

```bash
# Показать все настройки
python config_editor.py show

# Получить значение
python config_editor.py get project.name

# Установить значение
python config_editor.py set project.name "My Project"

# Интерактивная настройка
python config_editor.py interactive

# Поиск настроек
python config_editor.py search "gpu"

# Создать файл по умолчанию
python config_editor.py create-default
```

## 📊 Поддерживаемые форматы

### CTL v2.0 (JSON)
```rust
// @component: {"k":"C","id":"vector_store","t":"Vector storage with HNSW","m":{"cur":65,"tgt":95,"u":"%"}}
```

### CTL v3.0 (Tensor)
```rust
// @ctl3: Ⱦ[vector_optimizer:service] := {∇[85→100] ⊗[gpu_engine,cpu_fallback] ⊕async}
```

### ASCII альтернативы
```rust
// @ctl3: D[ml_coordinator:orchestrator] := {grad[70->95] compose[tensor_optimizer,batch_processor]}
```

## 🔧 Адаптация к изменениям CTL

### Добавление нового формата CTL v4.0

1. **Обновить settings.json**:
```json
{
  "parsing": {
    "ctl4": {
      "enabled": true,
      "patterns": ["//\\s*@ctl4:\\s*<новый_паттерн>"],
      "new_operators": ["⚡", "🔥", "⭐"]
    }
  }
}
```

2. **Создать parser**:
```python
# ctl_sync/parsers/ctl4_parser.py
class Ctl4Parser(BaseParser):
    def setup_patterns(self):
        patterns = self.config.get("parsing.ctl4.patterns", [])
        # Реализация нового парсера
```

3. **Подключить в core.py**:
```python
if self.config.is_ctl4_enabled():
    self.ctl4_parser = Ctl4Parser()
```

### Изменение валидации

Просто отредактируйте `settings.json`:
```json
{
  "validation": {
    "component": {
      "valid_kinds": ["T", "A", "B", "F", "M", "S", "R", "P", "D", "C", "E", "X"],
      "max_id_length": 64,
      "new_required_fields": ["k", "id", "t", "version"]
    }
  }
}
```

### Новые тензорные операторы

```json
{
  "parsing": {
    "ctl3": {
      "tensor_symbols": {
        "unicode": ["⊗", "⊕", "∇", "⚡", "🔥"],
        "ascii": ["compose", "parallel", "grad", "lightning", "fire"]
      }
    }
  }
}
```

## 📁 Структура проекта

```
docs-daemon-python/
├── settings.json           # Главный файл настроек
├── config_editor.py        # Интерактивный редактор настроек
├── ctl_sync/
│   ├── core.py            # Основная логика синхронизации
│   ├── json_config.py     # Система конфигурации JSON
│   ├── schema.py          # Валидация CTL компонентов
│   ├── parsers/           # Модульные парсеры
│   │   ├── base_parser.py
│   │   ├── ctl2_parser.py
│   │   └── ctl3_parser.py
│   ├── utils.py           # Утилиты и форматирование
│   ├── watchers.py        # Файловый мониторинг
│   └── main.py           # CLI интерфейс
└── tests/                 # Тесты
```

## 🎨 Примеры использования

### Базовое использование
```bash
# Автоматическая настройка
ctl-sync once

# С детальным выводом
ctl-sync once --verbose

# Другой проект
ctl-sync once --project-root /path/to/project
```

### Изменение настроек
```bash
# Отключить цветной вывод
python config_editor.py set features.colored_output false

# Добавить новое расширение файлов
python config_editor.py set scanning.file_extensions '[".rs", ".toml", ".md"]'

# Изменить маркер секции в CLAUDE.md
python config_editor.py set output.claude_md.section_marker "# COMPONENTS"
```

### Проверка результата
```bash
# Статистика
ctl-sync stats

# Проверка настроек
python config_editor.py validate

# Поиск компонентов
python config_editor.py search "gpu"
```

## 🚀 Преимущества над Rust версией

1. **Мгновенная адаптация** - изменения через `settings.json` без перекомпиляции
2. **Простота расширения** - новые парсеры на Python за минуты
3. **Интерактивная настройка** - GUI-подобный интерфейс через `config_editor.py`
4. **Быстрая отладка** - нет этапа компиляции
5. **Гибкая валидация** - легко изменяемые схемы

## 🎯 Идеально для

- **Быстрое прототипирование** новых CTL форматов
- **Экспериментирование** с тензорными операторами
- **Адаптация** к изменяющимся требованиям
- **Обучение** и понимание CTL языка
- **Кастомизация** под специфические проекты

Этот Python daemon создан специально для максимальной скорости адаптации к эволюции языка CTL!