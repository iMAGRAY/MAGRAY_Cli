# Локально‑первый AI‑ассистент — План исполнения v2 (Memory‑first, для ИИ‑агента)

## 0) Режим продакшен‑готовности (всегда включён)
- Работай предельно тщательно, внимательно и критично к себе и результатам. Проверяй предположения экспериментами и измерениями.
- Цель — **безупречно работающий продукт**, а не «зелёные тесты». Моки используются только на границах системы.
- Каждый инкремент — **семантически полный** (не заглушка), сопровождается тестами, мини‑ADR (что/почему/альтернативы), и предпросмотром/diff.
- Перед побочными эффектами — `preview/diff/dry‑run`, поддержка отката и отмены.
- Репозиторий всегда чист: нет мёртвого кода, лишних файлов, устаревшей документации.

## 1) Северная звезда UX
- Мгновенная полезность в первые 5 минут без настройки.
- План → Предпросмотр → Выполнение с явными последствиями.
- Каждый ответ подтверждён источниками из памяти (цитаты, пути к файлам, таймкоды).
- Фоновые индексации/компакции памяти не мешают пользователю.
- Flows решают крупные задачи без микроменеджмента; память подаёт релевантные артефакты.

## 2) Персоны и пути
Разработчик / Инженер‑дизайнер / Менеджер‑создатель / Privacy‑first. Память выстроена под быстрый и точный контекст в их сценариях.

## 3) Обзор (Memory‑first Fabric)
**Слои**
- **Domain:** `Task`, `Intent`, `Plan`, `MemoryItem`, `MemoryLink`, `RecallQuery`, `RecallResult`; контракты `Retriever`, `Reranker`, `VectorIndex`, `DocStore`, `Policy`.
- **Application:** Planner / Executor / Critic / Scheduler / **MemoryOrchestrator**; use‑cases `chat`, `smart`, `tool`, `tasks`, `memory`, `models`.
- **Adapters:** LLM (локальные/облачные), рантайм инструментов (WASI/subprocess/MCP), пайплайны эмбеддингов (текст/код/изображения/аудио/видео), OS‑интеграции.
- **Infrastructure:** конфиги‑профили, логирование/трейсинг, TUI/CLI, песочница, подписанные обновления, менеджер моделей, **Storage Fabric** (vector + sparse + KV).

**EventBus (с backpressure)**: `intent`, `plan`, `tool.invoked`, `fs.diff`, `memory.ingest`, `memory.compact`, `memory.gc`, `memory.eval`, `policy.block`, `job.progress`, `llm.tokens`, `error`.

**Actor‑модель:** каждый компонент — актор (Tokio) с SLA/таймаутами/ретраями/бюджетами.

## 4) Мульти‑агентные циклы
- **IntentAnalyzer** — извлечение слотов, валидация JSON‑контрактами.
- **Planner** — строит `ActionPlan`, запрашивает контекст у **MemoryOrchestrator** (в т.ч. tool‑hints/flows).
- **Executor** — детерминированное исполнение, параллельные группы, saga‑компенсации, отмена.
- **Critic/Reflector** — проверка результатов, свёртки в память с привязкой к первоисточникам.
- **Scheduler** — индекс/компакт/GC/прогрев кэшей и nightly‑оценка качества памяти (golden‑наборы).

## 5) Платформа инструментов (плагины)
- Манифест `tool.json`: name, version, commands[], JSON Schema аргументов, permissions, timeouts, side‑effects, required‑caps.
- Рантайм: WASI (wasmtime) или subprocess JSON‑RPC/MCP; потоковый вывод, heartbeats, отмена.
- Безопасность: capability‑модель (fs roots, net allowlist, shell off), подписи и доверенные каналы обновления.
- UX‑обязательства: dry‑run, auto‑diff, shadow‑execute (эмуляция).
- Рецепты (flows): декларативный YAML/DSL с типами/валидацией, условиями, циклами, шаблонами.

### 5.1 MCP
Клиент MCP (WS/JSON‑RPC), импорт инструментов в `ToolSpec`, кэш/реестр, политики allowlist, события `mcp.connected/tool_invoked/error`, устойчивость (fallback).

### 5.2 Tool Context Builder
Кандидаты: встроенные + плагины + MCP → предфильтры (policy/OS/GPU/контекст) → embedding‑поиск по usage‑guides → rerank → компактный список top‑N в подсказку для Planner/LLM.

### 5.3 UsageGuide генерация
Авто‑сводка при регистрации: `usage_title/summary`, `preconditions`, `arguments_brief`, `good_for/not_for`, `constraints`, `examples`, `platforms`, `cost/latency_class`, `side_effects`, `risk_score`, `capabilities`, `tags` + эмбеддинг.

---

## 6) Память — мультимодальная, сверхэффективная (ядро)

### 6.1 Эмбеддинги (локально)
- **Текст/код:** fastembed (Rust) с моделями *bge‑m3* / *mxbai‑embed‑large‑v1* / *nomic‑embed‑text‑v1.5*; для кода — чанкирование по AST/символам.
- **Изображения:** OpenCLIP ViT‑H/14 (или ViT‑L/14) ONNX; дополнительно SigLIP/SigLIP2 ONNX.
- **Аудио:** LAION‑CLAP (HTSAT) ONNX; VAD (webrtc‑vad/силеро), шумоподавление (nnnoiseless/dtln‑rs).
- **Видео:** ключевые кадры (ffmpeg/gstreamer) → CLIP/SigLIP по кадрам → агрегирование (NetVLAD/усреднение).

### 6.2 Индексы/хранение
- **Dense:** Qdrant (локально), HNSW/IVF, multi‑vector, payload‑фильтры, компрессия (scalar/IVF‑PQ). Коллекции:\
  `text_code {text:1024, code:1024} @ HNSW{m=32, efC=256, efS=64}`;\
  `vision {clip:1024} @ HNSW{m=32, efC=256, efS=64}`;\
  `audio {clap:768} @ HNSW{m=32, efC=256, efS=64}`.
- **Embedded‑fallback:** `hnsw_rs + redb` (persist/меммап, SIMD‑дистанции) — режим без внешних сервисов.
- **Sparse:** Tantivy (BM25/QL) с анализаторами для кода/ест. языка; fallback — SQLite FTS5.
- **KV/метаданные:** redb/SQLite: `source`, `project`, `modality`, `timestamp`, `quality`, `permissions`, линковка и версии.

### 6.3 Поиск/контекст
- **Гибрид:** `dense ∪ sparse` → **rerank** (bge‑reranker‑large ONNX или Qwen‑reranker ~0.6B) с ранним выходом по уверенности.
- **Скоринг по умолчанию:** `score = 0.58·dense + 0.22·sparse + 0.12·recency + 0.08·source_quality`.
- **Multi‑vector запросы:** объединяем текст, ключевые термины, имена символов/файлов, визуальные подсказки.
- **Дедуп/сглаживание:** SimHash/MinHash; группировка по источнику/сцене/функции.
- **Контекст‑пакет:** строгий лимит токенов, «доказательные» фрагменты (ссылка+цитата), «evidence windows» для длинных ответов.

### 6.4 Индексация
Инкрементальные вотчеры (fs/git), чанкирование (токены/AST/сцены), батч‑эмбеддинг, компрессия, PII‑скан, журнал событий для **replay** индекса; прогрев «горячих» проектов/каталогов.

### 6.5 Качество памяти (метрики/SLO)
- **Метрики:** recall@k, nDCG@k, MRR, latency P95/P99 (поиск/индекс), кросс‑модальная согласованность.
- **SLO по умолчанию:** recall@20 ≥ 0.85 (golden), P95 retrieve ≤ 80 мс (локально, тёплый кэш), индекс ≥ 2e5 чанк/час на CPU mid‑range.
- Непрерывная оценка: nightly golden‑прогоны, регресс‑алерты, панель «Memory Health».

### 6.6 Приватность/комплаенс
Локальное шифрование профилей/кэшей; право на забвение (selective delete + reindex); секрет‑скан в текстах/метаданных; нулевой сетевой доступ без согласия.

---

## 7) LLM‑оркестрация
Единый потоковый API локальных/облачных LLM; **memory‑gate** (reranker‑вратарь) перед длинным контекстом; спекулятивное декодирование; дедуп контекста; ретраи/таймауты; fallback на локальную модель.

## 8) Политики/Безопасность
Policy DSL: права fs/net/shell/git; риск‑скоринг шага плана; UX‑подтверждения; локальный vault; audit событий памяти (кто/что/когда извлёк).

## 9) Производительность/устойчивость
Раздельные пулы (LLM/FS/Index/Rerank), квоты, backpressure; mmap‑хранилище HNSW; LRU‑кэши эмбеддингов по контент‑хэшу; батчи эмбеддинга, многопоточный rerank; flamegraph тяжёлых сценариев.

## 10) TUI/CLI (опыт)
Интерактивный план и статусы; Дифф‑центр (FS/Git) с симуляцией; Таймлайн событий/инструментов/памяти; Навигатор памяти (RAG‑источники/цитаты/граф связей/фильтры); Flow‑студия с валидацией.

---

## 11) Контракты (минимальные)

### 11.1 MemoryItem
```json
{
  "id": "mi_01F...",
  "modality": "text|code|image|audio|video",
  "source": {"path":"...", "repo":"...", "commit":"...", "timestamp":"..."},
  "embeddings": {"text":[...], "code":[...], "clip":[...], "clap":[...]},
  "sparse": {"terms":["..."], "weights":[...]},
  "meta": {"project":"...", "language":"rs", "scope":"fn parse()", "pii": false}
}
```

### 11.2 RecallQuery
```json
{
  "text": "как устроен планировщик",
  "signals": {"code_symbols": ["scheduler","actor"], "file_hints": ["orchestrator/..."]},
  "modality": ["text","code"],
  "k": 200
}
```

### 11.3 RecallResult
```json
{
  "items": [
    {"id":"mi_...","score":0.91,"snippet":"...","cite":{"path":"...","line":123}}
  ],
  "took_ms": 42
}
```

---

## 12) Предпочитаемые технологии (срез 2025‑08‑09)
- **Async/runtime:** Tokio. **Web/IPC:** axum (hyper v1, tower), tonic (gRPC). **CLI/TUI:** clap, ratatui. **UI:** Tauri 2.
- **Parsing/serde:** serde, schemars. **Config:** figment/config + layered profiles.
- **Наблюдаемость:** tracing (+opentelemetry), structured logs; человеко‑понятные диагностики.
- **Хранилища:** SQLx (SQLite primary, Postgres по профилю), **Tantivy** (FTS), **redb** (встраиваемый KV).
- **Vector Search:** **Qdrant (локально)** + qdrant‑client; *fallback* — **hnsw_rs + redb**.
- **Аудио/медиа:** cpal, Symphonia, rubato/fixed‑resample, ffmpeg‑next/gstreamer‑rs.
- **ML‑инференс:** ONNX Runtime (CPU/GPU), candle (легковесный), tch‑rs (точечно). Веса — ONNX/gguf; mmap + частичная выгрузка.
- **Качество:** nextest, proptest, insta, cargo‑fuzz, cargo‑llvm‑cov, cargo‑mutants. **Supply chain:** cargo‑deny/audit/vet.

---

## 13) SLO/DoD/Гейты качества

### 13.1 SLO (по умолчанию)
- **Retrieve P95:** ≤ 80 мс (локально, тёплый кэш). **Index throughput:** ≥ 200k чанков/час (CPU mid‑range).
- **Recall@20:** ≥ 0.85 (golden наборы). **Crash‑free/неделя:** ≥ 99.9%.

### 13.2 DoD (жёстко)
1) Фича воспроизводимо работает в превью и реальном запуске.  
2) Все функции покрыты тестами, подтверждающими задуманное поведение; нет флаки/фальш‑позитивов.  
3) Линтеры/статанализ/безопасность — чисто; ключевые пути уложились в бюджеты.  
4) Репозиторий чист: нет мёртвого кода/лишних файлов/устаревших доков.  
5) UX соответствует ожиданиям: ясные диагностики, предсказуемая латентность, нулевая толерантность к багам.

### 13.3 CI‑гейты (минимум)
- Unit/integration/property/e2e проходят стабильно (2 прогонов подряд).
- `cargo‑llvm‑cov` пороги по критическим путям; `cargo‑mutants` без критических дыр.
- Golden‑оценка памяти без регрессов по recall/nDCG/latency.
- `cargo‑deny/audit/vet` чисто; SBOM/подписи релизов проверены.

---

## 14) Дорожная карта (память‑фокус)

**Фаза A :** Memory‑fabric MVP (текст/код, Qdrant text_code, Tantivy, гибрид + rerank), Навигатор памяти, EventBus.  
**Фаза B :** Изображения/аудио (CLIP/CLAP), multi‑vector, SimHash‑дедуп, golden‑наборы, «Memory Health».  
**Фаза C :** Видео (кадры+агрегация), embedded‑fallback (hnsw_rs+redb), GC/compact, агрессивная оптимизация латентности.  
**Фаза D :** LLM‑оптимизатор с memory‑gate, подписи/обновления плагинов памяти, лёгкий KG‑слой для навигации.

---

## 15) Эксплуатационные политики
- Default‑deny для fs/net/shell; любые рисковые действия — только после явного согласия пользователя.
- Секреты: вне логов/артефактов; локальное шифрование; ротация при необходимости.
- Никакой анонимизирующей телеметрии; без сетевых вызовов без согласия пользователя.

---

## 16) Риски и смягчение
- Дрейф релевантности — регулярные golden‑прогоны, авто‑тюнинг весов `α/β/γ/δ`, мониторинг query drift.
- Обновления моделей — версионирование эмбеддингов; параллельные коллекции (старые/новые) + A/B‑сравнение; миграции индекса.
- Латентность на CPU — батчи, кэш токенизации/векторов, квоты тяжёлых модальностей, профайлинг и flamegraph.

---

## 17) Структура репозитория (ориентир)
```
crates/
  core/              # доменные модели/контракты
  orchestrator/      # planner/executor/critic/scheduler + EventBus
  memory_fabric/     # эмбеддинги, индексы (Qdrant/Tantivy/redb), retriever, reranker, GC, compact
  memory_eval/       # golden-наборы, метрики, панели
  tools/             # runtime плагинов, registry, MCP-клиент
  llm/               # провайдеры, memory-gate, оптимизатор
  ui/                # TUI/CLI (план/дифф/панель памяти/таймлайн)
  recipes/           # DSL/валидация/исполнение flows
  policy/            # Policy DSL, риск-оценка, интеграция UX
models/
  text/ code/ clip/ clap/ reranker/
configs/
  config.example.toml
```
