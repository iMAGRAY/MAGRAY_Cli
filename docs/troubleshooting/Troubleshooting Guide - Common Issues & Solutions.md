# Troubleshooting Guide - Common Issues & Solutions

#troubleshooting #guide #production #magray-cli

> **Практическое руководство по диагностике и решению проблем MAGRAY CLI**
> 
> Базируется на анализе health monitoring систем, error handling patterns и production опыте

---

## 📋 Quick Diagnostic Commands

### 🚀 Fast System Check
```bash
# Быстрая проверка статуса всех компонентов
magray status

# Детальная health check с диагностикой
magray health --verbose

# Показать конфигурацию и доступные возможности
magray info --all
```

### 🔍 Deep System Analysis
```bash
# Проверка GPU и AI модели
magray gpu info
magray models list --check-integrity

# Анализ memory system
magray memory status --layers --stats
magray memory health --components
```

---

## 🏥 System Health Issues

### ❌ Health Check Failures

**Symptoms:**
- `magray status` показывает UNHEALTHY компоненты
- High latency в health check результатах
- Error messages в structured logs

**Diagnostic Commands:**
```bash
# Подробная диагностика каждого компонента
magray health --component=all --verbose

# Проверка системных ресурсов
magray health --system-resources

# Анализ логов за последний час
magray logs --level=error --since=1h
```

**Common Solutions:**

1. **Memory Service Degraded**
```bash
# Перезапуск memory service
magray memory restart

# Проверка HNSW индексов
magray memory validate-indexes

# Пересоздание поврежденных индексов
magray memory rebuild-index --layer=all
```

2. **LLM Service Unhealthy**
```bash
# Проверка API ключей
magray config check-keys

# Переключение на backup provider
magray llm switch-provider --backup

# Тест соединения
magray llm test-connection --all-providers
```

3. **Disk Space Critical**
```bash
# Освобождение места через очистку кэша
magray cache clean --old --size-limit=5GB

# Архивирование старых данных
magray backup create --incremental
magray memory archive --older-than=30d
```

### 🔄 Circuit Breaker Issues

**Symptoms:**
- GPU постоянно в CPU fallback режиме
- Warning: "Circuit breaker открыт"
- Degraded performance

**Solutions:**
```bash
# Сброс circuit breaker
magray gpu reset-circuit-breaker

# Диагностика GPU проблем
magray gpu diagnose --full

# Принудительный CPU режим если GPU нестабилен
magray config set gpu.force_cpu_mode=true
```

---

## 🖥️ GPU/AI Model Problems

### ⚡ GPU Detection Issues

**Error Patterns:**
- `GPU not available`
- `CUDA error`
- `GPU timeout after 30s`

**Diagnostic Steps:**
```bash
# Полная GPU диагностика
magray gpu info --detailed
nvidia-smi  # если доступен

# Проверка CUDA совместимости
magray gpu check-compatibility

# Тест GPU memory pool
magray gpu test-memory-pool
```

**Solutions:**

1. **GPU Not Detected**
```bash
# Переустановка CUDA драйверов (Windows)
# Скачать с nvidia.com/drivers

# Проверка PATH для CUDA
echo $PATH | grep -i cuda

# Fallback на CPU
magray config set ai.use_gpu=false
```

2. **GPU Out of Memory**
```bash
# Уменьшение batch size
magray config set ai.batch_size=16

# Очистка GPU memory pool
magray gpu clear-memory-pool

# Настройка memory limit
magray config set gpu.memory_limit_mb=4096
```

### 🤖 Model Loading Failures

**Error Patterns:**
- `Model not loaded`
- `Invalid dimensions`
- `Tokenization failed`
- `ONNX model load error`

**Diagnostic Commands:**
```bash
# Проверка целостности моделей
magray models validate --all

# Переустановка поврежденных моделей
magray models download --force --model=qwen3

# Тест модели с sample input
magray models test --model=qwen3 --input="test text"
```

**Recovery Steps:**
```bash
# Очистка model cache
rm -rf ~/.cache/magray/models/

# Повторная загрузка базовых моделей
magray models install --essential

# Fallback на CPU-only модели
magray config set ai.gpu_models=false
```

---

## 🧠 Memory System Failures

### 🗃️ Database Corruption

**Error Codes:** `DB_ERROR`, `CORRUPTED`

**Emergency Recovery:**
```bash
# Backup перед восстановлением
magray backup create --emergency

# Проверка целостности database
magray memory check-integrity

# Восстановление из последнего backup
magray backup restore --latest --verify

# Пересоздание индексов
magray memory rebuild-indexes --all-layers
```

### 🔍 HNSW Index Issues

**Symptoms:**
- Slow search performance (<5ms target not met)
- `Index corrupted` errors
- Search returns no results

**Troubleshooting:**
```bash
# Анализ производительности поиска
magray memory benchmark --search

# Проверка HNSW состояния
magray memory index-status --verbose

# Rebuilding specific layer indexes
magray memory rebuild-index --layer=insights --force
```

### 🧮 DI Container Problems

**Error Patterns:**
- `DI resolution failed`
- High DI overhead (>10ms)
- Memory leaks в DI container

**Solutions:**
```bash
# Переключение на optimized DI container
magray config set memory.use_optimized_di=true

# Диагностика DI performance
magray memory di-stats --detailed

# Принудительная очистка DI cache
magray memory di-clear-cache
```

### 📊 Memory Promotion Issues

**Symptoms:**
- Data не попадает в Insights layer
- ML promotion engine errors
- Неправильная prioritization

**Debug Commands:**
```bash
# Статистика promotion engine
magray memory promotion-stats

# ML promotion диагностика
magray memory ml-promotion-health

# Принудительный promotion для тестирования
magray memory force-promotion --record-id=<uuid>
```

---

## 🌐 LLM Provider Issues

### 🔑 API Authentication

**Error Codes:** `AUTH_ERROR`, `PERMISSION_DENIED`

**Solutions:**
```bash
# Проверка и обновление API ключей
magray config check-keys --validate
magray config set openai.api_key="new-key"

# Переключение на backup provider
magray llm switch-provider --to=anthropic

# Test различных providers
magray llm test-all-providers
```

### 🚧 Rate Limiting

**Error Patterns:**
- HTTP 429 errors
- `Rate limit exceeded`
- Request timeouts

**Mitigation:**
```bash
# Настройка backoff strategy
magray config set llm.retry_policy.exponential_backoff=true

# Уменьшение concurrent requests
magray config set llm.max_concurrent_requests=3

# Локальный fallback
magray llm configure-local --model=ollama/llama2
```

### ⏱️ Timeout Issues

**Configuration:**
```bash
# Увеличение timeouts для медленных providers
magray config set llm.request_timeout_sec=60

# Настройка streaming для длинных ответов
magray config set llm.use_streaming=true

# Circuit breaker для нестабильных providers
magray config set llm.circuit_breaker.enabled=true
```

---

## ⚡ Performance Problems

### 🐌 Slow Search Performance

**Target:** <5ms HNSW search, >1000 QPS

**Profiling Commands:**
```bash
# Detailed performance профиль
magray memory benchmark --comprehensive

# GPU vs CPU comparison
magray memory benchmark --compare-modes

# Profiling specific operations
magray memory profile --operation=search --duration=30s
```

**Optimization Steps:**

1. **HNSW Tuning:**
```bash
# Оптимизация HNSW параметров
magray config set hnsw.ef_construction=200
magray config set hnsw.m_l=16

# Rebuilding с оптимальными параметрами
magray memory optimize-indexes
```

2. **GPU Acceleration:**
```bash
# Включение GPU batch processing
magray config set gpu.batch_processing=true
magray config set gpu.batch_size=64

# Memory pool optimization
magray gpu optimize-memory-pool
```

### 💾 Memory Leaks

**Detection:**
```bash
# Memory usage monitoring
magray memory monitor --duration=300s

# Leak detection
magray memory leak-check --verbose

# Resource usage analysis
magray system resources --track-growth
```

**Mitigation:**
```bash
# Принудительная очистка кэшей
magray cache clear --all
magray memory gc --force

# Restart memory service без данных
magray memory restart --preserve-data=false
```

### 📈 High CPU Usage

**Analysis:**
```bash
# CPU profiling
magray profile cpu --duration=60s

# Bottleneck analysis
magray analyze bottlenecks --cpu

# Thread usage monitoring
magray system threads --live-monitor
```

---

## ⚙️ Configuration Errors

### 📄 Invalid Config Files

**Error Patterns:**
- `Configuration error`
- `Invalid format`
- `Missing required field`

**Recovery:**
```bash
# Проверка конфигурации
magray config validate

# Восстановление default config
magray config reset --backup-current

# Guided configuration setup
magray config setup --interactive
```

### 🔧 Environment Issues

**Path Problems:**
```bash
# Проверка всех путей в конфигурации
magray config check-paths

# Создание недостающих директорий
magray config ensure-directories

# Права доступа
magray config check-permissions --fix
```

---

## 🛠️ Development/Build Issues

### 🔨 Compilation Errors

**GPU Feature Issues:**
```bash
# Build только с CPU features
cargo build --no-default-features --features="cpu-only"

# Full GPU build
cargo build --features="gpu,cuda"

# Диагностика build dependencies
cargo tree | grep -E "(onnx|cuda|gpu)"
```

### 🧪 Test Failures

**Running Specific Test Suites:**
```bash
# Memory system tests
cargo test --package memory --test integration_tests

# GPU-specific tests (requires GPU)
cargo test --features gpu test_gpu

# Health check tests
cargo test health_checks
```

### 📦 Binary Size Issues

**Size Analysis:**
```bash
# Анализ размера binary
cargo bloat --release --crates

# Minimal build для production
cargo build --release --features="minimal"

# Strip debug symbols
strip target/release/magray
```

---

## 📊 Log Analysis Guide

### 🔍 Structured Logging

**Log Locations:**
- Windows: `%APPDATA%\magray\logs\`
- Linux/Mac: `~/.local/share/magray/logs/`

**Key Log Queries:**
```bash
# Error analysis за последний день
jq '.level == "ERROR" | select(.timestamp > (now - 86400))' logs/magray.jsonl

# Health check failures
grep -E "(health_check.*failed|UNHEALTHY)" logs/magray.log

# Performance metrics
jq '.operation_duration_ms > 1000' logs/magray.jsonl
```

### 🚨 Alert Patterns

**Critical Alerts:**
- `CRITICAL ALERT: Database corruption detected`
- `FATAL: GPU memory exhausted`
- `Circuit breaker opened after N errors`

**Performance Warnings:**
- `High memory usage: X%`
- `Search latency exceeds threshold`
- `Promotion engine backlog`

---

## 🆘 Emergency Recovery Procedures

### 🔥 Complete System Failure

1. **Immediate Backup:**
```bash
magray backup create --emergency --all-data
```

2. **Safe Mode Start:**
```bash
magray start --safe-mode --cpu-only --minimal-features
```

3. **Data Recovery:**
```bash
magray recovery scan --all-layers
magray recovery restore --interactive
```

### 💣 Database Corruption

1. **Stop все processes:**
```bash
magray stop --force
```

2. **Backup corrupt data:**
```bash
cp -r ~/.cache/magray/memory.db ~/.cache/magray/memory.db.corrupt.backup
```

3. **Recovery:**
```bash
magray recovery database --repair-attempt
# If fails:
magray recovery database --rebuild-from-backups
```

---

## 🔧 Preventive Maintenance

### 📅 Daily Tasks
```bash
# Automated health check
magray health --automated --log-results

# Cache cleanup
magray cache clean --auto-size

# Metrics collection
magray metrics collect --store
```

### 📆 Weekly Tasks
```bash
# Full system backup
magray backup create --full --verify

# Performance benchmark
magray benchmark --comprehensive --baseline

# Index optimization
magray memory optimize --all-indexes
```

### 🗓️ Monthly Tasks
```bash
# Archive old data
magray memory archive --older-than=30d

# Model updates check
magray models check-updates --auto-download

# Configuration audit
magray config audit --security-check
```

---

## 📞 Getting Help

### 🐛 Bug Reports
Включить в отчет:
```bash
# System information
magray info --system --verbose

# Recent logs
magray logs --export --last=1h

# Configuration (sanitized)
magray config export --anonymize
```

### 🔗 Useful Links
- [[GPU Configuration Guide]]
- [[Memory System Architecture]]
- [[Performance Tuning]]
- [[Production Deployment]]

---

*Последнее обновление: {{date:YYYY-MM-DD}} | Версия: Production Ready*

**Tags:** #troubleshooting #production #gpu #memory-system #llm #health-monitoring #performance #recovery