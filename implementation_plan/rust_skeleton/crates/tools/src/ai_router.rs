use crate::{ToolRegistry, ToolInput, ToolOutput};
use anyhow::{anyhow, Result};
use llm::LlmClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    llm_client: LlmClient,
    tool_registry: ToolRegistry,
}

impl SmartRouter {
    pub fn new(llm_client: LlmClient) -> Self {
        Self {
            llm_client,
            tool_registry: ToolRegistry::new(),
        }
    }
    
    /// Анализирует запрос пользователя и создает план действий
    pub async fn analyze_and_plan(&self, user_query: &str) -> Result<ActionPlan> {
        let tools_info = self.get_available_tools_description();
        
        let system_prompt = self.create_planning_prompt(&tools_info);
        let user_prompt = format!(
            "Пользователь просит: \"{}\"\n\nПроанализируй запрос и создай план действий в формате JSON.",
            user_query
        );
        
        let full_prompt = format!("{}\n\nЗапрос пользователя:\n{}", system_prompt, user_prompt);
        
        let llm_response = self.llm_client.chat(&full_prompt).await
            .map_err(|e| anyhow!("Ошибка LLM анализа: {}", e))?;
            
        self.parse_llm_response(&llm_response)
    }
    
    /// Выполняет план действий
    pub async fn execute_plan(&self, plan: &ActionPlan) -> Result<Vec<ToolOutput>> {
        let mut results = Vec::new();
        
        for action in &plan.steps {
            println!("[AI] Выполняю: {}", action.description);
            
            let tool = self.tool_registry.get(&action.tool)
                .ok_or_else(|| anyhow!("Инструмент '{}' не найден", action.tool))?;
                
            let input = ToolInput {
                command: action.tool.clone(),
                args: action.args.clone(),
                context: Some(action.description.clone()),
            };
            
            let result = tool.execute(input).await?;
            results.push(result);
            
            // Небольшая пауза между действиями для UX
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        
        Ok(results)
    }
    
    /// Полный цикл: анализ → планирование → выполнение
    pub async fn process_smart_request(&self, user_query: &str) -> Result<String> {
        println!("[●] Анализирую запрос с помощью AI...");
        
        let plan = self.analyze_and_plan(user_query).await?;
        
        println!("[AI] План создан: {}", plan.reasoning);
        println!("[►] Будет выполнено {} действий", plan.steps.len());
        
        if plan.confidence < 0.7 {
            println!("[⚠️] Низкая уверенность в плане ({:.1}%), продолжить? [y/N]", plan.confidence * 100.0);
            // TODO: Добавить интерактивное подтверждение
        }
        
        let results = self.execute_plan(&plan).await?;
        
        self.format_results(&plan, &results)
    }
    
    fn create_planning_prompt(&self, tools_info: &str) -> String {
        format!(r#"Ты - умный AI ассистент MAGRAY CLI. Твоя задача - анализировать запросы пользователей и создавать планы действий используя доступные инструменты.

ДОСТУПНЫЕ ИНСТРУМЕНТЫ:
{}

ПРАВИЛА ПЛАНИРОВАНИЯ:
1. Анализируй запрос и определи, какие инструменты нужны
2. Создай последовательный план действий  
3. Каждое действие должно иметь четкое описание
4. Оцени уверенность в плане (0.0-1.0)

ФОРМАТ ОТВЕТА (строго JSON):
{{
  "reasoning": "Объяснение логики планирования",
  "confidence": 0.85,
  "steps": [
    {{
      "tool": "имя_инструмента",
      "description": "Что делает этот шаг",
      "args": {{"key": "value"}},
      "expected_output": "Ожидаемый результат"
    }}
  ]
}}

ПРИМЕРЫ ПЛАНОВ:
- "покажи содержимое файла main.rs" → file_read с path="main.rs"
- "создай новый файл README" → file_write с path="README.md" и базовым содержимым
- "покажи файлы в папке src" → dir_list с path="src/"

Отвечай ТОЛЬКО валидным JSON без дополнительного текста."#, tools_info)
    }
    
    fn get_available_tools_description(&self) -> String {
        let tools = self.tool_registry.list_tools();
        let mut description = String::new();
        
        for tool in tools {
            description.push_str(&format!(
                "- {}: {}\n  Использование: {}\n  Примеры: {}\n\n",
                tool.name,
                tool.description,
                tool.usage,
                tool.examples.join(", ")
            ));
        }
        
        description
    }
    
    fn parse_llm_response(&self, response: &str) -> Result<ActionPlan> {
        // Пытаемся найти JSON в ответе
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
        let json_str = &response[json_start..json_end];
        
        serde_json::from_str::<ActionPlan>(json_str)
            .map_err(|e| anyhow!("Ошибка парсинга JSON ответа: {}\nОтвет LLM: {}", e, response))
    }
    
    fn format_results(&self, plan: &ActionPlan, results: &[ToolOutput]) -> Result<String> {
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