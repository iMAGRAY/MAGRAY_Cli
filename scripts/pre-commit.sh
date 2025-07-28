#!/usr/bin/env bash
set -euo pipefail

echo "🔍 [CLI-Agent Pre-commit] Multi-level quality checks"

# Определяем уровень проверки на основе изменённых файлов
CHANGED_FILES=$(git diff --cached --name-only 2>/dev/null || echo "")
CRITICAL_COMPONENTS="toolsvc|llm|memory|executor"

if echo "$CHANGED_FILES" | grep -qE "$CRITICAL_COMPONENTS"; then
    CHECK_LEVEL="L2"
    echo "🚨 Critical components changed - running L2 checks"
else
    CHECK_LEVEL="L1" 
    echo "📝 Standard changes - running L1 checks"
fi

# L1: Базовые проверки (всегда)
echo "📋 L1: Basic checks"
if command -v cargo >/dev/null 2>&1; then
    echo "  🦀 Rust formatting..."
    cargo fmt --all
    
    echo "  🔧 Rust linting..."
    if ! cargo clippy --all-targets -- -D warnings; then
        echo "❌ Clippy failed - attempting auto-fix"
        cargo clippy --all-targets --fix --allow-dirty --allow-staged || true
    fi
    
    echo "  ✅ Rust compilation..."
    cargo check --all
    
    echo "  🧪 Unit tests..."
    cargo test --all --quiet
fi

# TOML форматирование
if command -v taplo >/dev/null 2>&1; then
    echo "  📄 TOML formatting..."
    taplo fmt **/*.toml || true
fi

# L2: Качество (для критических компонентов)
if [ "$CHECK_LEVEL" = "L2" ]; then
    echo "🎯 L2: Quality checks for critical components"
    
    # Параллельные тесты
    if command -v cargo-nextest >/dev/null 2>&1; then
        echo "  🚀 Parallel testing..."
        cargo nextest run --all
    fi
    
    # Покрытие кода
    if command -v cargo-tarpaulin >/dev/null 2>&1; then
        echo "  📊 Code coverage..."
        cargo tarpaulin --out Xml --timeout 120 --skip-clean
        
        # Проверяем покрытие критических модулей
        if [ -f cobertura.xml ]; then
            COVERAGE=$(grep -o 'line-rate="[^"]*"' cobertura.xml | head -1 | cut -d'"' -f2)
            COVERAGE_PERCENT=$(echo "$COVERAGE * 100" | bc -l | cut -d. -f1)
            echo "  📈 Coverage: ${COVERAGE_PERCENT}%"
            
            if [ "$COVERAGE_PERCENT" -lt 60 ]; then
                echo "⚠️  Coverage below 60% - consider adding tests"
            fi
        fi
    fi
    
    # Проверка SQL миграций
    if command -v sqlx >/dev/null 2>&1 && [ -d "migrations" ]; then
        echo "  🗄️  Database migrations..."
        sqlx migrate info || echo "⚠️  Migration check failed"
    fi
    
    # Проверка на заглушки в критических компонентах
    echo "  🔍 Checking for stubs in critical components..."
    STUBS_FOUND=$(grep -r "#INCOMPLETE\|TODO.*BLOCKER\|fake_content\|заглушка" \
        --include="*.rs" \
        implementation_plan/rust_skeleton/crates/ 2>/dev/null || true)
    
    if [ -n "$STUBS_FOUND" ]; then
        echo "⚠️  Found stubs in critical code:"
        echo "$STUBS_FOUND"
        echo "💡 Consider implementing these before committing"
    fi
fi

# Проверка документации
echo "📚 Documentation checks"
if [ -f "implementation_plan/INCOMPLETE.md" ]; then
    # Проверяем, обновлён ли INCOMPLETE.md при изменении кода
    if echo "$CHANGED_FILES" | grep -q "\.rs$" && ! echo "$CHANGED_FILES" | grep -q "INCOMPLETE.md"; then
        echo "💡 Code changed but INCOMPLETE.md not updated - consider updating status"
    fi
fi

# JavaScript/TypeScript (если есть)
if [ -f package.json ]; then
    echo "🌐 JavaScript checks"
    npm run lint || echo "⚠️  JS lint failed"
    npm test -- --coverage --watchAll=false || echo "⚠️  JS tests failed"
fi

# Python (если есть)
if test -f "pyproject.toml" || test -f "requirements.txt"; then
    echo "🐍 Python checks"
    black . || echo "⚠️  Black formatting failed"
    ruff check . || echo "⚠️  Ruff linting failed"
    pytest -q || echo "⚠️  Python tests failed"
fi

echo "✅ Pre-commit checks completed (Level: $CHECK_LEVEL)"

# Возвращаем 0 для успеха (не блокируем коммит на предупреждениях)
exit 0
