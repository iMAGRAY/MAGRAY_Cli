use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio;

use llm::{LlmClient, ActionPlannerAgent, ToolSelectorAgent, ParameterExtractorAgent};
use tools::{ToolRegistry, ToolInput, ToolOutput};

// @component: {"k":"C","id":"smart_router","t":"Smart task orchestration","m":{"cur":70,"tgt":90,"u":"%"},"d":["llm_client","tools"],"f":["routing","orchestration"]}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlan {
    pub reasoning: String,
    pub steps: Vec<PlannedAction>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedAction {
    pub tool: String,
    pub description: String,
    pub args: HashMap<String, String>,
    pub expected_output: String,
}

pub struct SmartRouter {
    tool_registry: ToolRegistry,
    // Специализированные агенты для разных задач
    action_planner: ActionPlannerAgent,
    tool_selector: ToolSelectorAgent,
    parameter_extractor: ParameterExtractorAgent,
}

impl SmartRouter {
    pub fn new(llm_client: LlmClient) -> Self {
        // Создаем специализированные агенты с тем же LLM клиентом
        let action_planner = ActionPlannerAgent::new(llm_client.clone());
        let tool_selector = ToolSelectorAgent::new(llm_client.clone());
        let parameter_extractor = ParameterExtractorAgent::new(llm_client.clone());
        
        Self {
            tool_registry: ToolRegistry::new(),
            action_planner,
            tool_selector,
            parameter_extractor,
        }
    }
    
    /// Анализирует запрос пользователя и создает план действий используя специализированные агенты
    pub async fn analyze_and_plan(&self, user_query: &str) -> Result<ActionPlan> {
        println!("[●] Используем специализированные агенты для планирования...");
        
        let available_tools: Vec<String> = self.tool_registry
            .list_tools()
            .iter()
            .map(|t| t.name.clone())
            .collect();
        
        // Используем ActionPlannerAgent вместо захардкоженного промпта
        let plan = self.action_planner
            .create_plan(user_query, &available_tools)
            .await?;
        
        // Конвертируем план из специализированного агента в наш формат
        let converted_plan = ActionPlan {
            reasoning: plan.reasoning,
            confidence: plan.confidence,
            steps: plan.steps.into_iter().map(|step| PlannedAction {
                tool: step.tool,
                description: step.description,
                args: step.parameters,
                expected_output: "Результат выполнения инструмента".to_string(),
            }).collect(),
        };
        
        println!("[✓] План создан с помощью ActionPlannerAgent");
        Ok(converted_plan)
    }
    
    /// Выполняет план действий
    pub async fn execute_plan(&self, plan: &ActionPlan) -> Result<Vec<ToolOutput>> {
        let mut results = Vec::new();
        
        println!("[●] Доступные инструменты: {:?}", 
                self.tool_registry.list_tools().iter().map(|t| &t.name).collect::<Vec<_>>());
        
        for (i, action) in plan.steps.iter().enumerate() {
            println!("[AI] Шаг {}: {} (инструмент: {})", i + 1, action.description, action.tool);
            println!("[●] Аргументы: {:?}", action.args);
            
            let tool = self.tool_registry.get(&action.tool)
                .ok_or_else(|| anyhow!("Инструмент '{}' не найден в реестре", action.tool))?;
                
            let input = ToolInput {
                command: action.tool.clone(),
                args: action.args.clone(),
                context: Some(action.description.clone()),
            };
            
            match tool.execute(input).await {
                Ok(result) => {
                    println!("[✓] Шаг {} выполнен успешно", i + 1);
                    results.push(result);
                }
                Err(e) => {
                    println!("[✗] Ошибка в шаге {}: {}", i + 1, e);
                    return Err(e);
                }
            }
            
            // Небольшая пауза между действиями для UX
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        
        Ok(results)
    }
    
    /// Умный выбор и выполнение одного инструмента (для простых запросов)
    pub async fn process_single_tool_request(&self, user_query: &str) -> Result<String> {
        println!("[●] Используем умный выбор инструмента...");
        
        let available_tools: Vec<String> = self.tool_registry
            .list_tools()
            .iter()
            .map(|t| t.name.clone())
            .collect();
        
        // 1. Выбираем подходящий инструмент
        let tool_selection = self.tool_selector
            .select_tool(user_query, &available_tools)
            .await?;
        
        println!("[AI] Выбран инструмент: {} (уверенность: {:.1}%)", 
                tool_selection.tool_name, tool_selection.confidence * 100.0);
        
        // 2. Извлекаем параметры для выбранного инструмента
        let tools = self.tool_registry.list_tools();
        let tool_spec = tools
            .iter()
            .find(|t| t.name == tool_selection.tool_name)
            .ok_or_else(|| anyhow!("Инструмент {} не найден", tool_selection.tool_name))?;
        
        // Определяем требуемые параметры из схемы инструмента
        let required_params = self.extract_required_params(&tool_spec.input_schema);
        
        let parameter_extraction = self.parameter_extractor
            .extract_parameters(user_query, &tool_selection.tool_name, &required_params)
            .await?;
        
        println!("[AI] Извлечены параметры: {:?}", parameter_extraction.parameters);
        
        // 3. Выполняем инструмент
        let tool = self.tool_registry.get(&tool_selection.tool_name)
            .ok_or_else(|| anyhow!("Инструмент '{}' не найден в реестре", tool_selection.tool_name))?;
        
        let input = ToolInput {
            command: tool_selection.tool_name.clone(),
            args: parameter_extraction.parameters,
            context: Some(user_query.to_string()),
        };
        
        let result = tool.execute(input).await?;
        
        if result.success {
            Ok(result.formatted_output.unwrap_or(result.result))
        } else {
            Err(anyhow!("Ошибка выполнения инструмента: {}", result.result))
        }
    }
    
    /// Полный цикл: анализ → планирование → выполнение (для сложных запросов)
    pub async fn process_smart_request(&self, user_query: &str) -> Result<String> {
        println!("[●] Анализирую запрос с помощью AI...");
        
        let plan = self.analyze_and_plan(user_query).await?;
        
        println!("[AI] План создан: {}", plan.reasoning);
        println!("[►] Будет выполнено {} действий", plan.steps.len());
        
        // Для простых запросов (1 действие) используем быстрый путь
        if plan.steps.len() == 1 {
            println!("[●] Простой запрос, используем быстрое выполнение...");
            return self.process_single_tool_request(user_query).await;
        }
        
        if plan.confidence < 0.7 {
            println!("[⚠️] Низкая уверенность в плане ({:.1}%), продолжить? [y/N]", plan.confidence * 100.0);
            // TODO: Добавить интерактивное подтверждение
        }
        
        let results = self.execute_plan(&plan).await?;
        
        self.format_results(&plan, &results)
    }
    
    /// Извлекает требуемые параметры из JSON схемы инструмента
    pub fn extract_required_params(&self, schema: &str) -> Vec<String> {
        // Простой парсинг JSON схемы для извлечения имен параметров
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(schema) {
            if let Some(obj) = parsed.as_object() {
                return obj.keys().cloned().collect();
            }
        }
        
        // Fallback: стандартные параметры для известных инструментов
        vec!["path".to_string(), "content".to_string(), "command".to_string(), 
             "message".to_string(), "query".to_string()]
    }
    
    pub fn format_results(&self, plan: &ActionPlan, results: &[ToolOutput]) -> Result<String> {
        let mut output = String::new();
        
        output.push_str(&format!("[✓] План выполнен: {}\n", plan.reasoning));
        output.push_str(&format!("[●] Результаты {} действий:\n\n", results.len()));
        
        for (i, (action, result)) in plan.steps.iter().zip(results.iter()).enumerate() {
            output.push_str(&format!("{}. {}\n", i + 1, action.description));
            
            if result.success {
                if let Some(formatted) = &result.formatted_output {
                    output.push_str(formatted);
                } else {
                    output.push_str(&format!("   [✓] {}\n", result.result));
                }
            } else {
                output.push_str(&format!("   [✗] {}\n", result.result));
            }
            
            output.push('\n');
        }
        
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test] 
    fn test_action_plan_serialization() {
        let plan = ActionPlan {
            reasoning: "Test plan".to_string(),
            confidence: 0.9,
            steps: vec![
                PlannedAction {
                    tool: "file_read".to_string(),
                    description: "Read file".to_string(),
                    args: HashMap::from([("path".to_string(), "test.txt".to_string())]),
                    expected_output: "File contents".to_string(),
                }
            ],
        };
        
        let json = serde_json::to_string(&plan).unwrap();
        let parsed: ActionPlan = serde_json::from_str(&json).unwrap();
        
        assert_eq!(parsed.confidence, 0.9);
        assert_eq!(parsed.steps.len(), 1);
    }
}