# 🚀 MAGRAY CLI - Production CI/CD Pipeline

Comprehensive DevOps infrastructure для MAGRAY CLI с multi-platform builds, comprehensive security scanning, и production monitoring.

## 📋 Обзор Pipeline

### 🔄 GitHub Actions Workflows

| Workflow | Trigger | Назначение | Duration |
|----------|---------|------------|----------|
| **`ci.yml`** | Push/PR к `main`, `develop` | Main CI/CD pipeline с multi-platform builds | ~15-20 мин |
| **`security.yml`** | Daily, Push/PR | Advanced security scanning & SAST analysis | ~10-15 мин |
| **`release.yml`** | Tags `v*` | Automated release process с artifacts | ~25-30 мин |
| **`monitoring.yml`** | Daily, Weekly | Repository health & performance monitoring | ~8-12 мин |

---

## 🏗️ Multi-Platform Build Matrix

### Поддерживаемые Платформы

| Platform | Target | Features | Binary Size | Use Case |
|----------|--------|----------|-------------|----------|
| **Windows x64** | `x86_64-pc-windows-msvc` | `cpu` | ~16MB | Desktop workstations |
| **Linux x64** | `x86_64-unknown-linux-gnu` | `cpu,gpu` | ~18MB | Servers, GPU workstations |
| **Linux ARM64** | `aarch64-unknown-linux-gnu` | `cpu` | ~15MB | ARM servers, Raspberry Pi |
| **macOS Intel** | `x86_64-apple-darwin` | `cpu` | ~17MB | Intel Macs |
| **macOS ARM64** | `aarch64-apple-darwin` | `cpu` | ~15MB | Apple Silicon Macs |

### Docker Variants

| Variant | Base Image | Size | Use Case | Features |
|---------|------------|------|----------|----------|
| **CPU** | `debian:bookworm-slim` | ~50MB | Production servers | CPU-only ONNX, optimized |
| **GPU** | `nvidia/cuda:12.3-runtime` | ~800MB | GPU workstations | CUDA, TensorRT, GPU acceleration |
| **Minimal** | `scratch` | <20MB | Edge, containers | Static binary, UPX compressed |

---

## 🔒 Security Scanning Framework

### Multi-Layer Security Approach

#### 1. **Dependency Vulnerability Scanning**
```bash
# Cargo audit для critical vulnerabilities
cargo audit --json > audit-results.json

# Supply chain risk assessment  
cargo tree --format "{p} {f}" | analysis
```

#### 2. **Static Application Security Testing (SAST)**
```bash  
# Enhanced Clippy с security rules
cargo clippy --all-targets -- \
  -D warnings \
  -D clippy::suspicious \
  -W clippy::unwrap_used

# Unsafe code analysis
cargo geiger --format json
```

#### 3. **Secret & Credential Detection**
```bash
# TruffleHog scanning
trufflehog --path=./ --json --only-verified

# Custom pattern detection
grep -r "api[_-]?key\|auth[_-]?token" crates/
```

#### 4. **Code Quality Analysis**
- **CodeQL Analysis**: GitHub's semantic code analysis
- **License Compliance**: Проверка лицензий dependencies
- **Pattern Security**: SQL injection, command injection detection

### Security Quality Gates

| Check | Threshold | Action |
|-------|-----------|--------|
| **Critical Vulnerabilities** | 0 | ❌ Block merge |
| **Major Vulnerabilities** | <3 | ⚠️ Review required |
| **Secrets Detected** | 0 | ❌ Block merge |
| **License Issues** | 0 restrictive | ❌ Block merge |

---

## 📊 Quality Gates & Monitoring

### Code Quality Thresholds

```yaml
Quality Gates:
  Code Coverage: >80%       # Target coverage level
  Performance:
    Build Time: <10min      # Release build threshold  
    Binary Size: <16MB      # Size optimization target
    HNSW Search: <5ms       # Performance SLA
  Security:
    Vulnerabilities: 0      # Zero-tolerance policy
    Secrets: 0              # No hardcoded secrets
    License: MIT/Apache     # Approved licenses only
```

### Performance Regression Detection

```python
# Автоматический benchmark analysis
python scripts/ci/check_benchmark_regression.py \
  --results benchmark-results.json \
  --baseline benchmark-baseline.json \
  --strict
```

**Regression Thresholds:**
- **Minor**: 5% performance drop (warning)
- **Major**: 10% performance drop (fail PR)  
- **Critical**: 25% performance drop (block release)

---

## 🐳 Docker Build Strategy

### Multi-Stage Optimization

```dockerfile
# Stage 1: Dependency caching
COPY Cargo.toml Cargo.lock ./
RUN cargo build --release --features cpu \
    && rm -rf src target/release/deps/magray*

# Stage 2: Source compilation
COPY crates/ ./crates/
RUN cargo build --release --features cpu

# Stage 3: Runtime optimization
FROM debian:bookworm-slim
COPY --from=builder /app/target/release/magray /usr/local/bin/
```

### Build Optimizations

| Technique | CPU Variant | GPU Variant | Minimal Variant |
|-----------|-------------|-------------|-----------------|
| **Multi-stage build** | ✅ | ✅ | ✅ |
| **Dependency pre-build** | ✅ | ✅ | ✅ |
| **LTO optimization** | Fat LTO | Thin LTO | Fat LTO |
| **Binary stripping** | ✅ | ✅ | ✅ + UPX |
| **Layer caching** | ✅ | ✅ | ✅ |

---

## 📈 Release Automation

### Semantic Versioning

```bash
# Version format: X.Y.Z[-prerelease]
v1.0.0          # Stable release
v1.1.0-beta.1   # Pre-release
v2.0.0-rc.1     # Release candidate
```

### Automated Release Process

1. **Version Extraction**: автоматическое определение версии из tag
2. **Changelog Generation**: категоризация коммитов по типам
3. **Multi-Platform Builds**: сборка для всех target platforms  
4. **Artifact Packaging**: создание release archives с checksums
5. **GitHub Release**: автоматическое создание release с description
6. **Docker Publishing**: multi-arch images для всех variants

### Release Artifacts

```
magray-v1.0.0-x86_64-pc-windows-msvc.exe       # Windows executable
magray-v1.0.0-x86_64-unknown-linux-gnu.tar.gz  # Linux x64 archive
magray-v1.0.0-aarch64-unknown-linux-gnu.tar.gz # Linux ARM64 archive  
magray-v1.0.0-x86_64-apple-darwin.tar.gz       # macOS Intel archive
magray-v1.0.0-aarch64-apple-darwin.tar.gz      # macOS ARM64 archive
magray-v1.0.0-checksums.txt                    # SHA256 checksums
```

---

## 📊 Monitoring & Alerting

### Repository Health Metrics

#### Health Score Calculation (0-100)
```python
base_score = 100
# Deductions:
- clippy_warnings > 50:  -15 points
- vulnerabilities > 0:   -10 points each  
- outdated_deps > 10:    -10 points
- no_activity_week:      -5 points

# Bonuses:
+ high_activity > 10:    +5 points
```

#### Performance Monitoring

| Metric | Target | Warning | Critical |
|--------|--------|---------|----------|
| **Build Time** | <8min | >10min | >15min |
| **Binary Size** | <16MB | >20MB | >25MB |
| **Memory Usage** | <100MB | >200MB | >500MB |
| **HNSW Search** | <3ms | >5ms | >10ms |

### Alert System (Mock Implementation)

```yaml
Alerts:
  Critical Issues:
    - PagerDuty notification
    - Slack #critical-alerts  
    - Email to DevOps team
  
  Warning Issues:
    - Slack #dev-notifications
    - GitHub issue creation
    
  Daily Reports:
    - Health dashboard update
    - Metrics collection
    - Trend analysis
```

---

## 🔧 Configuration Files

### `.cargo/config.toml`
Production-optimized Rust build settings:
- **LTO**: Fat LTO для максимальной оптимизации
- **Target-specific**: Оптимизации для каждой платформы  
- **Dependency optimization**: Specific настройки для ONNX Runtime
- **Size optimization**: Минимальный размер binary

### Docker Configurations
- **`Dockerfile.cpu`**: CPU-optimized production build
- **`Dockerfile.gpu`**: CUDA-enabled GPU acceleration  
- **`Dockerfile.minimal`**: Ultra-minimal edge deployment
- **`healthcheck.sh`**: Comprehensive container health monitoring

---

## 🚀 Usage Examples

### Local Development
```bash
# Run tests локально
cargo test --workspace --features cpu

# Build release локально  
cargo build --release --features cpu

# Check performance
cargo bench --features cpu
```

### Docker Development
```bash
# Build CPU variant
docker build -f scripts/docker/Dockerfile.cpu -t magray:cpu .

# Run GPU variant (requires nvidia-docker)
docker run --gpus all magray:gpu

# Minimal variant
docker build -f scripts/docker/Dockerfile.minimal -t magray:minimal .
```

### CI/CD Triggers
```bash
# Trigger main CI pipeline
git push origin main

# Trigger security scan
git push origin feature/security-fix

# Create release
git tag v1.0.0
git push origin v1.0.0

# Manual workflow dispatch
gh workflow run monitoring.yml --field analysis_type=comprehensive
```

---

## 📋 Troubleshooting

### Common Issues

#### Build Failures
```bash
# Check ONNX Runtime setup
ls -la /opt/onnxruntime/lib/

# Verify target installation
rustup target list --installed

# Clear cache
cargo clean
rm -rf ~/.cargo/registry/cache
```

#### Docker Issues  
```bash
# Check multi-arch support
docker buildx ls

# Inspect layers
docker history magray:cpu

# Debug health check
docker run magray:cpu /usr/local/bin/healthcheck.sh status
```

#### Performance Problems
```bash
# Profile build time
cargo build --release --timings

# Analyze binary size
cargo bloat --release --crates

# Run benchmarks
cargo bench -- --save-baseline main
```

---

## 📞 Support & Maintenance

### Monitoring Dashboards
- **Repository Health**: Daily health score tracking
- **Build Performance**: Build time and size trends  
- **Security Status**: Vulnerability and compliance tracking
- **Release Metrics**: Release frequency and success rate

### Maintenance Tasks
- **Weekly**: Dependency updates review
- **Monthly**: Security audit comprehensive review
- **Quarterly**: Performance benchmark baseline updates
- **Annually**: CI/CD pipeline architecture review

### Contact Information
- **DevOps Team**: `#devops-magray` Slack channel
- **Security Issues**: `security@magray.dev`
- **CI/CD Issues**: `ci-cd@magray.dev`

---

**📅 Last Updated**: 2025-08-06  
**🔧 Pipeline Version**: 1.0.0  
**📊 Pipeline Health**: ✅ All systems operational