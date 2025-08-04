use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::LlmClient;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterExtraction {
    pub parameters: HashMap<String, String>,
    pub confidence: f32,
    pub missing_params: Vec<String>,
}

/// Агент для извлечения параметров из естественного языка
pub struct ParameterExtractorAgent {
    llm: LlmClient,
}

impl ParameterExtractorAgent {
    pub fn new(llm: LlmClient) -> Self {
        Self { llm }
    }
    
    pub async fn extract_parameters(
        &self, 
        user_query: &str, 
        tool_name: &str,
        required_params: &[String]
    ) -> Result<ParameterExtraction> {
        let params_list = required_params.join(", ");
        
        let prompt = format!(r#"Ты - эксперт по извлечению параметров. Проанализируй запрос пользователя и извлеки все необходимые параметры для инструмента.

<<<<<<< HEAD
ИНСТРУМЕНТ: {tool_name}
ТРЕБУЕМЫЕ ПАРАМЕТРЫ: {params_list}
ЗАПРОС ПОЛЬЗОВАТЕЛЯ: "{user_query}"
=======
ИНСТРУМЕНТ: {}
ТРЕБУЕМЫЕ ПАРАМЕТРЫ: {}
ЗАПРОС ПОЛЬЗОВАТЕЛЯ: "{}"
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c

ДЕТАЛЬНЫЕ ПРАВИЛА ИЗВЛЕЧЕНИЯ:

file_read:
- "path": путь к файлу (обязательно)
- Примеры: "прочитай main.rs", "покажи содержимое config.json"

file_write:
- "path": путь к файлу (обязательно)
- "content": содержимое файла (если не указано, создай подходящее)
- Примеры: "создай файл test.txt с текстом hello", "сохрани в config.json настройки"

dir_list:
- "path": путь к папке (по умолчанию "." для текущей папки)
- Примеры: "покажи файлы в src", "список файлов"

shell_exec:
- "command": команда для выполнения (обязательно)
- ВАЖНО: Адаптируй команды под операционную систему
- Windows: используй "mkdir", "dir", "del", "copy"
- Unix: используй "mkdir", "ls", "rm", "cp"
- Примеры: "создай папку test" → Windows: "mkdir test", Unix: "mkdir test"
- Примеры: "покажи файлы" → Windows: "dir", Unix: "ls"

git_commit:
- "message": сообщение коммита (обязательно)
- Примеры: "сделай коммит с сообщением fix bug", "коммит добавил новую функцию"

git_status:
- Параметры не требуются

web_search:
- "query": поисковый запрос (обязательно)
- Примеры: "найди информацию о Rust", "поиск best practices"

УМНАЯ ОБРАБОТКА:
- Если файл без расширения и контекст подсказывает тип, добавь расширение
- Если содержимое файла не указано, создай осмысленное по контексту запроса
- Если путь не указан явно, попробуй извлечь из контекста или используй разумные значения по умолчанию

КРИТИЧЕСКИ ВАЖНО ДЛЯ КОМАНД SHELL:
- Система: Windows (используй Windows команды!)
- Создание папки: "mkdir имя_папки" (НЕ "mkdir -p")
- Просмотр файлов: "dir" (НЕ "ls")
- Удаление файла: "del файл" (НЕ "rm")
- Копирование: "copy источник назначение" (НЕ "cp")
- Путь к рабочему столу: %USERPROFILE%\Desktop (БЕЗ кавычек в переменных!)
- Пример: "создай папку 242424 на рабочем столе" → "mkdir %USERPROFILE%\Desktop\242424"
- ВАЖНО: НЕ используй кавычки вокруг переменных окружения %USERPROFILE%
- Проверка папки: "dir %USERPROFILE%\Desktop" для просмотра рабочего стола

Ответь ТОЛЬКО в формате JSON:
{{
    "parameters": {{"param1": "value1", "param2": "value2"}},
    "confidence": 0.9,
    "missing_params": ["param3"]
<<<<<<< HEAD
}}"#);

        let response = self.llm.chat_simple(&prompt).await?;
=======
}}"#, tool_name, params_list, user_query);

        let response = self.llm.chat(&prompt).await?;
>>>>>>> cdac5c55f689e319aa18d538b93d7c8f8759a52c
        self.parse_parameter_extraction(&response)
    }
    
    fn parse_parameter_extraction(&self, response: &str) -> Result<ParameterExtraction> {
        let cleaned_response = response.trim();
        
        if let Some(json_start) = cleaned_response.find('{') {
            if let Some(json_end) = cleaned_response.rfind('}') {
                let json_str = &cleaned_response[json_start..=json_end];
                
                match serde_json::from_str::<ParameterExtraction>(json_str) {
                    Ok(extraction) => return Ok(extraction),
                    Err(e) => {
                        let fixed_json = self.fix_json_format(json_str);
                        return serde_json::from_str(&fixed_json)
                            .map_err(|_| anyhow!("Не удалось распарсить параметры: {}", e));
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