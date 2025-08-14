//! MAGRAY CLI Human-Like Testing System - Main Executable
//! 
//! Запускает полное тестирование MAGRAY CLI с GPT-5 nano evaluation
//! 
//! Usage:
//!   cargo run --bin magray_testing
//!   cargo run --bin magray_testing -- --type complex_task
//!   cargo run --bin magray_testing -- --scenarios custom_scenarios.yaml

use std::path::Path;
use clap::{Parser, Subcommand};
use anyhow::Result;
use tokio;

// Импортируем наши модули
mod integration {
    pub mod human_like_testing;
}
mod scenarios {
    pub mod scenario_manager;
}
mod evaluators {
    pub mod gpt5_evaluator;
}
mod reports {
    pub mod test_report_generator;
}

use integration::human_like_testing::TestExecutor;
use scenarios::scenario_manager::ScenarioManager;
use evaluators::gpt5_evaluator::Gpt5Evaluator;
use reports::test_report_generator::TestReportGenerator;

/// MAGRAY CLI Human-Like Testing System
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Scenarios file to use (default: complex_scenarios.yaml)
    #[arg(short, long, default_value = "complex_scenarios.yaml")]
    scenarios: String,

    /// Output directory for reports (default: ./test_reports)
    #[arg(short, long, default_value = "./test_reports")]
    output: String,

    /// Only run scenarios of specific type
    #[arg(short = 't', long)]
    scenario_type: Option<String>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Skip GPT-5 evaluation (faster testing)
    #[arg(long)]
    skip_evaluation: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run full test suite
    Run {
        /// Custom scenarios file
        #[arg(short, long)]
        file: Option<String>,
    },
    /// Check system readiness
    Check,
    /// List available scenarios
    List,
    /// Run health check only
    Health,
    /// Generate sample scenarios file
    Sample,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    let cli = Cli::parse();

    // Настройка вербального вывода
    if cli.verbose {
        println!("🔧 Verbose mode enabled");
        println!("📁 Scenarios file: {}", cli.scenarios);
        println!("📂 Output directory: {}", cli.output);
    }

    match cli.command {
        Some(Commands::Run { file }) => {
            let scenarios_file = file.unwrap_or(cli.scenarios);
            run_test_suite(&scenarios_file, &cli.output, cli.scenario_type, cli.skip_evaluation, cli.verbose).await?;
        }
        Some(Commands::Check) => {
            check_system_readiness(&cli.output).await?;
        }
        Some(Commands::List) => {
            list_scenarios(&cli.scenarios).await?;
        }
        Some(Commands::Health) => {
            run_health_check().await?;
        }
        Some(Commands::Sample) => {
            generate_sample_scenarios().await?;
        }
        None => {
            // Default: run full test suite
            run_test_suite(&cli.scenarios, &cli.output, cli.scenario_type, cli.skip_evaluation, cli.verbose).await?;
        }
    }

    Ok(())
}

/// Запускает полный набор тестов
async fn run_test_suite(
    scenarios_file: &str,
    output_dir: &str,
    scenario_type: Option<String>,
    skip_evaluation: bool,
    verbose: bool,
) -> Result<()> {
    println!("🚀 MAGRAY CLI Human-Like Testing System");
    println!("{}", "=".repeat(60));

    // Проверяем наличие файла сценариев
    let scenarios_path = Path::new("./tests/scenarios").join(scenarios_file);
    if !scenarios_path.exists() {
        return Err(anyhow::anyhow!(
            "Scenarios file not found: {}. Use 'sample' command to generate one.",
            scenarios_path.display()
        ));
    }

    // 1. Создаем компоненты системы
    let mut executor = TestExecutor::with_project_root("../");
    let scenario_manager = ScenarioManager::new("./tests/scenarios");
    let report_generator = TestReportGenerator::new(output_dir);

    // 2. Full readiness check (включает компиляцию и health check)
    println!("🔍 Performing full system readiness check...");
    if !executor.full_readiness_check().await? {
        return Err(anyhow::anyhow!("❌ System readiness check failed"));
    }
    println!("✅ System is ready for testing");

    // 3. Загружаем сценарии
    println!("📋 Loading test scenarios...");
    let scenarios = match scenario_type {
        Some(ref test_type) => {
            println!("🎯 Filtering scenarios by type: {}", test_type);
            scenario_manager.load_scenarios_by_type(test_type)?
        }
        None => scenario_manager.load_scenarios_from_file(scenarios_file)?
    };

    if scenarios.is_empty() {
        return Err(anyhow::anyhow!("No scenarios loaded"));
    }

    scenario_manager.validate_scenarios(&scenarios)?;
    println!("✅ Loaded {} valid scenarios", scenarios.len());

    // 4. Выполняем тесты
    println!("🧪 Executing test scenarios...");
    let test_results = executor.execute_test_suite(scenarios.clone()).await?;
    
    let successful_tests = test_results.iter().filter(|r| r.success).count();
    println!("✅ Test execution completed: {}/{} successful", successful_tests, test_results.len());

    // 5. Оценка качества (опционально)
    let evaluation_results = if skip_evaluation {
        println!("⏭️  Skipping GPT-5 evaluation");
        Vec::new()
    } else {
        println!("🤖 Evaluating response quality with GPT-5 nano...");
        match Gpt5Evaluator::from_env_file("./.env") {
            Ok(evaluator) => {
                match evaluator.evaluate_test_batch(&scenarios, &test_results).await {
                    Ok(results) => {
                        println!("✅ Evaluation completed for {} scenarios", results.len());
                        results
                    }
                    Err(e) => {
                        println!("⚠️  Evaluation failed: {}", e);
                        println!("🔄 Continuing without evaluation...");
                        Vec::new()
                    }
                }
            }
            Err(e) => {
                println!("⚠️  Cannot initialize GPT-5 evaluator: {}", e);
                println!("💡 Hint: Make sure OPENAI_API_KEY is set in .env file");
                Vec::new()
            }
        }
    };

    // 6. Генерируем отчет
    println!("📊 Generating test report...");
    let session_id = format!("MAGRAY_TEST_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
    let report = report_generator.generate_full_report(
        &scenarios,
        &test_results,
        &evaluation_results,
        session_id,
    ).await?;

    // 7. Сохраняем отчет
    let saved_files = report_generator.save_report(&report).await?;

    // 8. Выводим сводку
    print_final_summary(&report, &saved_files, verbose);

    Ok(())
}

/// Проверяет готовность системы
async fn check_system_readiness(output_dir: &str) -> Result<()> {
    println!("🔧 System Readiness Check");
    println!("{}", "=".repeat(40));

    let mut all_good = true;

    // Проверка MAGRAY CLI
    print!("🔍 MAGRAY CLI availability... ");
    let mut executor = TestExecutor::with_project_root("../");
    match executor.full_readiness_check().await {
        Ok(true) => println!("✅ OK"),
        Ok(false) => {
            println!("❌ FAILED");
            all_good = false;
        }
        Err(e) => {
            println!("❌ ERROR: {}", e);
            all_good = false;
        }
    }

    // Проверка сценариев
    print!("📋 Test scenarios... ");
    let scenario_manager = ScenarioManager::new("./tests/scenarios");
    match scenario_manager.get_scenarios_stats() {
        Ok(stats) => {
            println!("✅ {} scenarios available", stats.total_count);
            if stats.total_count == 0 {
                println!("⚠️  No scenarios found - use 'sample' command to generate");
            }
        }
        Err(e) => {
            println!("❌ ERROR: {}", e);
            all_good = false;
        }
    }

    // Проверка API ключа
    print!("🔑 OpenAI API key... ");
    match Gpt5Evaluator::from_env_file("./.env") {
        Ok(_) => println!("✅ Found"),
        Err(_) => {
            println!("⚠️  Not found in .env file");
            println!("💡 Evaluation will be skipped without API key");
        }
    }

    // Проверка директории отчетов
    print!("📂 Reports directory... ");
    match std::fs::create_dir_all(output_dir) {
        Ok(_) => println!("✅ {} ready", output_dir),
        Err(e) => {
            println!("❌ ERROR: {}", e);
            all_good = false;
        }
    }

    println!();
    if all_good {
        println!("🎉 System is ready for testing!");
    } else {
        println!("❌ System has issues that need to be resolved");
        return Err(anyhow::anyhow!("System readiness check failed"));
    }

    Ok(())
}

/// Выводит список доступных сценариев
async fn list_scenarios(scenarios_file: &str) -> Result<()> {
    println!("📋 Available Test Scenarios");
    println!("{}", "=".repeat(50));

    let scenario_manager = ScenarioManager::new("./tests/scenarios");
    
    // Показываем статистику
    match scenario_manager.get_scenarios_stats() {
        Ok(stats) => {
            println!("📊 Overview:");
            println!("   • Total scenarios: {}", stats.total_count);
            println!("   • Estimated time: {} minutes", stats.estimated_time_minutes);
            println!("   • Categories: {}", stats.categories.join(", "));
            println!();

            println!("📈 By type:");
            for (test_type, count) in &stats.by_type {
                println!("   • {}: {} scenarios", test_type, count);
            }
            println!();
        }
        Err(e) => {
            println!("❌ Failed to load scenario stats: {}", e);
        }
    }

    // Показываем конкретные сценарии из указанного файла
    match scenario_manager.load_scenarios_from_file(scenarios_file) {
        Ok(scenarios) => {
            println!("📄 From {}:", scenarios_file);
            for (i, scenario) in scenarios.iter().enumerate() {
                println!("   {}. {} ({})", 
                       i + 1, 
                       scenario.name, 
                       scenario.expected_type);
                println!("      Input: {}", 
                       if scenario.input.len() > 60 {
                           format!("{}...", &scenario.input[..60])
                       } else {
                           scenario.input.clone()
                       });
                println!("      Timeout: {}s", scenario.timeout_seconds);
                println!();
            }
        }
        Err(e) => {
            println!("❌ Failed to load scenarios from {}: {}", scenarios_file, e);
            println!("💡 Use 'sample' command to generate a scenarios file");
        }
    }

    Ok(())
}

/// Запускает только health check
async fn run_health_check() -> Result<()> {
    println!("🔍 MAGRAY CLI Health Check");
    println!("{}", "=".repeat(30));

    let mut executor = TestExecutor::with_project_root("../");
    
    match executor.full_readiness_check().await {
        Ok(true) => {
            println!("✅ MAGRAY CLI is healthy and fully ready");
            println!("🎯 System is ready for comprehensive testing");
        }
        Ok(false) => {
            println!("❌ MAGRAY CLI readiness check failed");
            println!("💡 Trying basic health check...");
            
            // Fallback к базовому health check
            match executor.health_check().await {
                Ok(true) => {
                    println!("✅ Basic health check passed");
                    println!("⚠️  Some features may not be available");
                }
                Ok(false) => {
                    println!("❌ Even basic health check failed");
                    return Err(anyhow::anyhow!("Health check failed"));
                }
                Err(e) => {
                    println!("💥 Health check error: {}", e);
                    return Err(e);
                }
            }
        }
        Err(e) => {
            println!("💥 Readiness check error: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Генерирует пример файла сценариев
async fn generate_sample_scenarios() -> Result<()> {
    println!("📝 Generating sample scenarios file...");
    
    let scenarios_dir = Path::new("./tests/scenarios");
    std::fs::create_dir_all(scenarios_dir)?;
    
    let sample_file = scenarios_dir.join("sample_scenarios.yaml");
    
    if sample_file.exists() {
        println!("⚠️  File already exists: {}", sample_file.display());
        println!("💡 Use a different name or delete the existing file");
        return Ok(());
    }

    let sample_content = r#"# Sample MAGRAY CLI Test Scenarios
scenarios:
  - id: "quick_test"
    name: "Quick Response Test"
    input: "привет, как дела?"
    expected_type: "simple_response"
    timeout_seconds: 15
    evaluation_criteria:
      - "responds_appropriately"
      - "shows_politeness"

  - id: "help_test"  
    name: "Help Request Test"
    input: "помоги мне создать простое Rust приложение"
    expected_type: "complex_task"
    timeout_seconds: 60
    evaluation_criteria:
      - "provides_clear_instructions"
      - "includes_code_examples"
      - "explains_concepts"

  - id: "error_handling_test"
    name: "Error Handling Test"
    input: "абракадабра nonsense 12345"
    expected_type: "error_handling" 
    timeout_seconds: 20
    evaluation_criteria:
      - "handles_gracefully"
      - "provides_helpful_message"

meta:
  version: "1.0"
  description: "Sample scenarios for testing"
  total_scenarios: 3
  estimated_total_time_minutes: 5
  categories: ["basic", "help", "error"]
"#;

    std::fs::write(&sample_file, sample_content)?;
    println!("✅ Sample scenarios created: {}", sample_file.display());
    println!("🚀 You can now run: cargo run --bin magray_testing -- --scenarios sample_scenarios.yaml");

    Ok(())
}

/// Выводит финальную сводку результатов
fn print_final_summary(
    report: &reports::test_report_generator::TestSummaryReport,
    saved_files: &[String],
    verbose: bool,
) {
    println!();
    println!("🏁 TEST SUITE COMPLETED");
    println!("{}", "=".repeat(60));
    
    // Основные метрики
    println!("📊 EXECUTION RESULTS:");
    println!("   • Total Tests: {}", report.execution_summary.total_tests);
    println!("   • Success Rate: {:.1}%", report.execution_summary.success_rate);
    println!("   • Average Response Time: {:.0}ms", report.execution_summary.average_response_time_ms);
    
    // Качество (если доступно)
    if report.evaluation_summary.total_evaluations > 0 {
        println!("\n🎯 QUALITY EVALUATION:");
        println!("   • Overall Score: {:.1}/10", report.evaluation_summary.average_overall_score);
        println!("   • Technical Accuracy: {:.1}/10", report.evaluation_summary.average_scores.technical_accuracy);
        println!("   • Completeness: {:.1}/10", report.evaluation_summary.average_scores.completeness);
        
        // Распределение оценок
        println!("\n📈 SCORE DISTRIBUTION:");
        println!("   • Excellent (9-10): {} scenarios", report.evaluation_summary.score_distribution.excellent);
        println!("   • Good (7-8.9): {} scenarios", report.evaluation_summary.score_distribution.good);
        println!("   • Satisfactory (5-6.9): {} scenarios", report.evaluation_summary.score_distribution.satisfactory);
        println!("   • Poor (1-4.9): {} scenarios", report.evaluation_summary.score_distribution.poor);
    }

    // Ключевые рекомендации
    if !report.recommendations.is_empty() {
        println!("\n💡 KEY RECOMMENDATIONS:");
        for (i, rec) in report.recommendations.iter().take(3).enumerate() {
            println!("   {}. {}", i + 1, rec);
        }
        
        if report.recommendations.len() > 3 {
            println!("   ... and {} more in the detailed report", report.recommendations.len() - 3);
        }
    }

    // Сохраненные файлы
    println!("\n📄 REPORTS GENERATED:");
    for file in saved_files {
        let extension = Path::new(file).extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown");
        
        let icon = match extension {
            "html" => "🌐",
            "json" => "📋", 
            "md" => "📝",
            _ => "📄",
        };
        
        println!("   {} {}", icon, file);
    }

    // Детали в verbose режиме
    if verbose {
        println!("\n🔍 DETAILED BREAKDOWN:");
        println!("   • Test Duration: {:.1} minutes", report.metadata.test_duration_minutes);
        println!("   • MAGRAY Version: {}", report.metadata.magray_version);
        println!("   • Report ID: {}", report.metadata.report_id);
        
        if !report.execution_summary.tests_by_type.is_empty() {
            println!("\n📊 TESTS BY TYPE:");
            for (test_type, count) in &report.execution_summary.tests_by_type {
                println!("   • {}: {} tests", test_type, count);
            }
        }
    }

    println!("\n🎉 Testing completed successfully!");
    println!("💡 Open the HTML report for detailed analysis");
    println!("{}", "=".repeat(60));
}