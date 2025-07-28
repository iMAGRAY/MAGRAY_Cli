# Planning Agent Checklist

1. Прочитай задачу и контекст репо.
2. Сделай semantic retrieval (M4) → rerank → MemRef → оригиналы.
3. Построй DAG: шаги, зависимости, критерии завершения.
4. Вставь проверки (lint/tests/policy) для каждого шага.
5. Обнови `docs/workflow_state.md`.
