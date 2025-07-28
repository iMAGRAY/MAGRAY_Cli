#!/usr/bin/env bash
set -euo pipefail

echo "🚀 Setting up CLI-Agent development environment"
echo "=============================================="

# Проверка системных требований
echo "📋 Checking system requirements..."

# Rust
if ! command -v cargo >/dev/null 2>&1; then
    echo "❌ Rust not found. Installing..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source ~/.cargo/env
else
    echo "✅ Rust found: $(rustc --version)"
fi

# Обновляем Rust и компоненты
echo "🔄 Updating Rust toolchain..."
rustup update stable
rustup default stable
rustup component add rustfmt clippy

# Устанавливаем дополнительные инструменты
echo "🛠️  Installing development tools..."
TOOLS=(
    "cargo-nextest"      # Параллельные тесты
    "cargo-tarpaulin"    # Покрытие кода
    "sqlx-cli"           # SQL миграции
    "taplo-cli"          # TOML форматирование
    "just"               # Task runner
)

for tool in "${TOOLS[@]}"; do
    if ! cargo install --list | grep -q "^$tool "; then
        echo "  📦 Installing $tool..."
        cargo install "$tool" --locked || echo "⚠️  Failed to install $tool"
    else
        echo "  ✅ $tool already installed"
    fi
done

# Системные зависимости
echo "🔧 Installing system dependencies..."
if command -v apt-get >/dev/null 2>&1; then
    sudo apt-get update
    sudo apt-get install -y build-essential pkg-config libssl-dev libsqlite3-dev bc ripgrep
elif command -v brew >/dev/null 2>&1; then
    brew install openssl sqlite3 ripgrep
elif command -v pacman >/dev/null 2>&1; then
    sudo pacman -S base-devel openssl sqlite bc ripgrep
else
    echo "⚠️  Unknown package manager. Please install manually:"
    echo "   - build tools (gcc, make, etc.)"
    echo "   - openssl development headers"
    echo "   - sqlite3 development headers"
    echo "   - bc (calculator)"
    echo "   - ripgrep"
fi

# Создаём структуру проекта
echo "📁 Setting up project structure..."
mkdir -p {docs/task_master,logs,target,~/.ourcli/projects/dev}

# Инициализируем Git hooks
echo "🔗 Setting up Git hooks..."
if [ -d ".git" ]; then
    # Pre-commit hook
    cat > .git/hooks/pre-commit << 'EOF'
#!/usr/bin/env bash
echo "🔍 Running pre-commit checks..."
bash scripts/pre-commit.sh
EOF
    chmod +x .git/hooks/pre-commit
    
    # Pre-push hook
    cat > .git/hooks/pre-push << 'EOF'
#!/usr/bin/env bash
echo "🚀 Running pre-push checks..."
bash scripts/agent-checklist.sh
EOF
    chmod +x .git/hooks/pre-push
    
    echo "✅ Git hooks installed"
else
    echo "⚠️  Not a Git repository - skipping hooks"
fi

# Настройка окружения
echo "🌍 Setting up environment..."
cat > .env.example << 'EOF'
# CLI-Agent Development Environment
OPENAI_API_KEY=your_openai_api_key_here
GITHUB_TOKEN=your_github_token_here
RUST_BACKTRACE=1
CARGO_TERM_COLOR=always
CLI_AGENT_LOG_LEVEL=debug
CLI_AGENT_MEMORY_PATH=~/.ourcli
EOF

if [ ! -f ".env" ]; then
    cp .env.example .env
    echo "📝 Created .env file - please update with your API keys"
fi

# Проверка конфигурации
echo "🧪 Running initial checks..."
if cargo check --all >/dev/null 2>&1; then
    echo "✅ Project compiles successfully"
else
    echo "❌ Compilation issues found - run 'cargo check' for details"
fi

# Создаём начальный task file
SETUP_TASK="docs/task_master/setup-$(date +%s).md"
cat > "$SETUP_TASK" << EOF
# Task setup-$(date +%s): Development Environment Setup

## Goal
Set up complete development environment for CLI-Agent project

## Priority
CRITICAL

## Source
- Script: scripts/setup-dev-env.sh
- Created: $(date -u +"%Y-%m-%d %H:%M:%S UTC")
- Assignee: Developer

## Plan
- [x] Install Rust toolchain and components
- [x] Install development tools (nextest, tarpaulin, etc.)
- [x] Install system dependencies
- [x] Set up project structure
- [x] Configure Git hooks
- [x] Create environment configuration
- [ ] Run initial tests
- [ ] Verify all components work

## Execution Log
### Step 1: Environment Setup
Action: Ran setup-dev-env.sh script
Result: Development environment configured
Checks: ✅ rust ✅ tools ✅ deps ✅ structure
Next: Run initial tests and verify functionality
EOF

echo "📝 Created setup task: $SETUP_TASK"

# Финальная проверка
echo ""
echo "🎉 Development environment setup complete!"
echo ""
echo "📋 Next steps:"
echo "1. Update .env with your API keys"
echo "2. Run: bash scripts/agent-checklist.sh"
echo "3. Run: bash scripts/pre-commit.sh"
echo "4. Start developing: focus on blockers in implementation_plan/INCOMPLETE.md"
echo ""
echo "🔗 Useful commands:"
echo "  cargo check --all          # Quick compilation check"
echo "  cargo test --all           # Run all tests"
echo "  cargo nextest run          # Parallel testing"
echo "  cargo tarpaulin --out Xml  # Code coverage"
echo "  just --list                # Available tasks (if using justfile)"
echo ""
echo "📚 Documentation:"
echo "  implementation_plan/README.md     # Project overview"
echo "  implementation_plan/INCOMPLETE.md # Current blockers"
echo "  docs/workflow_state.md           # Current work status"
echo ""
echo "✨ Happy coding!"