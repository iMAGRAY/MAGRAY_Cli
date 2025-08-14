use std::fs;
use std::path::Path;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use super::super::integration::human_like_testing::TestScenario;

/// Scenario Manager - загружает и управляет тестовыми сценариями из YAML файлов
pub struct ScenarioManager {
    scenarios_dir: String,
}

/// Структура для загрузки YAML файла со сценариями
#[derive(Debug, Deserialize, Serialize)]
struct ScenariosFile {
    scenarios: Vec<YamlScenario>,
    meta: Option<ScenarioMeta>,
}

/// YAML представление тестового сценария
#[derive(Debug, Deserialize, Serialize)]
struct YamlScenario {
    id: String,
    name: String,
    input: String,
    expected_type: String,
    timeout_seconds: u64,
    evaluation_criteria: Vec<String>,
}

/// Метаданные файла сценариев
#[derive(Debug, Deserialize, Serialize)]
struct ScenarioMeta {
    version: String,
    created: String,
    description: String,
    total_scenarios: u32,
    estimated_total_time_minutes: u32,
    categories: Vec<String>,
    success_criteria: Option<SuccessCriteria>,
}

/// Критерии успеха для набора тестов
#[derive(Debug, Deserialize, Serialize)]
struct SuccessCriteria {
    minimum_pass_rate: f64,
    average_response_time_seconds: u32,
    minimum_evaluation_score: f64,
}

/// Статистика загруженных сценариев
#[derive(Debug, Clone)]
pub struct ScenarioStats {
    pub total_count: usize,
    pub by_type: std::collections::HashMap<String, usize>,
    pub estimated_time_minutes: u32,
    pub categories: Vec<String>,
}

impl ScenarioManager {
    /// Создает новый Scenario Manager
    pub fn new<P: AsRef<Path>>(scenarios_dir: P) -> Self {
        Self {
            scenarios_dir: scenarios_dir.as_ref().to_string_lossy().to_string(),
        }
    }

    /// Загружает все сценарии из указанного YAML файла
    pub fn load_scenarios_from_file<P: AsRef<Path>>(&self, file_path: P) -> Result<Vec<TestScenario>> {
        let full_path = Path::new(&self.scenarios_dir).join(file_path);
        
        println!("📁 Loading scenarios from: {}", full_path.display());
        
        let content = fs::read_to_string(&full_path)
            .with_context(|| format!("Failed to read scenarios file: {}", full_path.display()))?;

        let scenarios_file: ScenariosFile = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML file: {}", full_path.display()))?;

        let scenarios: Vec<TestScenario> = scenarios_file
            .scenarios
            .into_iter()
            .map(|yaml_scenario| TestScenario {
                id: yaml_scenario.id,
                name: yaml_scenario.name,
                input: yaml_scenario.input,
                expected_type: yaml_scenario.expected_type,
                timeout_seconds: yaml_scenario.timeout_seconds,
                evaluation_criteria: yaml_scenario.evaluation_criteria,
            })
            .collect();

        if let Some(meta) = scenarios_file.meta {
            println!("📋 Loaded {} scenarios (version: {}, estimated time: {}min)", 
                   scenarios.len(), meta.version, meta.estimated_total_time_minutes);
            
            if !meta.categories.is_empty() {
                println!("🏷️  Categories: {}", meta.categories.join(", "));
            }
        }

        Ok(scenarios)
    }

    /// Загружает сценарии определенного типа
    pub fn load_scenarios_by_type(&self, scenario_type: &str) -> Result<Vec<TestScenario>> {
        let all_scenarios = self.load_all_scenarios()?;
        
        let filtered: Vec<TestScenario> = all_scenarios
            .into_iter()
            .filter(|scenario| scenario.expected_type == scenario_type)
            .collect();

        println!("🔍 Filtered {} scenarios of type '{}'", filtered.len(), scenario_type);
        
        Ok(filtered)
    }

    /// Загружает все доступные сценарии из всех YAML файлов
    pub fn load_all_scenarios(&self) -> Result<Vec<TestScenario>> {
        let scenarios_path = Path::new(&self.scenarios_dir);
        
        if !scenarios_path.exists() {
            return Err(anyhow::anyhow!("Scenarios directory does not exist: {}", scenarios_path.display()));
        }

        let mut all_scenarios = Vec::new();
        
        // Ищем все YAML файлы в директории
        for entry in fs::read_dir(scenarios_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") || 
               path.extension().and_then(|s| s.to_str()) == Some("yml") {
                
                match self.load_scenarios_from_file(&path.file_name().unwrap()) {
                    Ok(mut scenarios) => {
                        println!("✅ Loaded {} scenarios from {}", 
                               scenarios.len(), path.file_name().unwrap().to_string_lossy());
                        all_scenarios.append(&mut scenarios);
                    }
                    Err(e) => {
                        eprintln!("❌ Failed to load scenarios from {}: {}", 
                                path.file_name().unwrap().to_string_lossy(), e);
                    }
                }
            }
        }

        println!("📊 Total scenarios loaded: {}", all_scenarios.len());
        
        Ok(all_scenarios)
    }

    /// Возвращает статистику загруженных сценариев
    pub fn get_scenarios_stats(&self) -> Result<ScenarioStats> {
        let scenarios = self.load_all_scenarios()?;
        
        let mut by_type = std::collections::HashMap::new();
        let mut categories = std::collections::HashSet::new();
        
        for scenario in &scenarios {
            *by_type.entry(scenario.expected_type.clone()).or_insert(0) += 1;
            
            // Извлекаем категории из evaluation_criteria
            for criterion in &scenario.evaluation_criteria {
                if criterion.contains("_") {
                    let parts: Vec<&str> = criterion.split('_').collect();
                    if !parts.is_empty() {
                        categories.insert(parts[0].to_string());
                    }
                }
            }
        }

        // Примерная оценка времени (среднее время выполнения * количество)
        let estimated_time = scenarios
            .iter()
            .map(|s| s.timeout_seconds)
            .sum::<u64>() / 60; // конвертируем в минуты

        Ok(ScenarioStats {
            total_count: scenarios.len(),
            by_type,
            estimated_time_minutes: estimated_time as u32,
            categories: categories.into_iter().collect(),
        })
    }

    /// Создает простые тестовые сценарии программно
    pub fn create_basic_scenarios(&self) -> Vec<TestScenario> {
        vec![
            TestScenario {
                id: "quick_greeting".to_string(),
                name: "Quick Greeting".to_string(),
                input: "привет".to_string(),
                expected_type: "simple_response".to_string(),
                timeout_seconds: 10,
                evaluation_criteria: vec![
                    "responds_appropriately".to_string(),
                    "shows_politeness".to_string(),
                ],
            },
            TestScenario {
                id: "help_command".to_string(),
                name: "Help Command".to_string(),
                input: "help".to_string(),
                expected_type: "simple_response".to_string(),
                timeout_seconds: 15,
                evaluation_criteria: vec![
                    "provides_help_info".to_string(),
                    "lists_available_commands".to_string(),
                ],
            },
        ]
    }

    /// Валидирует корректность сценариев
    pub fn validate_scenarios(&self, scenarios: &[TestScenario]) -> Result<()> {
        for scenario in scenarios {
            if scenario.id.is_empty() {
                return Err(anyhow::anyhow!("Scenario has empty ID"));
            }
            
            if scenario.input.is_empty() {
                return Err(anyhow::anyhow!("Scenario '{}' has empty input", scenario.id));
            }
            
            if scenario.timeout_seconds == 0 {
                return Err(anyhow::anyhow!("Scenario '{}' has zero timeout", scenario.id));
            }
            
            if scenario.timeout_seconds > 300 {
                println!("⚠️  Warning: Scenario '{}' has very long timeout: {}s", 
                       scenario.id, scenario.timeout_seconds);
            }
        }
        
        // Проверяем на дублированные ID
        let mut ids = std::collections::HashSet::new();
        for scenario in scenarios {
            if !ids.insert(&scenario.id) {
                return Err(anyhow::anyhow!("Duplicate scenario ID: {}", scenario.id));
            }
        }
        
        println!("✅ All {} scenarios validated successfully", scenarios.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::write;

    #[test]
    fn test_scenario_manager_creation() {
        let manager = ScenarioManager::new("./tests/scenarios");
        assert!(!manager.scenarios_dir.is_empty());
    }

    #[test]
    fn test_basic_scenarios_creation() {
        let manager = ScenarioManager::new("./tests/scenarios");
        let scenarios = manager.create_basic_scenarios();
        
        assert_eq!(scenarios.len(), 2);
        assert_eq!(scenarios[0].id, "quick_greeting");
        assert_eq!(scenarios[1].id, "help_command");
    }

    #[test]
    fn test_scenario_validation() {
        let manager = ScenarioManager::new("./tests/scenarios");
        let scenarios = manager.create_basic_scenarios();
        
        assert!(manager.validate_scenarios(&scenarios).is_ok());
    }

    #[test]
    fn test_yaml_parsing() {
        let yaml_content = r#"
scenarios:
  - id: "test_scenario"
    name: "Test Scenario"
    input: "test input"
    expected_type: "simple_response"
    timeout_seconds: 30
    evaluation_criteria:
      - "criterion1"
      - "criterion2"

meta:
  version: "1.0"
  description: "Test scenarios"
  total_scenarios: 1
  estimated_total_time_minutes: 5
  categories: ["test"]
"#;

        let scenarios_file: ScenariosFile = serde_yaml::from_str(yaml_content).unwrap();
        assert_eq!(scenarios_file.scenarios.len(), 1);
        assert_eq!(scenarios_file.scenarios[0].id, "test_scenario");
    }
}