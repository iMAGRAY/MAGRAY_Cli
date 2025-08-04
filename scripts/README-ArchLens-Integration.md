# ArchLens Integration для Claude Code

## 🔍 Обзор

Автоматическая интеграция ArchLens с Claude Code для анализа архитектуры и качества кода при редактировании файлов.

## 📁 Файлы

### `archilens-auto-analysis.ps1`
Основной скрипт-hook для автоматического анализа при редактировании файлов.

**Функции:**
- Определение измененного crate по пути файла
- Анализ размера файлов и предупреждения о больших файлах
- Цветное логирование с временными метками
- Рекомендации по дальнейшим действиям

### `archilens-integration.ps1`
Расширенный скрипт для глубокой интеграции с ArchLens.

**Функции:**
- Базовый анализ проекта (`analyze`)
- Поиск критических проблем (`critical`)
- Генерация отчетов (`report`)
- Метрики размера и сложности проекта

## ⚙️ Настройка Hooks

В файле `.claude/settings.local.json` добавлены hooks:

```json
{
  "hooks": {
    "post-edit": "script for single file analysis",
    "post-multi-edit": "script for full analysis",
    "pre-commit": "pre-commit analysis notification"
  }
}
```

## 🚀 Использование

### Автоматическое срабатывание
Hooks срабатывают автоматически при:
- Редактировании любого `.rs` файла (`post-edit`)
- Множественном редактировании (`post-multi-edit`)
- Перед коммитом (`pre-commit`)

### Ручной запуск

#### Анализ конкретного файла:
```powershell
.\scripts\archilens-auto-analysis.ps1 -EditedFile "crates\memory\src\service.rs"
```

#### Полный анализ:
```powershell
.\scripts\archilens-auto-analysis.ps1 -FullAnalysis
```

#### Расширенный анализ:
```powershell
.\scripts\archilens-integration.ps1 -Action analyze
.\scripts\archilens-integration.ps1 -Action critical
.\scripts\archilens-integration.ps1 -Action report
```

## 🎯 Особенности для MAGRAY CLI

### Критические файлы
Hook особо отслеживает изменения в:
- `crates\memory\src\*` - система памяти
- `crates\ai\src\*` - AI/embedding сервисы
- `crates\llm\src\*` - LLM интеграция
- `crates\cli\src\*` - CLI интерфейс

### Метрики качества
- **Размер файла**: Предупреждение при >100KB
- **Сложность проекта**: Оценка по общему размеру
- **Архитектурные проблемы**: Поиск антипаттернов

### Рекомендации
Hook предоставляет конкретные рекомендации:
- Рефакторинг больших файлов
- Полный архитектурный анализ для критических изменений
- Команды для детального анализа через ArchLens

## 📊 Пример вывода

```
[2025-08-04 21:19:23] Info: Starting ArchLens automatic analysis...
[2025-08-04 21:19:23] Info: Analyzing changes in file: crates\memory\src\service.rs
[2025-08-04 21:19:23] Info: Detected crate: memory
[2025-08-04 21:19:23] Info: Getting project structure...
[2025-08-04 21:19:23] Info: File size: 54.93 KB
[2025-08-04 21:19:23] Success: Automatic analysis completed
[2025-08-04 21:19:23] Info: For detailed analysis run: archlens export_ai_compact
```

## 🔧 Настройка окружения

### Требования
- PowerShell 5.1+
- ArchLens MCP Server настроенный в Claude Code
- Проект MAGRAY CLI в стандартной структуре

### Разрешения PowerShell
Если возникают проблемы с выполнением, установите:
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
```

## 🎨 Цветовая схема логов

- **Cyan** - Информационные сообщения
- **Green** - Успешное завершение
- **Yellow** - Предупреждения
- **Red** - Ошибки

## 📈 Интеграция с ArchLens MCP

Hook готов к интеграции с ArchLens через MCP API:
- `mcp__archlens__analyze_project` - базовый анализ
- `mcp__archlens__export_ai_compact` - полный отчет
- `mcp__archlens__get_project_structure` - структура проекта

## 🔄 Следующие шаги

1. **Расширение анализа**: Добавить реальные вызовы ArchLens MCP
2. **Кэширование**: Оптимизация для больших проектов
3. **Настройка**: Конфигурационный файл для пороговых значений
4. **Интеграция CI/CD**: Hooks для GitHub Actions

## 📝 Логирование

Все действия hooks логируются с временными метками для отслеживания производительности и отладки.

Hooks автоматически определяют контекст изменений и адаптируют анализ соответственно.