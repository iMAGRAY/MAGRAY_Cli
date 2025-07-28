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
        
        match self.llm_client.chat(&full_prompt).await {
            Ok(response) => match self.parse_llm_response(&response) {
                Ok(plan) => Ok(plan),
                Err(parse_err) => {
                    eprintln!("[✗] Ошибка парсинга LLM: {}\n[●] Включаю локальное планирование", parse_err);
                    self.heuristic_plan(user_query).await
                }
            },
            Err(llm_err) => {
                eprintln!("[✗] Ошибка LLM: {}\n[●] Включаю локальное планирование", llm_err);
                self.heuristic_plan(user_query).await
            }
        }
    }

    /// Простое локальное планирование, если LLM недоступен
    async fn heuristic_plan(&self, user_query: &str) -> Result<ActionPlan> {
        if let Some(tool) = self.tool_registry.find_tool_for_query(user_query).await {
            let input = tool.parse_natural_language(user_query).await?;
            let step = PlannedAction {
                tool: input.command.clone(),
                description: user_query.to_string(),
                args: input.args.clone(),
                expected_output: "".to_string(),
            };
            Ok(ActionPlan {
                reasoning: "Локальный эвристический планировщик".to_string(),
                steps: vec![step],
                confidence: 0.5,
            })
        } else {
            Err(anyhow!("Не удалось спланировать задачу — инструмент не найден"))
        }
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
        format!(r#"Ты - умный AI ассистент MAGRAY CLI. Анализируй запросы пользователей и создавай планы действий используя ТОЧНЫЕ имена инструментов.

ДОСТУПНЫЕ ИНСТРУМЕНТЫ:
{}

КРИТИЧЕСКИ ВАЖНО - ИСПОЛЬЗУЙ ТОЛЬКО ЭТИ ТОЧНЫЕ ИМЕНА:
- file_read (чтение файлов)
- file_write (создание/запись файлов) 
- dir_list (просмотр директорий)
- git_status (git статус)
- git_commit (git коммиты)
- web_search (поиск в интернете)
- shell_exec (выполнение команд)

ПРАВИЛА:
1. Используй ТОЛЬКО имена инструментов из списка выше
2. Для аргументов file_read/file_write используй "path"
3. Для dir_list используй "path" (по умолчанию ".")
4. Для file_write добавляй "content"

ФОРМАТ ОТВЕТА (строго JSON):
{{
  "reasoning": "Объяснение логики",
  "confidence": 0.9,
  "steps": [
    {{
      "tool": "file_read",
      "description": "Читаем файл X",
      "args": {{"path": "путь_к_файлу"}},
      "expected_output": "Содержимое файла"
    }}
  ]
}}

ПРИМЕРЫ:
"покажи файл main.rs" → tool: "file_read", args: {{"path": "main.rs"}}
"создай файл hello.txt с текстом привет" → tool: "file_write", args: {{"path": "hello.txt", "content": "привет"}}
"покажи содержимое папки src" → tool: "dir_list", args: {{"path": "src"}}

Отвечай ТОЛЬКО JSON!"#, tools_info)
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
        println!("[●] Ответ LLM: {}", response);
        
        // Пытаемся найти JSON в ответе
        let json_start = response.find('{').unwrap_or(0);
        let json_end = response.rfind('}').map(|i| i + 1).unwrap_or(response.len());
        let json_str = &response[json_start..json_end];
        
        println!("[●] Извлеченный JSON: {}", json_str);
        
        match serde_json::from_str::<ActionPlan>(json_str) {
            Ok(plan) => {
                println!("[✓] JSON успешно распарсен, {} шагов", plan.steps.len());
                Ok(plan)
            }
            Err(e) => {
                println!("[✗] Ошибка парсинга JSON: {}", e);
                Err(anyhow!("Ошибка парсинга JSON ответа: {}\nОтвет LLM: {}", e, response))
            }
        }
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