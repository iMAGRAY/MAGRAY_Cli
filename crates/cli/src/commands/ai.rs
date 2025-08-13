#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::useless_conversion)]
use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json;
use std::path::PathBuf;

use application::use_cases::ai_use_cases::{
    AiStatusRequest, AiStatusUseCase, InferenceRequest, InferenceUseCase, ListModelsRequest,
    ListModelsUseCase, LoadModelRequest, LoadModelUseCase,
};

#[derive(Debug, Parser)]
pub struct AiCommand {
    #[command(subcommand)]
    pub subcommand: AiSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum AiSubcommand {
    /// List available AI models
    Models {
        /// Show only models with specific type (embedding, reranking)
        #[arg(long)]
        model_type: Option<String>,

        /// Show detailed model information
        #[arg(long, short)]
        verbose: bool,

        /// Output format (table, json)
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Load AI model into memory
    Load {
        /// Model name to load
        model_name: String,

        /// Force reload if model already loaded
        #[arg(long, short)]
        force: bool,

        /// Prefer GPU if available
        #[arg(long)]
        gpu: bool,

        /// Model file path (if not in registry)
        #[arg(long)]
        path: Option<PathBuf>,
    },

    /// Run AI inference
    Inference {
        /// Model name to use for inference
        model: String,

        /// Input text or file path
        input: String,

        /// Input is a file path
        #[arg(long, short)]
        file: bool,

        /// Output format (json, text)
        #[arg(long, default_value = "text")]
        format: String,

        /// Batch size for processing
        #[arg(long, default_value = "1")]
        batch_size: usize,

        /// Top K results for reranking
        #[arg(long)]
        top_k: Option<usize>,
    },

    /// Show AI system status
    Status {
        /// Show detailed status
        #[arg(long, short)]
        verbose: bool,

        /// Output format (table, json)
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Unload model from memory
    Unload {
        /// Model name to unload
        model_name: String,

        /// Force unload even if in use
        #[arg(long, short)]
        force: bool,
    },

    /// Test model performance
    Benchmark {
        /// Model name to benchmark
        model: String,

        /// Number of test iterations
        #[arg(long, default_value = "10")]
        iterations: usize,

        /// Test input size
        #[arg(long, default_value = "512")]
        input_size: usize,

        /// Batch size for testing
        #[arg(long, default_value = "1")]
        batch_size: usize,

        /// Output detailed metrics
        #[arg(long, short)]
        verbose: bool,
    },
}

impl AiCommand {
    pub async fn execute(
        &self,
        list_models_use_case: &ListModelsUseCase,
        load_model_use_case: &LoadModelUseCase,
        inference_use_case: &InferenceUseCase,
        ai_status_use_case: &AiStatusUseCase,
    ) -> Result<()> {
        match &self.subcommand {
            AiSubcommand::Models {
                model_type,
                verbose,
                format,
            } => {
                self.handle_models(list_models_use_case, model_type, *verbose, format)
                    .await
            }

            AiSubcommand::Load {
                model_name,
                force,
                gpu,
                path,
            } => {
                self.handle_load(load_model_use_case, model_name, *force, *gpu, path)
                    .await
            }

            AiSubcommand::Inference {
                model,
                input,
                file,
                format,
                batch_size,
                top_k,
            } => {
                self.handle_inference(
                    inference_use_case,
                    model,
                    input,
                    *file,
                    format,
                    *batch_size,
                    *top_k,
                )
                .await
            }

            AiSubcommand::Status { verbose, format } => {
                self.handle_status(ai_status_use_case, *verbose, format)
                    .await
            }

            AiSubcommand::Unload { model_name, force } => {
                self.handle_unload(load_model_use_case, model_name, *force)
                    .await
            }

            AiSubcommand::Benchmark {
                model,
                iterations,
                input_size,
                batch_size,
                verbose,
            } => {
                self.handle_benchmark(
                    inference_use_case,
                    model,
                    *iterations,
                    *input_size,
                    *batch_size,
                    *verbose,
                )
                .await
            }
        }
    }

    async fn handle_models(
        &self,
        use_case: &ListModelsUseCase,
        model_type: &Option<String>,
        verbose: bool,
        format: &str,
    ) -> Result<()> {
        let request = ListModelsRequest {
            model_type: model_type.clone(),
            include_loaded: true,
            include_available: true,
        };

        match use_case.execute(request).await {
            Ok(response) => {
                match format {
                    "json" => {
                        println!("{}", serde_json::to_string_pretty(&response.models)?);
                    }
                    _ => {
                        println!("\n📋 Available AI Models:");
                        println!("┌─────────────────────────────────────────────────────────────────────┐");

                        for model in &response.models {
                            let status = if model.loaded {
                                "✅ Loaded"
                            } else {
                                "💤 Available"
                            };
                            let device = model.device.as_deref().unwrap_or("N/A");
                            let size = model
                                .size_mb
                                .map_or("Unknown".to_string(), |s| format!("{s:.1}MB"));

                            if verbose {
                                println!(
                                    "│ 🤖 {:<25} │ {:<12} │ {:<8} │ {:<10} │",
                                    model.name, status, device, size
                                );
                                if let Some(desc) = &model.description {
                                    println!("│    Description: {desc:<48} │");
                                }
                                if let Some(path) = &model.path {
                                    println!("│    Path: {:<55} │", path.display());
                                }
                                println!("├─────────────────────────────────────────────────────────────────────┤");
                            } else {
                                println!(
                                    "│ 🤖 {:<25} │ {:<12} │ {:<8} │ {:<10} │",
                                    model.name, status, device, size
                                );
                            }
                        }

                        println!("└─────────────────────────────────────────────────────────────────────┘");
                        println!(
                            "\n📊 Summary: {} models ({} loaded, {} available)",
                            response.models.len(),
                            response.models.iter().filter(|m| m.loaded).count(),
                            response.models.iter().filter(|m| !m.loaded).count()
                        );
                    }
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("❌ Error listing models: {e}");
                Err(e.into())
            }
        }
    }

    async fn handle_load(
        &self,
        use_case: &LoadModelUseCase,
        model_name: &str,
        force: bool,
        prefer_gpu: bool,
        path: &Option<PathBuf>,
    ) -> Result<()> {
        println!("🔄 Loading model '{model_name}'...");

        let request = LoadModelRequest {
            model_name: model_name.to_string(),
            force_reload: force,
            prefer_gpu,
            custom_path: path.clone(),
            device_id: None,
        };

        match use_case.execute(request).await {
            Ok(response) => {
                println!("✅ Model '{model_name}' loaded successfully!");
                println!("   📊 Device: {}", response.device);
                println!("   💾 Memory: {:.1}MB", response.memory_usage_mb);
                println!(
                    "   ⏱️  Load time: {:.2}s",
                    response.load_time_ms as f64 / 1000.0
                );

                if let Some(capabilities) = response.capabilities {
                    println!("   🎯 Capabilities: {}", capabilities.join(", "));
                }

                Ok(())
            }
            Err(e) => {
                eprintln!("❌ Error loading model '{model_name}': {e}");
                Err(e.into())
            }
        }
    }

    async fn handle_inference(
        &self,
        use_case: &InferenceUseCase,
        model: &str,
        input: &str,
        is_file: bool,
        format: &str,
        batch_size: usize,
        top_k: Option<usize>,
    ) -> Result<()> {
        let input_text = if is_file {
            std::fs::read_to_string(input)?
        } else {
            input.to_string()
        };

        println!("🔄 Running inference with model '{model}'...");

        let request = InferenceRequest {
            model_name: model.to_string(),
            input: input_text,
            batch_size,
            top_k,
            temperature: None,
            max_tokens: None,
        };

        match use_case.execute(request).await {
            Ok(response) => {
                match format {
                    "json" => {
                        println!("{}", serde_json::to_string_pretty(&response)?);
                    }
                    _ => {
                        println!("\n🎯 Inference Results:");
                        println!("┌─────────────────────────────────────────────────────────────────────┐");

                        match &response.result_type[..] {
                            "embedding" => {
                                if let Some(embedding) = &response.embedding {
                                    println!("│ 🔢 Embedding Vector (dimension: {})                              │", embedding.len());
                                    println!("│ First 10 values: {:.6?}...                                       │", &embedding[..10.min(embedding.len())]);
                                }
                            }
                            "reranking" => {
                                if let Some(scores) = &response.scores {
                                    println!("│ 📊 Reranking Scores:                                               │");
                                    for (i, score) in scores.iter().enumerate() {
                                        println!("│   #{}: {:.6}                                                      │", i + 1, score);
                                    }
                                }
                            }
                            "text" => {
                                if let Some(text) = &response.text_result {
                                    println!("│ 📝 Text Result:                                                    │");
                                    println!("│ {text}                                                                │");
                                }
                            }
                            _ => {
                                println!("│ ✅ Inference completed successfully                                  │");
                            }
                        }

                        println!("└─────────────────────────────────────────────────────────────────────┘");
                        println!(
                            "\n⏱️  Processing time: {:.2}ms",
                            response.processing_time_ms
                        );
                        println!("🔧 Model: {}", response.model_name);
                        if let Some(device) = &response.device {
                            println!("💻 Device: {device}");
                        }
                    }
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("❌ Error running inference: {e}");
                Err(e.into())
            }
        }
    }

    async fn handle_status(
        &self,
        use_case: &AiStatusUseCase,
        verbose: bool,
        format: &str,
    ) -> Result<()> {
        let request = AiStatusRequest {
            include_models: true,
            include_system_info: verbose,
            include_performance_stats: verbose,
        };

        match use_case.execute(request).await {
            Ok(response) => {
                match format {
                    "json" => {
                        println!("{}", serde_json::to_string_pretty(&response)?);
                    }
                    _ => {
                        println!("\n🔧 AI System Status:");
                        println!("┌─────────────────────────────────────────────────────────────────────┐");
                        println!(
                            "│ Status: {} {:<56} │",
                            if response.system_healthy {
                                "✅ Healthy"
                            } else {
                                "❌ Issues"
                            },
                            format!("({} loaded models)", response.loaded_models_count)
                        );

                        if let Some(memory) = response.total_memory_usage_mb {
                            println!("│ Memory Usage: {memory:.1}MB                                                │");
                        }

                        if verbose {
                            if let Some(gpu_available) = response.gpu_available {
                                println!(
                                    "│ GPU Available: {:<52} │",
                                    if gpu_available { "✅ Yes" } else { "❌ No" }
                                );
                            }

                            if let Some(onnx_version) = &response.onnx_version {
                                println!("│ ONNX Runtime: {onnx_version:<52} │");
                            }
                        }

                        println!("└─────────────────────────────────────────────────────────────────────┘");

                        if !response.loaded_models.is_empty() {
                            println!("\n🤖 Loaded Models:");
                            for model in &response.loaded_models {
                                println!(
                                    "  • {} ({})",
                                    model.name,
                                    model.device.as_deref().unwrap_or("CPU")
                                );
                            }
                        }

                        if let Some(warnings) = &response.warnings {
                            if !warnings.is_empty() {
                                println!("\n⚠️  Warnings:");
                                for warning in warnings {
                                    println!("  • {warning}");
                                }
                            }
                        }
                    }
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("❌ Error getting AI status: {e}");
                Err(e.into())
            }
        }
    }

    async fn handle_unload(
        &self,
        use_case: &LoadModelUseCase,
        model_name: &str,
        force: bool,
    ) -> Result<()> {
        println!("🔄 Unloading model '{model_name}'...");

        // Note: This would need an UnloadModelUseCase, using LoadModelUseCase as placeholder
        println!("⚠️  Model unloading functionality not yet implemented");
        println!("💡 Use 'magray ai status' to see loaded models");

        Ok(())
    }

    async fn handle_benchmark(
        &self,
        use_case: &InferenceUseCase,
        model: &str,
        iterations: usize,
        input_size: usize,
        batch_size: usize,
        verbose: bool,
    ) -> Result<()> {
        println!(
            "🔄 Benchmarking model '{model}' ({iterations} iterations, input size: {input_size}, batch size: {batch_size})..."
        );

        // Generate test input
        let test_input = "test ".repeat(input_size / 5); // Approximate input size

        let mut total_time = 0.0;
        let mut successful_runs = 0;
        let mut errors = Vec::new();

        for i in 0..iterations {
            if verbose {
                print!("  Run {}/{}: ", i + 1, iterations);
                std::io::Write::flush(&mut std::io::stdout())
                    .expect("Operation failed - converted from unwrap()");
            }

            let request = InferenceRequest {
                model_name: model.to_string(),
                input: test_input.clone(),
                batch_size,
                top_k: None,
                temperature: None,
                max_tokens: None,
            };

            let start_time = std::time::Instant::now();
            match use_case.execute(request).await {
                Ok(_) => {
                    let elapsed = start_time.elapsed().as_millis() as f64;
                    total_time += elapsed;
                    successful_runs += 1;

                    if verbose {
                        println!("✅ {elapsed:.2}ms");
                    }
                }
                Err(e) => {
                    errors.push(format!("Run {}: {}", i + 1, e));
                    if verbose {
                        println!("❌ Error");
                    }
                }
            }
        }

        println!("\n📊 Benchmark Results:");
        println!("┌─────────────────────────────────────────────────────────────────────┐");
        println!("│ Model: {model:<60} │");
        println!("│ Successful runs: {successful_runs}/{iterations:<48} │");

        if successful_runs > 0 {
            let avg_time = total_time / successful_runs as f64;
            let throughput = 1000.0 / avg_time; // requests per second

            println!(
                "│ Average time: {avg_time:.2}ms                                             │"
            );
            println!(
                "│ Throughput: {throughput:.2} req/s                                           │"
            );
        }

        println!("└─────────────────────────────────────────────────────────────────────┘");

        if !errors.is_empty() && verbose {
            println!("\n❌ Errors:");
            for error in &errors {
                println!("  • {error}");
            }
        }

        Ok(())
    }
}

// Feature compatibility - only compile if AI features are available
#[cfg(not(any(feature = "cpu", feature = "gpu")))]
pub struct AiCommand;

#[cfg(not(any(feature = "cpu", feature = "gpu")))]
impl AiCommand {
    pub async fn execute(&self) -> Result<()> {
        eprintln!("❌ AI functionality not available in this build");
        eprintln!("💡 Build with --features cpu or --features gpu to enable AI commands");
        Ok(())
    }
}
