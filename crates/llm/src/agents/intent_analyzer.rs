use crate::LlmClient;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentDecision {
    #[serde(alias = "intent_type")]
    pub action_type: String, // "chat" или "tools"
    pub confidence: f32,
    #[serde(default)]
    pub reasoning: String,
}

/// Агент для принятия решений о типе действия
pub struct IntentAnalyzerAgent {
    llm: LlmClient,
}

impl IntentAnalyzerAgent {
    pub fn new(llm: LlmClient) -> Self {
        Self { llm }
    }

    pub async fn analyze_intent(&self, user_query: &str) -> Result<IntentDecision> {
        let prompt = format!(
            r#"Ты - эксперт по анализу намерений пользователя. Определи, требует ли запрос использования инструментов или достаточно обычного чата.

ЗАПРОС ПОЛЬЗОВАТЕЛЯ: "{user_query}"

ДЕТАЛЬНЫЕ КРИТЕРИИ ДЛЯ "tools":

Работа с файлами:
- "создай файл", "сохрани в файл", "запиши в"
- "прочитай файл", "покажи содержимое", "открой"
- "файл", "document", "script"

Операции с папками:
- "покажи файлы", "список файлов", "что в папке"
- "директория", "folder", "ls", "dir"

Git операции:
- "git status", "статус git", "изменения"
- "сделай коммит", "commit", "зафиксируй"

Выполнение команд:
- "выполни команду", "запусти", "shell"
- "cargo build", "npm install", конкретные команды

Поиск в интернете:
- "найди в интернете", "поищи", "search"
- "что такое", "как работает" (если нужна актуальная информация)

КРИТЕРИИ ДЛЯ "chat":
- Вопросы о концепциях программирования
- Объяснения алгоритмов и паттернов
- Просьбы о совете или рекомендациях
- Обсуждение архитектуры
- Общие вопросы без конкретных действий

ПРИМЕРЫ:
- "создай файл main.rs" → tools (file_write)
- "что такое ownership в Rust?" → chat (объяснение концепции)
- "покажи статус git" → tools (git_status)
- "как лучше структурировать проект?" → chat (совет)

Ответь ТОЛЬКО в формате JSON:
{{
    "action_type": "tools",
    "confidence": 0.9,
    "reasoning": "пользователь просит создать файл"
}}"#
        );

        let response = self.llm.chat_simple(&prompt).await?;
        self.parse_intent_decision(&response)
    }

    fn parse_intent_decision(&self, response: &str) -> Result<IntentDecision> {
        let cleaned_response = response.trim();

        if let Some(json_start) = cleaned_response.find('{') {
            if let Some(json_end) = cleaned_response.rfind('}') {
                let json_str = &cleaned_response[json_start..=json_end];

                match serde_json::from_str::<IntentDecision>(json_str) {
                    Ok(decision) => return Ok(decision),
                    Err(e) => {
                        let fixed_json = self.fix_json_format(json_str);
                        return serde_json::from_str(&fixed_json)
                            .map_err(|_| anyhow!("Не удалось распарсить намерение: {}", e));
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
    }
}
