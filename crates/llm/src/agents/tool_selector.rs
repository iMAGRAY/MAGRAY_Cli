use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use crate::LlmClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSelection {
    pub tool_name: String,
    pub confidence: f32,
    pub reasoning: String,
}

/// Агент для выбора подходящего инструмента
pub struct ToolSelectorAgent {
    llm: LlmClient,
}

impl ToolSelectorAgent {
    pub fn new(llm: LlmClient) -> Self {
        Self { llm }
    }
    
    pub async fn select_tool(&self, user_query: &str, available_tools: &[String]) -> Result<ToolSelection> {
        let tools_list = available_tools.join(", ");
        
        let prompt = format!(r#"Ты - эксперт по выбору инструментов. Проанализируй запрос пользователя и выбери наиболее подходящий инструмент.

ДОСТУПНЫЕ ИНСТРУМЕНТЫ: {tools_list}

ЗАПРОС ПОЛЬЗОВАТЕЛЯ: "{user_query}"

АНАЛИЗ ПО КЛЮЧЕВЫМ СЛОВАМ:
- "файл", "создать", "записать", "сохранить" → file_write
- "прочитать", "показать", "открыть", "содержимое" → file_read  
- "папка", "директория", "список", "ls", "dir" → dir_list
- "команда", "выполнить", "запустить", "shell" → shell_exec
- "git", "коммит", "commit" → git_commit или git_status
- "поиск", "найти", "search", "гугл" → web_search

ПРАВИЛА:
- Выбери ТОЛЬКО один инструмент из списка
- Учитывай контекст и намерения пользователя
- Если сомневаешься, выбери наиболее вероятный вариант
- Оцени уверенность от 0.0 до 1.0

Ответь ТОЛЬКО в формате JSON:
{{
    "tool_name": "название_инструмента",
    "confidence": 0.9,
    "reasoning": "краткое объяснение выбора"
}}"#);

        let response = self.llm.chat_simple(&prompt).await?;
        self.parse_tool_selection(&response)
    }
    
    fn parse_tool_selection(&self, response: &str) -> Result<ToolSelection> {
        // Улучшенный парсинг JSON с обработкой ошибок
        let cleaned_response = response.trim();
        
        // Ищем JSON блок
        if let Some(json_start) = cleaned_response.find('{') {
            if let Some(json_end) = cleaned_response.rfind('}') {
                let json_str = &cleaned_response[json_start..=json_end];
                
                match serde_json::from_str::<ToolSelection>(json_str) {
                    Ok(selection) => return Ok(selection),
                    Err(e) => {
                        // Попробуем исправить распространенные ошибки JSON
                        let fixed_json = self.fix_json_format(json_str);
                        return serde_json::from_str(&fixed_json)
                            .map_err(|_| anyhow!("Не удалось распарсить ответ LLM: {}", e));
                    }
                }
            }
        }
        
        Err(anyhow!("Не найден валидный JSON в ответе: {}", response))
    }
    
    fn fix_json_format(&self, json_str: &str) -> String {
        json_str
            .replace("'", "\"")  // Заменяем одинарные кавычки на двойные
            .replace("True", "true")  // Python-style boolean
            .replace("False", "false")
            .replace(",}", "}")  // Убираем лишние запятые
            .replace(",]", "]")
    }
}