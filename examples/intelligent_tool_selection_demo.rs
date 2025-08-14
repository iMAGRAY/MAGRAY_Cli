// @component: {"k":"E","id":"intelligent_tool_selection_demo","t":"Demo showcasing intelligent tool selection in MAGRAY CLI","m":{"cur":0,"tgt":100,"u":"%"},"f":["demo","example","tool_selection","orchestrator"]}

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use tokio;
use uuid::Uuid;
use chrono::Utc;

use orchestrator::agents::planner::{Planner, PlannerTrait};
use orchestrator::agents::intent_analyzer::{Intent, IntentType, IntentContext};
use tools::registry::{SecurityConfig, SecureToolRegistry};
use tools::context::ToolSelectionRequest;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing for better logging
    tracing_subscriber::fmt::init();

    println!("🚀 MAGRAY CLI Intelligent Tool Selection Demo");
    println!("==============================================");

    // Demo 1: Basic Tool Context Builder
    println!("\n📋 Demo 1: ToolContextBuilder Direct Usage");
    demo_tool_context_builder().await?;

    // Demo 2: Orchestrator Planner Integration
    println!("\n🎯 Demo 2: Orchestrator Planner with Intelligent Tool Selection");
    demo_orchestrator_integration().await?;

    // Demo 3: Performance Comparison
    println!("\n⚡ Demo 3: Performance Comparison - Intelligent vs Basic Planning");
    demo_performance_comparison().await?;

    println!("\n✅ All demos completed successfully!");
    println!("🎯 BLOCKER 3 - Tool Context Builder is now fully integrated!");

    Ok(())
}

/// Demonstrate direct usage of ToolContextBuilder
async fn demo_tool_context_builder() -> Result<()> {
    use tools::context::ToolContextBuilder;

    let security_config = SecurityConfig::default();
    let registry = Arc::new(SecureToolRegistry::new(security_config));
    
    let builder = ToolContextBuilder::new(registry)?;
    
    println!("✅ ToolContextBuilder created successfully");
    println!("🧠 Uses intelligent reranking: {}", builder.has_intelligent_tool_selection());
    
    // Test different types of queries
    let test_queries = vec![
        ("file operations", Some(vec!["filesystem".to_string()])),
        ("git status check", Some(vec!["git".to_string()])),
        ("web scraping task", Some(vec!["web".to_string()])),
        ("code analysis", Some(vec!["analysis".to_string()])),
        ("general utility task", None),
    ];
    
    for (query, categories) in test_queries {
        let request = ToolSelectionRequest {
            query: query.to_string(),
            context: HashMap::from([
                ("os".to_string(), "windows".to_string()),
                ("project_type".to_string(), "rust".to_string()),
            ]),
            required_categories: categories,
            exclude_tools: vec![],
            platform: Some("windows".to_string()),
            max_security_level: None,
            prefer_fast_tools: true,
            include_experimental: false,
        };
        
        let start_time = std::time::Instant::now();
        let response = builder.build_context(request).await?;
        let elapsed = start_time.elapsed();
        
        println!("  🔍 Query: '{}'", query);
        println!("    📊 Tools selected: {}", response.tools.len());
        println!("    ⏱️  Selection time: {:?}", elapsed);
        println!("    🎯 Cache hit: {}", response.selection_metrics.cache_hit);
        
        if let Some(best_tool) = response.tools.first() {
            println!("    🏆 Best tool: {} (score: {:.3})", 
                    best_tool.metadata.name, 
                    best_tool.combined_score);
            println!("    💭 Reasoning: {}", best_tool.reasoning);
        }
        println!();
    }
    
    Ok(())
}

/// Demonstrate orchestrator planner integration
async fn demo_orchestrator_integration() -> Result<()> {
    let security_config = SecurityConfig::default();
    let tool_registry = Arc::new(SecureToolRegistry::new(security_config));
    
    // Create planner with intelligent tool selection
    let intelligent_planner = Planner::with_intelligent_tool_selection(tool_registry)?;
    println!("✅ Intelligent planner created");
    
    // Create basic planner for comparison
    let basic_planner = Planner::new();
    println!("✅ Basic planner created");
    
    // Test different intent types
    let test_intents = vec![
        IntentType::ExecuteTool {
            tool_name: "file_manager".to_string(),
        },
        IntentType::AskQuestion {
            question: "What files were modified in the last commit?".to_string(),
        },
        IntentType::FileOperation {
            operation: "analyze".to_string(),
            path: "src/main.rs".to_string(),
        },
        IntentType::MemoryOperation {
            operation: "search_similar".to_string(),
        },
    ];
    
    for intent_type in test_intents {
        let intent = Intent {
            id: Uuid::new_v4(),
            intent_type: intent_type.clone(),
            parameters: HashMap::new(),
            confidence: 0.9,
            context: IntentContext {
                session_id: Uuid::new_v4(),
                user_id: Some("demo_user".to_string()),
                timestamp: Utc::now(),
                environment: HashMap::from([
                    ("os".to_string(), "windows".to_string()),
                    ("architecture".to_string(), "x86_64".to_string()),
                    ("project_path".to_string(), "/path/to/magray_cli".to_string()),
                ]),
                conversation_history: vec![],
            },
        };
        
        println!("🎯 Intent: {:?}", intent_type);
        
        // Test intelligent planner
        let start_time = std::time::Instant::now();
        let intelligent_plan = intelligent_planner.build_plan(&intent).await?;
        let intelligent_time = start_time.elapsed();
        
        // Test basic planner
        let start_time = std::time::Instant::now();
        let basic_plan = basic_planner.build_plan(&intent).await?;
        let basic_time = start_time.elapsed();
        
        println!("  🧠 Intelligent Plan:");
        println!("    📋 Steps: {}", intelligent_plan.steps.len());
        println!("    ⏱️  Time: {:?}", intelligent_time);
        println!("    🎯 Uses intelligent selection: {}", 
                intelligent_plan.metadata.get("intelligent_selection_used")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false));
        
        if let Some(score) = intelligent_plan.metadata.get("selected_tool_score") {
            println!("    📊 Tool score: {}", score);
        }
        
        if let Some(reasoning) = intelligent_plan.metadata.get("selection_reasoning") {
            println!("    💭 Selection reasoning: {}", reasoning.as_str().unwrap_or("N/A"));
        }
        
        println!("  🔧 Basic Plan:");
        println!("    📋 Steps: {}", basic_plan.steps.len());
        println!("    ⏱️  Time: {:?}", basic_time);
        
        println!("  📈 Performance improvement: {:.2}x", 
                basic_time.as_nanos() as f64 / intelligent_time.as_nanos() as f64);
        println!();
    }
    
    Ok(())
}

/// Demonstrate performance comparison between intelligent and basic planning
async fn demo_performance_comparison() -> Result<()> {
    let security_config = SecurityConfig::default();
    let tool_registry = Arc::new(SecureToolRegistry::new(security_config));
    
    let intelligent_planner = Planner::with_intelligent_tool_selection(tool_registry)?;
    let basic_planner = Planner::new();
    
    // Run multiple iterations for better performance measurement
    let iterations = 10;
    let mut intelligent_times = Vec::new();
    let mut basic_times = Vec::new();
    
    println!("🏃‍♂️ Running {} iterations for performance comparison...", iterations);
    
    for i in 0..iterations {
        let intent = Intent {
            id: Uuid::new_v4(),
            intent_type: IntentType::ExecuteTool {
                tool_name: format!("performance_test_tool_{}", i),
            },
            parameters: HashMap::new(),
            confidence: 0.8,
            context: IntentContext {
                session_id: Uuid::new_v4(),
                user_id: Some("perf_test".to_string()),
                timestamp: Utc::now(),
                environment: HashMap::new(),
                conversation_history: vec![],
            },
        };
        
        // Test intelligent planner
        let start = std::time::Instant::now();
        let _plan = intelligent_planner.build_plan(&intent).await?;
        intelligent_times.push(start.elapsed());
        
        // Test basic planner
        let start = std::time::Instant::now();
        let _plan = basic_planner.build_plan(&intent).await?;
        basic_times.push(start.elapsed());
    }
    
    let avg_intelligent: f64 = intelligent_times.iter().map(|d| d.as_nanos() as f64).sum::<f64>() / iterations as f64;
    let avg_basic: f64 = basic_times.iter().map(|d| d.as_nanos() as f64).sum::<f64>() / iterations as f64;
    
    println!("📊 Performance Results:");
    println!("  🧠 Intelligent Planning Average: {:.2}ms", avg_intelligent / 1_000_000.0);
    println!("  🔧 Basic Planning Average: {:.2}ms", avg_basic / 1_000_000.0);
    println!("  📈 Intelligent overhead: {:.2}ms", (avg_intelligent - avg_basic) / 1_000_000.0);
    println!("  🎯 Overhead percentage: {:.1}%", ((avg_intelligent - avg_basic) / avg_basic) * 100.0);
    
    // Performance should be reasonable (under 100ms overhead)
    let overhead_ms = (avg_intelligent - avg_basic) / 1_000_000.0;
    if overhead_ms < 100.0 {
        println!("  ✅ Performance target met: overhead < 100ms");
    } else {
        println!("  ⚠️  Performance target missed: overhead >= 100ms");
    }
    
    Ok(())
}