# Руководство по сборке и развертыванию MAGRAY CLI

## 🏗️ Варианты сборки

MAGRAY CLI поддерживает три основных варианта сборки с conditional compilation для оптимизации под разные сценарии использования.

### 📊 Сравнение feature sets

| Feature | CPU | GPU | Minimal | Описание |
|---------|-----|-----|---------|----------|
| **Binary размер** | 16MB | 16MB | 16MB | Оптимизированный размер |
| **Startup время** | ~150ms | ~300ms | ~100ms | Холодный запуск |
| **GPU ускорение** | ❌ | ✅ | ❌ | CUDA acceleration |
| **Зависимости** | ONNX CPU | ONNX GPU | ONNX CPU | Runtime dependencies |
| **Использование** | Production | Workstation | Containers | Рекомендуемый сценарий |

## 🔨 Команды сборки

### С использованием Makefile (рекомендуется)

```bash
# Показать все доступные команды
make help

# Базовые сборки
make build-cpu       # CPU-only режим
make build-gpu       # GPU ускорение  
make build-minimal   # Минимальная сборка
make build-all       # Все варианты

# Development сборки
make dev-cpu         # Debug сборка CPU
make dev-gpu         # Debug сборка GPU

# Верификация
make verify-features # Проверка feature compatibility
make size-analysis   # Анализ размеров бинарников
make perf-test       # Быстрый performance тест
```

### Прямые Cargo команды

```bash
# CPU-only (production)
cargo build --release --features=cpu

# GPU-enabled (workstation) 
cargo build --release --features=gpu

# Minimal (containers)
cargo build --release --features=minimal

# Development builds
cargo build --features=cpu
cargo build --features=gpu
```

## 🧪 Тестирование

### Comprehensive testing

```bash
# Все тесты для всех feature sets
make test-all

# Специфичные feature тесты
make test-cpu        # CPU features
make test-gpu        # GPU features (требует CUDA)
make test-minimal    # Minimal features

# Бенчмарки производительности
make bench           # CPU benchmarks
make bench-gpu       # GPU benchmarks
```

### Unit и Integration тесты

```bash
# Workspace тесты
cargo test --workspace

# Feature-specific тесты
cargo test --features=cpu --workspace
cargo test --features=gpu --workspace
cargo test --features=minimal --workspace

# Конкретные модули
cargo test -p memory --features=cpu
cargo test -p ai --features=gpu
```

## 🐳 Docker развертывание

### Предварительно настроенные образы

Проект включает три оптимизированных Dockerfile для разных сценариев:

#### CPU Production образ
```bash
# Сборка
docker build -f scripts/docker/Dockerfile.cpu -t magray:cpu .

# Запуск
docker run -it \
  -v ~/.magray:/root/.magray \
  -v $(pwd):/workspace \
  magray:cpu

# Характеристики
# - Base: Debian Bookworm Slim
# - Size: ~100MB
# - Runtime: CPU ONNX Runtime
# - Target: Production servers
```

#### GPU Workstation образ
```bash
# Сборка (требует NVIDIA Docker)
docker build -f scripts/docker/Dockerfile.gpu -t magray:gpu .

# Запуск
docker run -it --gpus all \
  -v ~/.magray:/root/.magray \
  -v $(pwd):/workspace \
  magray:gpu

# Характеристики  
# - Base: NVIDIA CUDA 12.3
# - Size: ~3GB
# - Runtime: GPU ONNX Runtime
# - Target: GPU workstations
```

#### Minimal Container образ
```bash
# Сборка
docker build -f scripts/docker/Dockerfile.minimal -t magray:minimal .

# Запуск
docker run -it magray:minimal

# Характеристики
# - Base: Scratch (статическая линковка)
# - Size: ~20MB
# - Runtime: Minimal ONNX
# - Target: Edge devices, K8s
```

### Docker Compose оркестрация

```bash
cd scripts/docker

# CPU режим
docker-compose --profile cpu up

# GPU режим (требует nvidia-docker)
docker-compose --profile gpu up

# Minimal режим
docker-compose --profile minimal up

# Benchmark testing
docker-compose --profile benchmark up
```

### Multi-stage build оптимизация

Все Dockerfile используют multi-stage builds для минимизации размера:

1. **Builder stage**: Компиляция с полным Rust toolchain
2. **Runtime stage**: Только необходимые runtime зависимости
3. **Оптимизация**: Strip символов, минимальные base образы

## ⚙️ Настройка окружения

### Системные требования

**Базовые требования:**
- **Rust 1.75+** (установить через [rustup](https://rustup.rs/))
- **4GB RAM** минимум (8GB рекомендуется) 
- **2GB дискового пространства** для ONNX моделей

**GPU требования (optional):**
- **NVIDIA GPU** с CUDA Compute Capability 6.0+
- **CUDA Toolkit 11.8+** или **12.x**
- **cuDNN 8.x** (автоматически устанавливается)
- **NVIDIA Container Runtime** для Docker

### ONNX Runtime настройка

ONNX Runtime автоматически настраивается через скрипты:

```bash
# Windows
./scripts/install_onnxruntime.ps1        # CPU версия
./scripts/install_onnxruntime_gpu.ps1    # GPU версия

# Linux/macOS  
./scripts/install_onnxruntime.sh

# Верификация установки
magray status
```

### Переменные окружения

```bash
# ONNX Runtime configuration
export ORT_DYLIB_PATH="/path/to/onnxruntime/lib/libonnxruntime.so"

# Логирование
export RUST_LOG=info
export LOG_FORMAT=json
export LOG_FILE=magray.log

# GPU configuration
export CUDA_VISIBLE_DEVICES=0
export MAGRAY_FORCE_CPU=1  # Принудительный CPU режим

# Model paths
export MAGRAY_MODEL_DIR="/custom/model/path"
```

## 🚀 Production развертывание

### Системные сервисы

#### SystemD service (Linux)
```ini
# /etc/systemd/system/magray.service
[Unit]
Description=MAGRAY CLI Agent
After=network.target

[Service]
Type=simple
User=magray
WorkingDirectory=/opt/magray
ExecStart=/opt/magray/bin/magray
Restart=always
RestartSec=10
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable magray
sudo systemctl start magray
sudo systemctl status magray
```

#### Windows Service
```powershell
# Создание Windows службы
sc create "MAGRAY CLI" binPath="C:\magray\magray.exe" start=auto

# Запуск службы
sc start "MAGRAY CLI"

# Статус службы
sc query "MAGRAY CLI"
```

### Kubernetes развертывание

```yaml
# magray-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: magray-cli
spec:
  replicas: 3
  selector:
    matchLabels:
      app: magray
  template:
    metadata:
      labels:
        app: magray
    spec:
      containers:
      - name: magray
        image: magray:cpu
        ports:
        - containerPort: 8080
        env:
        - name: RUST_LOG
          value: "info"
        resources:
          requests:
            memory: "512Mi"
            cpu: "250m"
          limits:
            memory: "2Gi"
            cpu: "1000m"
        volumeMounts:
        - name: magray-data
          mountPath: /root/.magray
      volumes:
      - name: magray-data
        persistentVolumeClaim:
          claimName: magray-pvc
```

```bash
kubectl apply -f magray-deployment.yaml
kubectl get pods -l app=magray
kubectl logs -f deployment/magray-cli
```

### Monitoring и observability

#### Prometheus метрики
```yaml
# prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'magray'
    static_configs:
      - targets: ['localhost:9090']
    metrics_path: /metrics
    scrape_interval: 10s
```

#### Grafana dashboard
```json
{
  "dashboard": {
    "title": "MAGRAY CLI Metrics",
    "panels": [
      {
        "title": "Memory Usage",
        "type": "graph",
        "targets": [
          {
            "expr": "magray_memory_usage_bytes",
            "legendFormat": "Memory Usage"
          }
        ]
      },
      {
        "title": "Vector Search Latency", 
        "type": "graph",
        "targets": [
          {
            "expr": "magray_vector_search_duration_seconds",
            "legendFormat": "Search Latency"
          }
        ]
      }
    ]
  }
}
```

### Load balancing

#### NGINX configuration
```nginx
upstream magray_backend {
    server 127.0.0.1:8080;
    server 127.0.0.1:8081;
    server 127.0.0.1:8082;
}

server {
    listen 80;
    server_name magray.example.com;
    
    location / {
        proxy_pass http://magray_backend;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }
}
```

## 📊 Performance tuning

### CPU оптимизация

```bash
# Compile with native optimizations
RUSTFLAGS="-C target-cpu=native" cargo build --release --features=cpu

# Profile-guided optimization (PGO)
RUSTFLAGS="-C profile-generate=/tmp/pgo-data" cargo build --release --features=cpu
# ... run representative workload ...
RUSTFLAGS="-C profile-use=/tmp/pgo-data" cargo build --release --features=cpu
```

### Memory настройка

```toml
# ~/.magray/config.toml
[memory]
max_vectors_per_layer = 1000000  # Увеличить для больших datasets
cache_size_mb = 2048             # Увеличить для лучшего кэширования

[memory.hnsw]
max_connections = 32             # Увеличить для лучшего recall
ef_construction = 800            # Увеличить для лучшего качества
ef_search = 200                  # Увеличить для лучшего recall
```

### GPU оптимизация

```toml
[ai]
embed_batch_size = 64    # Увеличить для GPU
use_gpu = true
gpu_memory_fraction = 0.8

[ai.gpu]
enable_tensorrt = true
fp16_mode = true
max_workspace_size_mb = 1024
```

## 🔍 Troubleshooting

### Диагностика проблем сборки

```bash
# Проверка зависимостей
rustc --version
cargo --version

# Проверка feature flags
make verify-features

# Детальная диагностика
RUST_LOG=debug cargo build --features=cpu

# Проверка ONNX Runtime
./scripts/install_onnxruntime.ps1
magray status
```

### Runtime диагностика

```bash
# Подробные логи
RUST_LOG=debug magray status

# Memory debugging
RUST_LOG=debug magray memory stats

# Performance profiling
cargo flamegraph --bin magray -- status
```

### Известные проблемы

**Windows ONNX Runtime DLL issues:**
```powershell
# Переустановить ONNX Runtime
./scripts/install_onnxruntime.ps1

# Проверить PATH
echo $env:PATH | Select-String "onnxruntime"

# Manually set DLL path
$env:ORT_DYLIB_PATH = "C:\path\to\onnxruntime.dll"
```

**Linux shared library issues:**
```bash
# Установить зависимости
sudo apt-get install libgomp1 libssl3

# Обновить LD_LIBRARY_PATH
export LD_LIBRARY_PATH="/path/to/onnxruntime/lib:$LD_LIBRARY_PATH"

# Проверить линковку
ldd target/release/magray
```

**macOS security issues:**
```bash
# Разрешить выполнение
xattr -d com.apple.quarantine target/release/magray

# Проверить библиотеки
otool -L target/release/magray
```

## 📚 Дополнительные ресурсы

- [Makefile Reference](../Makefile) - полный список команд сборки
- [Docker Compose](../scripts/docker/docker-compose.yml) - оркестрация контейнеров
- [CI/CD Workflows](../.github/workflows/) - автоматизация сборки
- [Configuration Guide](CONFIGURATION.md) - детальная настройка
- [Monitoring Setup](MONITORING.md) - observability и метрики