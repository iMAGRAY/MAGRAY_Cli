# EventBus / Tracing Events

## Основные типы
- `RequestAccepted`
- `PlanBuilt`
- `TaskStarted { node_id }`
- `TaskFinished { node_id, ok }`
- `TaskFailed { node_id, error }`
- `TodoCreated / TodoStateChanged`
- `MemoryIngested { ref }`
- `EmbeddingCachedHit`
- `PolicyViolation`
- `SchedulerJobRun { job_name }`

## Логи
- `events.log` — человеко-читаемый + JSON
- `metrics.json` — агрегированные метрики (latency, cache-hit, counts)
