# Multi-Agent Orchestration System Documentation

## Overview

The MAGRAY CLI multi-agent orchestration system implements a comprehensive agent-based architecture for intelligent task execution. The system follows the Intent→Plan→Execute→Critic workflow pattern with robust reliability, monitoring, and saga-based transaction management.

## Agent Architecture

### 🧠 IntentAnalyzer Agent
**Purpose**: Analyzes user input and extracts structured intentions

**Key Features**:
- Natural language processing with LLM integration
- Confidence scoring and intent classification
- Context-aware analysis with session management
- Support for multiple intent types (tool execution, questions, file operations, etc.)

**API Contract**: `IntentAnalyzerTrait`

### 🛠️ Planner Agent  
**Purpose**: Converts intents into executable action plans

**Key Features**:
- Structured action plan generation with dependencies
- Resource requirement estimation
- Retry policies and validation rules
- Support for complex workflows (loops, conditionals, user interactions)

**API Contract**: `PlannerTrait`

### ⚡ Executor Agent
**Purpose**: Executes action plans with deterministic and reliable execution

**Key Features**:
- Step-by-step plan execution with state tracking
- Tool invocation integration
- Rollback mechanism for failed executions
- Resource monitoring and timeout management
- Saga pattern integration for transactional consistency

**API Contract**: `ExecutorTrait`

### 🔍 Critic Agent
**Purpose**: Analyzes execution results and provides improvement feedback

**Key Features**:
- Quality metrics evaluation (efficiency, reliability, etc.)
- Success/failure detection with configurable thresholds
- Improvement suggestions with priority classification
- Risk assessment and mitigation recommendations

**API Contract**: `CriticTrait`

### ⏰ Scheduler Agent
**Purpose**: Manages background tasks and job scheduling

**Key Features**:
- Priority-based job queue management
- Cron-style scheduling support
- Job persistence across application restarts
- Resource-aware scheduling with load balancing

**API Contract**: `SchedulerTrait`

## Core Components

### 🎯 AgentOrchestrator
Central coordinator managing the complete agent lifecycle and workflow execution.

**Key Responsibilities**:
- Agent startup and shutdown management
- Workflow orchestration (Intent→Plan→Execute→Critic)
- Health monitoring and agent coordination
- Event-driven communication with EventBus integration

### 🎭 Actor System
Tokio-based actor framework providing message passing and fault tolerance.

**Features**:
- Asynchronous message passing between agents
- Actor lifecycle management (start/stop/restart)
- Supervision strategies for fault tolerance
- Resource budgets and monitoring

### 🔄 Saga Pattern
Implements distributed transaction management with compensation logic.

**Features**:
- Transaction step definitions with compensation actions
- Automatic rollback on failures
- Saga state persistence and recovery
- Integration with executor for transactional workflows

## Getting Started

See the [Integration Guide](./integration-guide.md) for detailed setup and usage instructions.

## Agent Contracts

Detailed API documentation for each agent is available in:
- [IntentAnalyzer API](./intent-analyzer-api.md) ✅ **DOCUMENTED**
- [Planner API](./planner-api.md) ✅ **DOCUMENTED**
- [Executor API](./executor-api.md) ✅ **DOCUMENTED**
- [Critic API](./critic-api.md) - *Available in source code*
- [Scheduler API](./scheduler-api.md) - *Available in source code*
- [Integration Guide](./integration-guide.md) ✅ **COMPREHENSIVE**

## Quick Links

- **🚀 [Get Started](./integration-guide.md#quick-start)** - Basic setup and first workflow
- **📖 [Workflow Patterns](./integration-guide.md#multi-agent-workflow-patterns)** - Common usage patterns
- **⚙️ [Configuration](./integration-guide.md#agent-configuration-and-customization)** - Agent customization
- **🔍 [Monitoring](./integration-guide.md#health-monitoring-and-diagnostics)** - Health and performance
- **🛠️ [Troubleshooting](./integration-guide.md#troubleshooting)** - Common issues and solutions