# IntentAnalyzer API Documentation

## Overview

The `IntentAnalyzer` agent is responsible for analyzing user input and extracting structured intentions that can be processed by the planning and execution pipeline.

## API Contract

### IntentAnalyzerTrait

```rust
#[async_trait]
pub trait IntentAnalyzerTrait: Send + Sync {
    /// Analyze user input and extract structured intent
    async fn analyze_intent(&self, input: &str, context: &IntentContext) -> Result<Intent>;
    
    /// Update confidence based on execution results
    async fn update_confidence(&mut self, intent_id: Uuid, success: bool) -> Result<()>;
    
    /// Get intent analysis statistics
    async fn get_statistics(&self) -> Result<IntentAnalysisStats>;
}
```

## Data Structures

### Intent

Represents a structured user intention:

```rust
pub struct Intent {
    pub id: Uuid,                    // Unique intent identifier
    pub intent_type: IntentType,     // Classified intent type
    pub parameters: HashMap<String, serde_json::Value>, // Extracted parameters
    pub confidence: f64,             // Confidence score (0.0-1.0)
    pub context: IntentContext,      // Analysis context
}
```

### IntentType

Supported intent classifications:

```rust
pub enum IntentType {
    /// Execute a specific tool or command
    ExecuteTool { tool_name: String },
    
    /// Ask a question or request information
    AskQuestion { question: String },
    
    /// Perform file operations
    FileOperation { operation: String, path: String },
    
    /// Memory operations (search, store, analyze)
    MemoryOperation { operation: String },
    
    /// Complex workflow or recipe execution
    WorkflowExecution { workflow_name: String },
    
    /// System management commands
    SystemCommand { command: String },
    
    /// Unknown intent requiring further analysis
    Unknown { raw_input: String },
}
```

### IntentContext

Context information for intent analysis:

```rust
pub struct IntentContext {
    pub session_id: Uuid,                           // Session identifier
    pub user_id: Option<String>,                    // Optional user ID
    pub timestamp: chrono::DateTime<chrono::Utc>,   // Analysis timestamp
    pub environment: HashMap<String, String>,       // Environment variables
    pub conversation_history: Vec<String>,          // Previous interactions
}
```

## Usage Examples

### Basic Intent Analysis

```rust
use orchestrator::agents::{IntentAnalyzer, IntentAnalyzerTrait, IntentContext};
use uuid::Uuid;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    // Create intent analyzer
    let analyzer = IntentAnalyzer::new();
    
    // Create analysis context
    let context = IntentContext {
        session_id: Uuid::new_v4(),
        user_id: Some("user123".to_string()),
        timestamp: chrono::Utc::now(),
        environment: HashMap::new(),
        conversation_history: vec![],
    };
    
    // Analyze user input
    let intent = analyzer.analyze_intent("list all files in the documents folder", &context).await?;
    
    println!("Intent: {:?}", intent.intent_type);
    println!("Confidence: {:.2}", intent.confidence);
    
    Ok(())
}
```

### Enhanced Analysis with LLM Provider

```rust
use llm::AnthropicProvider;

#[tokio::main]
async fn main() -> Result<()> {
    // Create LLM provider
    let llm_provider = Box::new(AnthropicProvider::new("api_key"));
    
    // Create analyzer with LLM enhancement
    let analyzer = IntentAnalyzer::new()
        .with_llm_provider(llm_provider)
        .with_confidence_threshold(0.8);
    
    let context = IntentContext {
        session_id: Uuid::new_v4(),
        user_id: Some("user123".to_string()),
        timestamp: chrono::Utc::now(),
        environment: HashMap::new(),
        conversation_history: vec![
            "I need to analyze some data".to_string(),
            "Show me the memory usage".to_string(),
        ],
    };
    
    // Complex intent analysis
    let intent = analyzer.analyze_intent(
        "Based on the memory analysis, can you optimize the data processing pipeline?", 
        &context
    ).await?;
    
    match intent.intent_type {
        IntentType::WorkflowExecution { workflow_name } => {
            println!("Detected workflow: {}", workflow_name);
        }
        IntentType::MemoryOperation { operation } => {
            println!("Memory operation: {}", operation);
        }
        _ => println!("Other intent type: {:?}", intent.intent_type),
    }
    
    Ok(())
}
```

### Updating Confidence Based on Results

```rust
#[tokio::main]
async fn main() -> Result<()> {
    let mut analyzer = IntentAnalyzer::new();
    
    // Analyze intent
    let intent = analyzer.analyze_intent("backup my project", &context).await?;
    let intent_id = intent.id;
    
    // ... execute the intent ...
    
    // Update confidence based on execution success
    analyzer.update_confidence(intent_id, true).await?;
    
    // Get updated statistics
    let stats = analyzer.get_statistics().await?;
    println!("Success rate: {:.2}%", 
        (stats.successful_predictions as f64 / stats.total_analyzed as f64) * 100.0);
    
    Ok(())
}
```

## Configuration

### Builder Pattern

```rust
let analyzer = IntentAnalyzer::new()
    .with_confidence_threshold(0.75)           // Minimum confidence threshold
    .with_llm_provider(llm_provider)           // Enhanced LLM analysis
    .with_context_window_size(10)              // Conversation history size
    .with_intent_cache_size(1000);             // Intent cache size
```

### Intent Type Patterns

The analyzer recognizes various input patterns:

| Pattern | Intent Type | Example |
|---------|-------------|---------|
| `"run <tool>"` | ExecuteTool | "run cargo build" |
| `"what is <query>?"` | AskQuestion | "what is the current memory usage?" |
| `"copy <path1> to <path2>"` | FileOperation | "copy src/ to backup/" |
| `"search for <query>"` | MemoryOperation | "search for rust patterns" |
| `"execute <workflow>"` | WorkflowExecution | "execute deployment workflow" |

## Integration with EventBus

```rust
use common::event_bus::GLOBAL_EVENT_BUS;

#[tokio::main]
async fn main() -> Result<()> {
    let analyzer = IntentAnalyzer::new();
    
    // Subscribe to intent events
    GLOBAL_EVENT_BUS.subscribe("agent.intent.analyzed", |event| {
        println!("Intent analyzed: {:?}", event);
    }).await;
    
    // The analyzer automatically publishes events when intents are analyzed
    let intent = analyzer.analyze_intent("help me debug this error", &context).await?;
    
    Ok(())
}
```

## Error Handling

The IntentAnalyzer provides detailed error information:

```rust
match analyzer.analyze_intent(input, &context).await {
    Ok(intent) => {
        if intent.confidence < 0.5 {
            println!("Low confidence intent: {:?}", intent.intent_type);
        }
    }
    Err(e) => {
        eprintln!("Intent analysis failed: {}", e);
        // Fallback to unknown intent type
    }
}
```

## Performance Considerations

- **LLM Calls**: Enable LLM fallback only for complex intents to reduce latency
- **Context Size**: Limit conversation history to prevent memory growth
- **Caching**: Intent patterns are cached for improved performance
- **Batch Processing**: Multiple intents can be analyzed in parallel

## Monitoring and Statistics

```rust
let stats = analyzer.get_statistics().await?;

println!("Intent Analysis Statistics:");
println!("Total analyzed: {}", stats.total_analyzed);
println!("Success rate: {:.2}%", 
    (stats.successful_predictions as f64 / stats.total_analyzed as f64) * 100.0);
println!("Average confidence: {:.2}", stats.average_confidence);

for (intent_type, count) in &stats.intent_type_distribution {
    println!("{}: {} occurrences", intent_type, count);
}
```