# CLI Agent

Локальный CLI-агент с 5-слойной памятью, семантическим роутингом (BGE3/BG3), Todo/TaskBoard и DAG‑планировщиком.

## Быстрый старт
1. `cli init` — создаёт DocStore проекта в `~/.ourcli/projects/<id>/`.
2. `cli todo add "сделать анализ репо"` — задача попадёт в TaskBoard.
3. `cli run "анализ репо"` — агент построит план и выполнит его.
4. `cli vec reindex` / `cli mem compact` — фоновые операции.

## Документация
- `docs/ARCHITECTURE.md` — полное описание архитектуры
- `docs/DIAGRAMS.md` — все диаграммы Mermaid
- `docs/SEQUENCES.md` — sequence диаграммы
- `docs/TODO_SPEC.md` — спецификация TaskBoard
- `docs/EVENTS.md` — события EventBus
- `config/config.example.toml` — пример конфига
- `db/schema.sql` — схемы SQLite таблиц
- `rust_skeleton/` — минимальный каркас на Rust

## Лицензия
MIT (или выбери свою)

