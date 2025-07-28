#!/bin/bash
set -euo pipefail

# Цвета для вывода
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Функция для вывода с цветом
print_color() {
    local color=$1
    local message=$2
    echo -e "${color}${message}${NC}"
}

# Проверка аргументов
if [ $# -lt 1 ]; then
    print_color $RED "❌ Использование: $0 <project_name> [author] [description]"
    exit 1
fi

PROJECT_NAME=$1
AUTHOR=${2:-"$(git config user.name || echo 'Anonymous')"}
DESCRIPTION=${3:-"A CLI tool built with Rust"}
GITHUB_USER=$(git config user.name || echo "username")

print_color $BLUE "🚀 Создаем новый CLI проект: $PROJECT_NAME"

# Создаем структуру проекта
print_color $YELLOW "📁 Создаем структуру директорий..."
mkdir -p "$PROJECT_NAME"/{src/{commands,ui},tests,docs,.github/workflows}

# Копируем шаблоны
print_color $YELLOW "📝 Копируем шаблоны..."

# Cargo.toml
sed -e "s/{{project_name}}/$PROJECT_NAME/g" \
    -e "s/{{author}}/$AUTHOR/g" \
    -e "s/{{description}}/$DESCRIPTION/g" \
    -e "s/{{github_user}}/$GITHUB_USER/g" \
    templates/cli-starter/Cargo.toml > "$PROJECT_NAME/Cargo.toml"

# main.rs
sed -e "s/{{project_name}}/$PROJECT_NAME/g" \
    -e "s/{{description}}/$DESCRIPTION/g" \
    templates/cli-starter/src/main.rs > "$PROJECT_NAME/src/main.rs"

# Создаем базовые модули
cat > "$PROJECT_NAME/src/commands/mod.rs" << 'EOF'
pub mod analyze;
pub mod config;
pub mod create;
pub mod init;
EOF

cat > "$PROJECT_NAME/src/ui/mod.rs" << 'EOF'
use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, Select};

pub async fn interactive_menu() -> Result<()> {
    let items = vec!["Init", "Create", "Analyze", "Config", "Exit"];
    
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("What would you like to do?")
        .items(&items)
        .default(0)
        .interact()?;

    match selection {
        0 => println!("Selected: Init"),
        1 => println!("Selected: Create"),
        2 => println!("Selected: Analyze"),
        3 => println!("Selected: Config"),
        4 => std::process::exit(0),
        _ => unreachable!(),
    }

    Ok(())
}
EOF

cat > "$PROJECT_NAME/src/config.rs" << 'EOF'
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub version: String,
}

pub fn load() -> Result<Config> {
    Ok(Config {
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}
EOF

# Создаем заглушки для команд
for cmd in init create analyze config; do
    mkdir -p "$PROJECT_NAME/src/commands"
    cat > "$PROJECT_NAME/src/commands/$cmd.rs" << 'EOF'
use anyhow::Result;
use colored::Colorize;

pub async fn run() -> Result<()> {
    println!("{}", "Command not implemented yet".yellow());
    Ok(())
}
EOF
done

# README.md
cat > "$PROJECT_NAME/README.md" << EOF
# $PROJECT_NAME

$DESCRIPTION

## Installation

\`\`\`bash
cargo install --path .
\`\`\`

## Usage

\`\`\`bash
$PROJECT_NAME --help
\`\`\`

## Development

\`\`\`bash
# Run in development mode
cargo run -- --help

# Run tests
cargo test

# Build release version
cargo build --release
\`\`\`

## License

MIT OR Apache-2.0
EOF

# .gitignore
cp .gitignore "$PROJECT_NAME/.gitignore"

# GitHub Actions
cat > "$PROJECT_NAME/.github/workflows/ci.yml" << 'EOF'
name: CI

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

env:
  CARGO_TERM_COLOR: always

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - uses: Swatinem/rust-cache@v2
    - run: cargo test --all-features

  fmt:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt
    - run: cargo fmt --all -- --check

  clippy:
    runs-on: ubuntu-latest
    steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: clippy
    - uses: Swatinem/rust-cache@v2
    - run: cargo clippy --all-features -- -D warnings
EOF

# Инициализируем git
cd "$PROJECT_NAME"
git init
git add .
git commit -m "Initial commit: $PROJECT_NAME CLI"

print_color $GREEN "✅ Проект $PROJECT_NAME успешно создан!"
print_color $BLUE "📖 Следующие шаги:"
echo "   1. cd $PROJECT_NAME"
echo "   2. cargo run -- --help"
echo "   3. Начните разработку!"