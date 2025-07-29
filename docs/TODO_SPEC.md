# TodoService / TaskBoard Spec

## Модель
- `id: Uuid`
- `title: String`
- `desc: String`
- `state: Planned|Ready|InProgress|Blocked|Done|Archived`
- `priority: i32`
- `deps: Vec<Uuid>`
- `tags: Vec<String>`
- `created_at, due_at, last_touch: DateTime`
- `staleness: f32`

## Операции CLI
- `todo add "title" --desc "..." --due 2025-08-01 --tag foo`
- `todo list --state ready --sort priority`
- `todo done <id>` / `todo snooze <id> --for 3d`
- `todo prune --stale`

## Staleness Score
Пример: `staleness = f(now - last_touch, due_at - now, deps_blocked)`

## Интеграция
- Planner связывает TodoItem с узлами DAG (TaskNode.task_id).
- Executor обновляет состояния (started/done/failed).
- Scheduler периодически пересматривает задачи.
