    # Диаграммы (Mermaid)

    ## Общая архитектура
    ```mermaid
    %% скопировано из ARCHITECTURE.md

flowchart LR
    %% ===== Entry =====
    CLI[[CLI]] --> GW[Request Gateway]

    %% ===== Core =====
    subgraph CORE[Core]
        GW --> TODO[TodoService / TaskBoard]
        TODO --> PL[Graph Planner (DAG)]
        PL --> EXE[Executor]
        EXE --> EVT[(EventBus / Tracing)]
        EXE --> POL[Policy / Guardrails]
    end

    %% ===== Shared Services =====
    subgraph SRV[Shared Services]
        MEMC[Memory Coordinator + Semantic Router]
        TOOLS[ToolService (Registry+Selector+Descriptor)]
        PROMPT[PromptBuilder]
        VEC[VectorizerSvc (BGE3)]
        RER[RerankSvc (BG3)]
        LLM[LLM Clients (local-first)]
        CFG[Config / Feature Flags]
        SCH[Scheduler / Background Jobs]
    end

    %% ===== 5-Layer Memory =====
    subgraph MEM["Memory Layers (5)"]
        M0[Ephemeral / Scratch (RAM)]
        M1[ShortTerm (SQLite KV)]
        M2[MediumTerm (SQL Tables)]
        M3[LongTerm (Blobs/Archives)]
        M4[Semantic Index (Vectors)]
    end

    %% ===== CLI DocStore (per-project, persistent, local) =====
    subgraph DOCS["CLI DocStore (per project)"]
        CFGF[(config.toml)]
        SQL[(sqlite.db : M1/M2, tools, meta)]
        TASKS[(tasks.db : todo)]
        BLOBS[(blobs/ : M3)]
        VDB[(vectors.idx / qdrant_dir : M4)]
        CACHE[(embed_cache.db)]
        LOGS[(events.log / metrics.json)]
    end

    EXE --> MEMC
    EXE --> PROMPT
    EXE --> TOOLS
    CFG --> GW
    CFG --> PL
    CFG --> EXE
    CFG --> SRV
    SCH --> MEMC
    SCH --> TOOLS
    SCH --> VEC
    SCH --> TODO
    SCH --> CACHE

    %% Memory wiring
    MEMC --> M0
    MEMC --> M1
    MEMC --> M2
    MEMC --> M3
    MEMC --> M4

    %% Storage bindings
    M1 --> SQL
    M2 --> SQL
    M3 --> BLOBS
    M4 --> VDB
    TOOLS --> SQL
    TODO --> TASKS
    VEC --> VDB
    VEC --> CACHE
    EVT -.-> LOGS

    %% Prompt / NLU
    PROMPT --> VEC
    PROMPT --> RER
    PROMPT --> LLM
    RER --> EXE
    M4 --> MEMC
```
    ## Уточнённая (с Semantic Router внутри MemoryCoordinator)
    ```mermaid
    flowchart LR
        CLI[[CLI]] --> GW[Request Gateway]
        subgraph CORE[Core]
            GW --> TODO[TodoService / TaskBoard]
            TODO --> PL[Graph Planner (DAG)]
            PL --> EXE[Executor]
            EXE --> EVT[(EventBus / Tracing)]
            EXE --> POL[Policy / Guardrails]
        end
        subgraph SRV[Shared Services]
            MEMC[Memory Coordinator + Semantic Router]
            TOOLS[ToolService]
            PROMPT[PromptBuilder]
            VEC[VectorizerSvc (BGE3)]
            RER[RerankSvc (BG3)]
            LLM[LLM Clients]
            CFG[Config]
            SCH[Scheduler]
        end
        subgraph MEM[Memory Layers]
            M0[Ephemeral]
            M1[ShortTerm]
            M2[MediumTerm]
            M3[LongTerm]
            M4[Semantic Index]
        end
        subgraph DOCS[DocStore per project]
            SQL[(sqlite.db)]
            TASKS[(tasks.db)]
            BLOBS[(blobs/)]
            VDB[(vectors/)]
            CACHE[(embed_cache.db)]
            LOGS[(events.log)]
            CFGF[(config.toml)]
        end
        EXE --> MEMC
        EXE --> PROMPT
        EXE --> TOOLS
        CFG --> GW
        CFG --> PL
        CFG --> EXE
        CFG --> SRV
        SCH --> MEMC
        SCH --> TOOLS
        SCH --> VEC
        SCH --> TODO
        SCH --> CACHE
        MEMC --> M0-->M1-->M2-->M3
        MEMC --> M4
        M1 --> SQL
        M2 --> SQL
        M3 --> BLOBS
        M4 --> VDB
        TOOLS --> SQL
        TODO --> TASKS
        VEC --> VDB
        VEC --> CACHE
        EVT -.-> LOGS
        PROMPT --> VEC
        PROMPT --> RER
        PROMPT --> LLM
        RER --> EXE
        M4 --> MEMC
    ```
