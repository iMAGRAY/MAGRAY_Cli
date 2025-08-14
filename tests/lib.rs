//! MAGRAY CLI Human-Like Testing System
//! 
//! Полнофункциональная система автотестирования для MAGRAY CLI с human-like interaction testing
//! и GPT-5 nano evaluation. Система симулирует реального пользователя, запускает CLI как subprocess,
//! отправляет сложные задачи и оценивает качество ответов через OpenAI API.
//!
//! ## Основные компоненты:
//!
//! - **TestExecutor** - управление subprocess для CLI и human-like interaction
//! - **ScenarioManager** - загрузка тестовых сценариев из YAML файлов
//! - **Gpt5Evaluator** - оценка качества ответов через GPT-5 nano API
//! - **TestReportGenerator** - создание детальных отчетов в HTML/JSON/Markdown
//!
//! ## Использование:
//!
//! ```rust
//! use magray_testing::{TestExecutor, ScenarioManager, Gpt5Evaluator, TestReportGenerator};
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // 1. Загружаем сценарии
//!     let scenario_manager = ScenarioManager::new("./tests/scenarios");
//!     let scenarios = scenario_manager.load_scenarios_from_file("complex_scenarios.yaml")?;
//!     
//!     // 2. Выполняем тесты
//!     let executor = TestExecutor::new();
//!     let test_results = executor.execute_test_suite(scenarios.clone()).await?;
//!     
//!     // 3. Оцениваем качество через GPT-5
//!     let evaluator = Gpt5Evaluator::from_env()?;
//!     let evaluations = evaluator.evaluate_test_batch(&scenarios, &test_results).await?;
//!     
//!     // 4. Генерируем отчет
//!     let report_generator = TestReportGenerator::new("./test_reports");
//!     let report = report_generator.generate_full_report(
//!         &scenarios, &test_results, &evaluations, "test_session_001".to_string()
//!     ).await?;
//!     
//!     // 5. Сохраняем отчет
//!     let saved_files = report_generator.save_report(&report).await?;
//!     println!("Reports saved: {:?}", saved_files);
//!     
//!     Ok(())
//! }
//! ```

pub mod integration {
    pub mod human_like_testing;
}

pub mod scenarios {
    pub mod scenario_manager;
}

pub mod evaluators {
    pub mod gpt5_evaluator;
}

pub mod reports {
    pub mod test_report_generator;
}

// Re-export main types for convenient access
pub use integration::human_like_testing::{TestExecutor, TestResult, TestScenario};
pub use scenarios::scenario_manager::{ScenarioManager, ScenarioStats};
pub use evaluators::gpt5_evaluator::{Gpt5Evaluator, EvaluationResult, EvaluationScores};
pub use reports::test_report_generator::{TestReportGenerator, TestSummaryReport};

/// Основной API для выполнения полного цикла тестирования
pub struct MagrayTestSuite {
    executor: TestExecutor,
    scenario_manager: ScenarioManager,
    evaluator: Gpt5Evaluator,
    report_generator: TestReportGenerator,
}

impl MagrayTestSuite {
    /// Создает новый набор тестов с настройками по умолчанию
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            executor: TestExecutor::new(),
            scenario_manager: ScenarioManager::new("./tests/scenarios"),
            evaluator: Gpt5Evaluator::from_env()?,
            report_generator: TestReportGenerator::new("./test_reports"),
        })
    }

    /// Создает набор тестов с пользовательскими путями
    pub fn with_paths(
        scenarios_dir: &str,
        reports_dir: &str,
        api_key: Option<String>,
    ) -> anyhow::Result<Self> {
        let evaluator = match api_key {
            Some(key) => Gpt5Evaluator::new(key),
            None => Gpt5Evaluator::from_env()?,
        };

        Ok(Self {
            executor: TestExecutor::new(),
            scenario_manager: ScenarioManager::new(scenarios_dir),
            evaluator,
            report_generator: TestReportGenerator::new(reports_dir),
        })
    }

    /// Выполняет полный цикл тестирования: загрузка сценариев → выполнение → оценка → отчет
    pub async fn run_full_test_suite(&self, scenarios_file: &str) -> anyhow::Result<Vec<String>> {
        println!("🚀 Starting MAGRAY CLI Human-Like Testing Suite");

        // 1. Health check
        println!("🔍 Performing CLI health check...");
        if !self.executor.health_check().await? {
            return Err(anyhow::anyhow!("CLI health check failed - MAGRAY CLI is not responding"));
        }

        // 2. Загружаем сценарии
        println!("📋 Loading test scenarios from: {}", scenarios_file);
        let scenarios = self.scenario_manager.load_scenarios_from_file(scenarios_file)?;
        self.scenario_manager.validate_scenarios(&scenarios)?;

        // 3. Выполняем тесты
        println!("🧪 Executing {} test scenarios...", scenarios.len());
        let test_results = self.executor.execute_test_suite(scenarios.clone()).await?;

        // 4. Оцениваем качество
        println!("🤖 Evaluating response quality with GPT-5 nano...");
        let evaluation_results = self.evaluator.evaluate_test_batch(&scenarios, &test_results).await?;

        // 5. Генерируем отчет
        println!("📊 Generating comprehensive test report...");
        let session_id = format!("MAGRAY_TEST_{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
        let report = self.report_generator.generate_full_report(
            &scenarios,
            &test_results,
            &evaluation_results,
            session_id,
        ).await?;

        // 6. Сохраняем отчет
        println!("💾 Saving test report...");
        let saved_files = self.report_generator.save_report(&report).await?;

        // 7. Выводим сводку
        self.print_test_summary(&report);

        Ok(saved_files)
    }

    /// Выполняет тесты только определенного типа
    pub async fn run_targeted_tests(&self, test_type: &str) -> anyhow::Result<Vec<String>> {
        println!("🎯 Running targeted tests for type: {}", test_type);

        let scenarios = self.scenario_manager.load_scenarios_by_type(test_type)?;
        if scenarios.is_empty() {
            return Err(anyhow::anyhow!("No scenarios found for type: {}", test_type));
        }

        let test_results = self.executor.execute_test_suite(scenarios.clone()).await?;
        let evaluation_results = self.evaluator.evaluate_test_batch(&scenarios, &test_results).await?;

        let session_id = format!("MAGRAY_TARGETED_{}_{}", test_type.to_uppercase(), chrono::Utc::now().format("%Y%m%d_%H%M%S"));
        let report = self.report_generator.generate_full_report(
            &scenarios,
            &test_results,
            &evaluation_results,
            session_id,
        ).await?;

        let saved_files = self.report_generator.save_report(&report).await?;
        self.print_test_summary(&report);

        Ok(saved_files)
    }

    /// Выводит краткую сводку результатов тестирования
    fn print_test_summary(&self, report: &TestSummaryReport) {
        println!("\n🏁 TEST SUITE COMPLETED");
        println!("=" .repeat(50));
        println!("📊 EXECUTION SUMMARY:");
        println!("   • Total Tests: {}", report.execution_summary.total_tests);
        println!("   • Success Rate: {:.1}%", report.execution_summary.success_rate);
        println!("   • Avg Response Time: {:.0}ms", report.execution_summary.average_response_time_ms);
        
        println!("\n🎯 QUALITY EVALUATION:");
        println!("   • Overall Score: {:.1}/10", report.evaluation_summary.average_overall_score);
        println!("   • Technical Accuracy: {:.1}/10", report.evaluation_summary.average_scores.technical_accuracy);
        println!("   • Completeness: {:.1}/10", report.evaluation_summary.average_scores.completeness);
        
        println!("\n📈 SCORE DISTRIBUTION:");
        println!("   • Excellent (9-10): {} scenarios", report.evaluation_summary.score_distribution.excellent);
        println!("   • Good (7-8.9): {} scenarios", report.evaluation_summary.score_distribution.good);
        println!("   • Satisfactory (5-6.9): {} scenarios", report.evaluation_summary.score_distribution.satisfactory);
        println!("   • Poor (1-4.9): {} scenarios", report.evaluation_summary.score_distribution.poor);

        println!("\n💡 KEY RECOMMENDATIONS:");
        for (i, rec) in report.recommendations.iter().take(3).enumerate() {
            println!("   {}. {}", i + 1, rec);
        }

        println!("\n📄 Report ID: {}", report.metadata.report_id);
        println!("=" .repeat(50));
    }

    /// Получает статистику доступных сценариев
    pub fn get_scenario_stats(&self) -> anyhow::Result<ScenarioStats> {
        self.scenario_manager.get_scenarios_stats()
    }

    /// Проверяет готовность системы к тестированию
    pub async fn verify_system_readiness(&self) -> anyhow::Result<bool> {
        println!("🔧 Verifying system readiness...");

        // Проверка CLI
        if !self.executor.health_check().await? {
            println!("❌ CLI health check failed");
            return Ok(false);
        }

        // Проверка сценариев
        match self.scenario_manager.load_all_scenarios() {
            Ok(scenarios) => {
                if scenarios.is_empty() {
                    println!("❌ No test scenarios found");
                    return Ok(false);
                }
                println!("✅ Found {} test scenarios", scenarios.len());
            }
            Err(e) => {
                println!("❌ Failed to load scenarios: {}", e);
                return Ok(false);
            }
        }

        // Проверка API ключа (не делаем реальный запрос)
        println!("✅ GPT-5 evaluator configured");

        // Проверка директории отчетов
        std::fs::create_dir_all(&self.report_generator.output_dir).map_err(|e| {
            println!("❌ Cannot create reports directory: {}", e);
            e
        })?;
        println!("✅ Reports directory ready");

        println!("🎉 System is ready for testing!");
        Ok(true)
    }
}

impl Default for MagrayTestSuite {
    fn default() -> Self {
        Self::new().expect("Failed to create default MagrayTestSuite")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_magray_test_suite_creation() {
        // This test requires environment setup, so we'll skip it in normal runs
        if std::env::var("OPENAI_API_KEY").is_ok() {
            let suite = MagrayTestSuite::new();
            assert!(suite.is_ok());
        }
    }

    #[test]
    fn test_module_exports() {
        // Test that all main types are properly exported
        use crate::{TestExecutor, ScenarioManager, Gpt5Evaluator, TestReportGenerator};
        
        let _executor = TestExecutor::new();
        let _manager = ScenarioManager::new("./test");
        // GPT5Evaluator and TestReportGenerator require additional setup
    }
}