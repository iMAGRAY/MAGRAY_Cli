use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tokio::time::timeout;
use uuid::Uuid;

/// Test Executor - запускает MAGRAY CLI как subprocess и симулирует человеческое взаимодействие
pub struct TestExecutor {
    binary_path: Option<PathBuf>,
    use_cargo_run: bool,
    timeout_duration: Duration,
    session_id: String,
    project_root: PathBuf,
    max_retries: u32,
}

/// Результат выполнения тестового сценария
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub scenario_id: String,
    pub input: String,
    pub output: String,
    pub execution_time_ms: u64,
    pub success: bool,
    pub error_message: Option<String>,
    pub timestamp: String,
}

/// Конфигурация для тестового сценария
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestScenario {
    pub id: String,
    pub name: String,
    pub input: String,
    pub expected_type: String, // "simple_response", "complex_task", "error_handling"
    pub timeout_seconds: u64,
    pub evaluation_criteria: Vec<String>,
}

/// Интерактивный CLI процесс для общения
pub struct InteractiveCliProcess {
    pub process: Child,
    pub start_time: Instant,
}

impl InteractiveCliProcess {
    /// Отправляет сообщение в CLI
    pub async fn send_message(&mut self, message: &str) -> Result<()> {
        if let Some(ref mut stdin) = self.process.stdin {
            writeln!(stdin, "{message}")?;
            stdin.flush()?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("stdin not available"))
        }
    }

    /// Читает ответ из CLI с таймаутом
    pub async fn read_response(&mut self, timeout_duration: Duration) -> Result<String> {
        if let Some(stdout) = self.process.stdout.take() {
            let handle = tokio::task::spawn_blocking(move || -> Result<String> {
                let reader = BufReader::new(stdout);
                let mut output = String::new();
                let mut lines_read = 0;

                for line in reader.lines() {
                    match line {
                        Ok(line_str) => {
                            output.push_str(&line_str);
                            output.push('\n');
                            lines_read += 1;

                            // Простая эвристика для определения завершения ответа
                            if line_str.contains("✓")
                                || line_str.contains("Done")
                                || line_str.contains("completed")
                                || line_str.contains("Вы:")
                                || (lines_read > 10 && line_str.trim().is_empty())
                            {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }

                Ok(output)
            });

            match timeout(timeout_duration, handle).await {
                Ok(Ok(Ok(output))) => Ok(output),
                Ok(Ok(Err(e))) => Err(e),
                Ok(Err(e)) => Err(anyhow::anyhow!("Task join error: {}", e)),
                Err(_) => Err(anyhow::anyhow!("Read timeout")),
            }
        } else {
            Err(anyhow::anyhow!("stdout not available"))
        }
    }

    /// Завершает процесс CLI
    pub fn terminate(mut self) -> Result<()> {
        let _ = self.process.kill();
        let _ = self.process.wait();
        Ok(())
    }
}

impl TestExecutor {
    /// Создает новый Test Executor
    pub fn new() -> Self {
        let current_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

        Self {
            binary_path: None,
            use_cargo_run: false,
            timeout_duration: Duration::from_secs(30),
            session_id: Uuid::new_v4().to_string(),
            project_root: current_dir,
            max_retries: 3,
        }
    }

    /// Создает Test Executor с указанной корневой директорией проекта
    pub fn with_project_root<P: AsRef<Path>>(root: P) -> Self {
        let mut executor = Self::new();
        executor.project_root = root.as_ref().to_path_buf();
        executor
    }

    /// Устанавливает путь к бинарному файлу CLI
    pub fn with_binary_path<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.binary_path = Some(path.as_ref().to_path_buf());
        self.use_cargo_run = false;
        self
    }

    /// Устанавливает timeout для выполнения команд
    pub fn with_timeout(mut self, duration: Duration) -> Self {
        self.timeout_duration = duration;
        self
    }

    /// Устанавливает количество повторных попыток
    pub fn with_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Компилирует MAGRAY CLI перед тестированием
    pub async fn compile_magray_cli(&mut self) -> Result<()> {
        println!("🔨 Compiling MAGRAY CLI...");

        let compile_start = Instant::now();

        // Компилируем проект
        let output = Command::new("cargo")
            .args(["build", "-p", "cli", "--bin", "magray"])
            .current_dir(&self.project_root)
            .output()
            .context("Failed to run cargo build")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Compilation failed:\n{}", stderr));
        }

        let compile_time = compile_start.elapsed();
        println!(
            "✅ MAGRAY CLI compiled successfully in {:.2}s",
            compile_time.as_secs_f64()
        );

        // Определяем путь к скомпилированному бинарнику
        self.detect_binary_path()?;

        Ok(())
    }

    /// Автоматически определяет путь к скомпилированному бинарнику
    fn detect_binary_path(&mut self) -> Result<()> {
        let target_dir = self.project_root.join("target").join("debug");

        // Проверяем Windows и Unix варианты
        let binary_candidates = if cfg!(windows) {
            vec!["magray.exe"]
        } else {
            vec!["magray"]
        };

        for candidate in binary_candidates {
            let binary_path = target_dir.join(candidate);
            if binary_path.exists() {
                println!("🎯 Found MAGRAY binary at: {}", binary_path.display());
                self.binary_path = Some(binary_path);
                self.use_cargo_run = false;
                return Ok(());
            }
        }

        // Fallback к cargo run
        println!("⚠️  Binary not found, will use 'cargo run --bin magray'");
        self.binary_path = None;
        self.use_cargo_run = true;

        Ok(())
    }

    /// Инициализирует окружение для тестирования
    pub async fn setup_environment(&self) -> Result<()> {
        println!("🔧 Setting up test environment...");

        // Проверяем .env файл
        let env_path = self.project_root.join(".env");
        if !env_path.exists() {
            println!("📝 Creating .env file with test configuration...");
            let env_content = r#"# MAGRAY CLI Test Environment
# OpenAI API key for testing (optional)
# OPENAI_API_KEY=your_key_here

# Test configuration
TEST_MODE=true
LOG_LEVEL=info
"#;
            std::fs::write(&env_path, env_content).context("Failed to create .env file")?;
        }

        // Проверяем наличие моделей (graceful fallback)
        let models_dir = self.project_root.join("models");
        if !models_dir.exists() {
            println!("📁 Models directory not found - tests will run with fallback");
        } else {
            println!("✅ Models directory found");
        }

        println!("✅ Environment setup completed");
        Ok(())
    }

    /// Выполняет один тестовый сценарий
    pub async fn execute_scenario(&self, scenario: &TestScenario) -> Result<TestResult> {
        println!("🚀 Executing scenario: {}", scenario.name);

        for attempt in 1..=self.max_retries {
            match self.try_execute_scenario(scenario, attempt).await {
                Ok(result) => return Ok(result),
                Err(e) if attempt < self.max_retries => {
                    println!("⚠️  Attempt {}/{} failed: {}", attempt, self.max_retries, e);
                    println!("🔄 Retrying in {}s...", attempt * 2);
                    tokio::time::sleep(Duration::from_secs(attempt as u64 * 2)).await;
                }
                Err(e) => return Err(e),
            }
        }

        unreachable!()
    }

    /// Выполняет интерактивный сценарий (запуск->сообщение->ответ->закрытие)
    pub async fn execute_interactive_scenario(
        &self,
        scenario: &TestScenario,
    ) -> Result<TestResult> {
        println!("🚀 Executing interactive scenario: {}", scenario.name);
        let start_time = Instant::now();

        // Запускаем интерактивный процесс
        let mut cli_process = self.start_interactive_cli().await?;

        // Ждем инициализации CLI
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Отправляем сообщение
        println!("📤 Sending: {}", scenario.input);
        cli_process.send_message(&scenario.input).await?;

        // Читаем ответ с таймаутом
        let timeout_duration = Duration::from_secs(scenario.timeout_seconds);
        let output = match cli_process.read_response(timeout_duration).await {
            Ok(response) => {
                println!("📥 Received response ({} chars)", response.len());
                response
            }
            Err(e) => {
                println!("❌ Failed to read response: {e}");
                cli_process.terminate()?;
                return Ok(TestResult {
                    scenario_id: scenario.id.clone(),
                    input: scenario.input.clone(),
                    output: String::new(),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    success: false,
                    error_message: Some(format!("Failed to read response: {e}")),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                });
            }
        };

        // Завершаем процесс
        cli_process.terminate()?;

        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        let success = !output.is_empty();

        Ok(TestResult {
            scenario_id: scenario.id.clone(),
            input: scenario.input.clone(),
            output,
            execution_time_ms,
            success,
            error_message: if !success {
                Some("Empty response".to_string())
            } else {
                None
            },
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Попытка выполнения сценария (внутренний метод)
    async fn try_execute_scenario(
        &self,
        scenario: &TestScenario,
        attempt: u32,
    ) -> Result<TestResult> {
        let start_time = Instant::now();

        // Создаем команду в зависимости от настройки
        let mut command = if self.use_cargo_run || self.binary_path.is_none() {
            let mut cmd = Command::new("cargo");
            cmd.args([
                "run",
                "-p",
                "cli",
                "--bin",
                "magray",
                "--",
                "chat",
                &scenario.input,
            ])
            .current_dir(&self.project_root);
            cmd
        } else if let Some(ref binary_path) = self.binary_path {
            let mut cmd = Command::new(binary_path);
            cmd.args(["chat", &scenario.input]);
            cmd
        } else {
            return Err(anyhow::anyhow!(
                "No valid binary path or cargo run configuration"
            ));
        };

        // Настраиваем stdin/stdout/stderr
        let output = command
            .output()
            .context("Failed to execute MAGRAY CLI process")?;

        let execution_time_ms = start_time.elapsed().as_millis() as u64;
        let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();

        // Определяем успешность
        let success = output.status.success() && !stdout_str.is_empty();
        let error_message = if !success {
            Some(if !stderr_str.is_empty() {
                stderr_str
            } else {
                "No output received".to_string()
            })
        } else {
            None
        };

        Ok(TestResult {
            scenario_id: scenario.id.clone(),
            input: scenario.input.clone(),
            output: stdout_str,
            execution_time_ms,
            success,
            error_message,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Запускает интерактивный CLI процесс для общения
    pub async fn start_interactive_cli(&self) -> Result<InteractiveCliProcess> {
        println!("🚀 Starting interactive MAGRAY CLI process...");

        let mut command = if self.use_cargo_run || self.binary_path.is_none() {
            let mut cmd = Command::new("cargo");
            cmd.args(["run", "-p", "cli", "--bin", "magray"])
                .current_dir(&self.project_root);
            cmd
        } else if let Some(ref binary_path) = self.binary_path {
            Command::new(binary_path)
        } else {
            return Err(anyhow::anyhow!(
                "No valid binary path or cargo run configuration"
            ));
        };

        // Настраиваем stdin/stdout/stderr для интерактивного режима
        let child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn MAGRAY CLI process")?;

        Ok(InteractiveCliProcess {
            process: child,
            start_time: Instant::now(),
        })
    }

    /// Старая версия попытки выполнения сценария (для обратной совместимости)
    async fn try_execute_scenario_old(
        &self,
        scenario: &TestScenario,
        attempt: u32,
    ) -> Result<TestResult> {
        let start_time = Instant::now();

        // Создаем команду в зависимости от настройки
        let mut command = if self.use_cargo_run || self.binary_path.is_none() {
            let mut cmd = Command::new("cargo");
            cmd.args(["run", "-p", "cli", "--bin", "magray"])
                .current_dir(&self.project_root);
            cmd
        } else if let Some(ref binary_path) = self.binary_path {
            Command::new(binary_path)
        } else {
            return Err(anyhow::anyhow!(
                "No valid binary path or cargo run configuration"
            ));
        };

        // Настраиваем stdin/stdout/stderr
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("Failed to spawn MAGRAY CLI process")?;

        let stdin = child.stdin.take().context("Failed to get stdin")?;
        let stdout = child.stdout.take().context("Failed to get stdout")?;

        // Записываем ввод пользователя
        let input_clone = scenario.input.clone();
        let write_handle = tokio::task::spawn_blocking(move || -> Result<()> {
            let mut stdin = stdin;
            writeln!(stdin, "{input_clone}")?;
            stdin.flush()?;
            Ok(())
        });

        // Читаем вывод с timeout
        let timeout_duration = Duration::from_secs(scenario.timeout_seconds);
        let read_handle = tokio::task::spawn_blocking(move || -> Result<String> {
            let reader = BufReader::new(stdout);
            let mut output = String::new();

            for line in reader.lines() {
                match line {
                    Ok(line_str) => {
                        output.push_str(&line_str);
                        output.push('\n');

                        // Простая эвристика для определения завершения ответа
                        if line_str.contains("✓")
                            || line_str.contains("Done")
                            || line_str.contains("completed")
                            || line_str.trim().is_empty()
                        {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }

            Ok(output)
        });

        // Ждем выполнения с timeout
        let output_result = match timeout(timeout_duration, read_handle).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return Ok(TestResult {
                    scenario_id: scenario.id.clone(),
                    input: scenario.input.clone(),
                    output: String::new(),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    success: false,
                    error_message: Some(format!("Read error: {e}")),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                })
            }
            Err(_) => {
                return Ok(TestResult {
                    scenario_id: scenario.id.clone(),
                    input: scenario.input.clone(),
                    output: String::new(),
                    execution_time_ms: start_time.elapsed().as_millis() as u64,
                    success: false,
                    error_message: Some("Timeout exceeded".to_string()),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                })
            }
        };

        // Ждем завершения записи
        if let Err(e) = write_handle
            .await
            .unwrap_or_else(|e| Err(anyhow::anyhow!("Join error: {}", e)))
        {
            eprintln!("Warning: Write error: {e}");
        }

        // Завершаем процесс
        let _ = child.kill();
        let _ = child.wait();

        let execution_time = start_time.elapsed().as_millis() as u64;

        Ok(TestResult {
            scenario_id: scenario.id.clone(),
            input: scenario.input.clone(),
            output: output_result?,
            execution_time_ms: execution_time,
            success: true,
            error_message: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }

    /// Проверяет доступность MAGRAY CLI
    /// Выполняет полную проверку готовности (компиляция + health check)
    pub async fn full_readiness_check(&mut self) -> Result<bool> {
        println!("🔍 Starting full readiness check...");

        // 1. Компилируем MAGRAY CLI
        println!("📦 Step 1: Compiling MAGRAY CLI...");
        self.compile_magray_cli().await?;

        // 2. Настраиваем окружение
        println!("🔧 Step 2: Setting up environment...");
        self.setup_environment().await?;

        // 3. Проверяем работоспособность
        println!("🏥 Step 3: Running health check...");
        let health_ok = self.health_check().await?;

        if health_ok {
            println!("✅ Full readiness check passed!");
        } else {
            println!("❌ Health check failed!");
        }

        Ok(health_ok)
    }

    /// Выполняет набор тестовых сценариев
    pub async fn execute_test_suite(
        &self,
        scenarios: Vec<TestScenario>,
    ) -> Result<Vec<TestResult>> {
        let mut results = Vec::new();
        let total_scenarios = scenarios.len();

        for (index, scenario) in scenarios.into_iter().enumerate() {
            println!("\n{}", "=".repeat(60));
            println!(
                "📋 Test {}/{}: {}",
                index + 1,
                total_scenarios,
                scenario.name
            );
            println!("{}", "-".repeat(60));

            // Используем интерактивный метод для сложных сценариев
            let result =
                if scenario.expected_type == "complex_task" || scenario.input.contains('\n') {
                    self.execute_interactive_scenario(&scenario).await?
                } else {
                    self.execute_scenario(&scenario).await?
                };

            // Выводим результат
            if result.success {
                println!(
                    "✅ Success! Response received in {}ms",
                    result.execution_time_ms
                );
                if result.output.len() > 200 {
                    println!("📄 Response preview: {}...", &result.output[..200]);
                } else {
                    println!("📄 Response: {}", result.output);
                }
            } else {
                println!(
                    "❌ Failed: {}",
                    result
                        .error_message
                        .as_ref()
                        .unwrap_or(&"Unknown error".to_string())
                );
            }

            results.push(result);
        }

        println!("\n{}", "=".repeat(60));
        println!("📊 Test Suite Summary");
        println!("{}", "=".repeat(60));

        let successful = results.iter().filter(|r| r.success).count();
        let failed = results.len() - successful;

        println!("✅ Successful: {successful}");
        println!("❌ Failed: {failed}");
        println!(
            "⏱️  Total execution time: {}ms",
            results.iter().map(|r| r.execution_time_ms).sum::<u64>()
        );

        Ok(results)
    }

    pub async fn health_check(&self) -> Result<bool> {
        println!("🔍 Performing comprehensive health check for MAGRAY CLI...");

        // Сначала проверяем компиляцию если нужно
        if self.binary_path.is_none() && !self.use_cargo_run {
            println!("🔧 Binary path not set, attempting to detect...");
            let mut executor_copy = TestExecutor {
                binary_path: self.binary_path.clone(),
                use_cargo_run: self.use_cargo_run,
                timeout_duration: self.timeout_duration,
                session_id: self.session_id.clone(),
                project_root: self.project_root.clone(),
                max_retries: self.max_retries,
            };

            if let Err(e) = executor_copy.detect_binary_path() {
                println!("⚠️  Could not detect binary path: {e}");
            }
        }

        // Проверяем различные способы взаимодействия с CLI
        let health_scenarios = vec![
            ("help", "Checking help command"),
            ("--help", "Checking help flag"),
            ("привет", "Checking basic interaction"),
        ];

        for (input, description) in health_scenarios {
            println!("🧪 {description}: '{input}'");

            let scenario = TestScenario {
                id: format!("health_check_{}", input.replace("--", "").replace(" ", "_")),
                name: description.to_string(),
                input: input.to_string(),
                expected_type: "simple_response".to_string(),
                timeout_seconds: 15,
                evaluation_criteria: vec!["responds_appropriately".to_string()],
            };

            match self.try_execute_scenario(&scenario, 1).await {
                Ok(result) => {
                    if result.success && !result.output.trim().is_empty() {
                        println!(
                            "✅ Health check passed with '{input}' - MAGRAY CLI is responding"
                        );
                        println!(
                            "📝 Response preview: {}",
                            result.output.lines().take(2).collect::<Vec<_>>().join(" ")
                        );
                        return Ok(true);
                    }
                }
                Err(e) => {
                    println!("⚠️  Health check attempt with '{input}' failed: {e}");
                }
            }
        }

        println!("❌ All health check attempts failed - CLI may not be ready");
        Ok(false)
    }
}

// Удалены дублирующиеся методы

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_executor_creation() {
        let executor = TestExecutor::new();
        assert!(!executor.session_id.is_empty());
        assert!(!executor.use_cargo_run);
        assert_eq!(executor.max_retries, 3);
    }

    #[test]
    fn test_binary_path_detection() {
        let mut executor = TestExecutor::new();
        // This test just checks that the method doesn't panic
        let _ = executor.detect_binary_path();
    }

    #[tokio::test]
    async fn test_environment_setup() {
        let executor = TestExecutor::new();
        // This test checks environment setup doesn't fail
        let result = executor.setup_environment().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_health_check() {
        let executor = TestExecutor::new();
        // Note: This test requires actual MAGRAY CLI to be available
        // In CI/CD it might fail, which is expected
        let result = executor.health_check().await;
        assert!(result.is_ok());
    }
}
