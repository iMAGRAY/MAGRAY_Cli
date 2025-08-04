use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::LlmClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPlan {
    pub steps: Vec<PlanStep>,
    pub reasoning: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub tool: String,
    pub description: String,
    pub parameters: HashMap<String, String>,
}

/// Агент для планирования сложных действий
pub struct ActionPlannerAgent {
    llm: LlmClient,
}

impl ActionPlannerAgent {
    pub fn new(llm: LlmClient) -> Self {
        Self { llm }
    }
    
    pub async fn create_plan(&self, user_query: &str, available_tools: &[String]) -> Result<ActionPlan> {
        let tools_list = available_tools.join(", ");
        
        let prompt = format!(r#"Ты - эксперт по планированию сложных действий. Проанализируй запрос пользователя и создай оптимальный пошаговый план выполнения.

ДОСТУПНЫЕ ИНСТРУМЕНТЫ: {}
ЗАПРОС ПОЛЬЗОВАТЕЛЯ: "{}"

ПРИНЦИПЫ ПЛАНИРОВАНИЯ:
1. Разбивай сложные задачи на атомарные операции
2. Учитывай зависимости между шагами
3. Используй только доступные инструменты
4. Каждый шаг должен иметь четкую цель и результат
5. Предусматривай проверку результатов

ТИПОВЫЕ СЦЕНАРИИ:

Работа с файлами:
- "создай и покажи файл" → 1) file_write 2) file_read
- "создай несколько файлов" → file_write для каждого файла
- "покажи содержимое папки и создай файл" → 1) dir_list 2) file_write

Git операции:
- "проверь статус и сделай коммит" → 1) git_status 2) git_commit
- "сделай коммит с проверкой" → 1) git_status 2) git_commit

Комплексные задачи:
- "найди информацию и сохрани в файл" → 1) web_search 2) file_write
- "выполни команду и сохрани результат" → 1) shell_exec 2) file_write
- "создай папку и проверь" → 1) shell_exec (mkdir) 2) shell_exec (dir)

ВАЖНО ДЛЯ WINDOWS КОМАНД:
- Используй Windows синтаксис: "mkdir", "dir", "del", "copy"
- Рабочий стол: %USERPROFILE%\Desktop (БЕЗ кавычек!)
- Переменные окружения НЕ заключай в кавычки: %USERPROFILE%, %USERNAME%
- Кавычки только для путей с пробелами: mkdir "My Folder"

ОПТИМИЗАЦИЯ:
- Объединяй похожие операции
- Избегай избыточных шагов
- Учитывай контекст предыдущих действий

Ответь ТОЛЬКО в формате JSON:
{{
    "steps": [
        {{
            "tool": "file_write",
            "description": "создаем конфигурационный файл",
            "parameters": {{"path": "config.json", "content": "{{\\"version\\": \\"1.0\\"}}"}}
        }},
        {{
            "tool": "file_read",
            "description": "проверяем созданный файл",
            "parameters": {{"path": "config.json"}}
        }}
    ],
    "reasoning": "план создан для выполнения запроса пользователя с проверкой результата",
    "confidence": 0.95
}}"#, tools_list, user_query);

        let response = self.llm.chat_simple(&prompt).await?;
        self.parse_action_plan(&response)
    }
    
    fn parse_action_plan(&self, response: &str) -> Result<ActionPlan> {
        let cleaned_response = response.trim();
        
        if let Some(json_start) = cleaned_response.find('{') {
            if let Some(json_end) = cleaned_response.rfind('}') {
                let json_str = &cleaned_response[json_start..=json_end];
                
                match serde_json::from_str::<ActionPlan>(json_str) {
                    Ok(plan) => return Ok(plan),
                    Err(e) => {
                        let fixed_json = self.fix_json_format(json_str);
                        return serde_json::from_str(&fixed_json)
                            .map_err(|_| anyhow!("Не удалось распарсить план действий: {}", e));
                    }
                }
            }
        }
        
        Err(anyhow!("Не найден валидный JSON в ответе: {}", response))
    }
    
    fn fix_json_format(&self, json_str: &str) -> String {
        json_str
            .replace("'", "\"")
            .replace("True", "true")
            .replace("False", "false")
            .replace(",}", "}")
            .replace(",]", "]")
            .replace("\\\"", "\"")  // Исправляем экранированные кавычки в JSON
    }
}