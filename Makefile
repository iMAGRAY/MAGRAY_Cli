.PHONY: help build test clean dev check fix bench doc

help: ## Показать это сообщение
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}'

build: ## Собрать проект в release режиме
	cargo build --release

dev: ## Собрать проект в dev режиме
	cargo build

test: ## Запустить все тесты
	cargo test --workspace --all-features

test-verbose: ## Запустить тесты с подробным выводом
	cargo test --workspace --all-features -- --nocapture

check: ## Проверить код (fmt, clippy, тесты)
	cargo fmt -- --check
	cargo clippy --workspace --all-features -- -W clippy::all
	cargo test --workspace --all-features

fix: ## Автоматически исправить проблемы
	cargo fmt
	cargo clippy --workspace --all-features --fix --allow-dirty
	cargo fix --workspace --allow-dirty

bench: ## Запустить бенчмарки
	cargo bench --workspace

doc: ## Сгенерировать документацию
	cargo doc --workspace --all-features --no-deps --open

clean: ## Очистить артефакты сборки
	cargo clean
	rm -rf target/

watch: ## Запустить в режиме отслеживания изменений
	cargo watch -x check -x test -x run

new-app: ## Создать новое приложение (использование: make new-app NAME=myapp)
	@if [ -z "$(NAME)" ]; then echo "Ошибка: укажите NAME=имя_приложения"; exit 1; fi
	cargo new apps/$(NAME) --bin
	@echo "Создано новое приложение: apps/$(NAME)"

new-lib: ## Создать новую библиотеку (использование: make new-lib NAME=mylib)
	@if [ -z "$(NAME)" ]; then echo "Ошибка: укажите NAME=имя_библиотеки"; exit 1; fi
	cargo new libs/$(NAME) --lib
	@echo "Создана новая библиотека: libs/$(NAME)"

update-deps: ## Обновить зависимости
	cargo update
	cargo outdated

setup: ## Настроить окружение разработки
	rustup update
	rustup component add rustfmt clippy
	cargo install cargo-watch cargo-outdated cargo-audit
	@echo "Окружение настроено!"