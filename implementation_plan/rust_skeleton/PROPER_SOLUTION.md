# Правильное архитектурное решение

## Проблема с текущим подходом

Текущее решение имеет фундаментальные архитектурные проблемы:

1. **Хрупкая эвристика** - жестко закодированные ключевые слова
2. **Дублирование кода** - SmartRouter создается в разных местах
3. **Нарушение SRP** - функции делают слишком много
4. **Плохая расширяемость** - сложно добавлять новые инструменты

## Правильное решение: Unified AI Agent

### 1. Создать единый AI Agent

```rust
// crates/cli/src/agent.rs
pub struct UnifiedAgent {
    llm_client: LlmClient,
    smart_router: SmartRouter,
}

impl UnifiedAgent {
    pub fn new(llm_client: LlmClient) -> Self {
        let smart_router = SmartRouter::new(llm_client.clone());
        Self { llm_client, smart_router }
    }
    
    pub async fn process_message(&self, message: &str) -> Result<AgentResponse> {
        // Всегда сначала спрашиваем у LLM, что делать
        let decision = self.analyze_intent(message).await?;
        
        match decision.action_type {
            ActionType::SimpleChat => {
                let response = self.llm_client.chat(message).await?;
                Ok(AgentResponse::Chat(response))
            }
            ActionType::UseTools => {
                let result = self.smart_router.process_smart_request(message).await?;
                Ok(AgentResponse::ToolExecution(result))
            }
        }
    }
    
    async fn analyze_intent(&self, message: &str) -> Result<IntentDecision> {
        let prompt = format!(
            "Analyze this user message and decide if it requires tools or just chat:
            
            Message: \"{}\"
            
            Respond with JSON:
            {{
                \"action_type\": \"simple_chat\" | \"use_tools\",
                \"reasoning\": \"explanation\",
                \"confidence\": 0.0-1.0
            }}",
            message
        );
        
        let response = self.llm_client.chat(&prompt).await?;
        let decision: IntentDecision = serde_json::from_str(&response)?;
        Ok(decision)
    }
}

#[derive(Deserialize)]
struct IntentDecision {
    action_type: ActionType,
    reasoning: String,
    confidence: f32,
}

#[derive(Deserialize)]
enum ActionType {
    #[serde(rename = "simple_chat")]
    SimpleChat,
    #[serde(rename = "use_tools")]
    UseTools,
}

pub enum AgentResponse {
    Chat(String),
    ToolExecution(String),
}
```

### 2. Упростить main.rs

```rust
// crates/cli/src/main.rs
async fn handle_chat(message: Option<String>) -> Result<()> {
    let llm_client = LlmClient::from_env()?;
    let agent = UnifiedAgent::new(llm_client);
    
    if let Some(msg) = message {
        process_single_message(&agent, &msg).await?;
    } else {
        run_interactive_chat(&agent).await?;
    }
    
    Ok(())
}

async fn process_single_message(agent: &UnifiedAgent, message: &str) -> Result<()> {
    let response = agent.process_message(message).await?;
    display_response(response).await;
    Ok(())
}

async fn run_interactive_chat(agent: &UnifiedAgent) -> Result<()> {
    loop {
        let input = read_user_input()?;
        if input == "exit" || input == "quit" {
            break;
        }
        
        let response = agent.process_message(&input).await?;
        display_response(response).await;
    }
    Ok(())
}

async fn display_response(response: AgentResponse) {
    match response {
        AgentResponse::Chat(text) => {
            display_chat_response(&text).await;
        }
        AgentResponse::ToolExecution(result) => {
            println!("{}", result);
        }
    }
}
```

### 3. Убрать дублирование команд

```rust
// Все команды используют один агент
match cli.command {
    Some(Commands::Chat { message }) => {
        handle_unified_chat(message).await?;
    }
    // Убираем отдельные команды Tool и Smart - они теперь не нужны
    Some(Commands::Read { path }) => {
        // Используем агент для чтения файлов
        let agent = create_agent().await?;
        let message = format!("прочитай файл {}", path);
        let response = agent.process_message(&message).await?;
        display_response(response).await;
    }
    // И так далее...
}
```

## Преимущества правильного решения:

### 1. **Умное принятие решений**
- LLM сам решает, нужны ли инструменты
- Нет жестко закодированных правил
- Легко адаптируется к новым типам запросов

### 2. **Единая точка входа**
- Все запросы обрабатываются одинаково
- Консистентное поведение
- Простота тестирования

### 3. **Расширяемость**
- Добавление новых инструментов не требует изменения логики
- LLM автоматически узнает о новых возможностях

### 4. **Чистая архитектура**
- Разделение ответственности
- Легко поддерживать и развивать
- Следует принципам SOLID

### 5. **Лучший UX**
- Пользователю не нужно думать о командах
- Естественное взаимодействие
- Интеллектуальная обработка запросов

## Миграционный план:

1. Создать `UnifiedAgent`
2. Рефакторить `handle_chat` для использования агента
3. Постепенно мигрировать другие команды
4. Убрать дублирующуюся логику
5. Добавить тесты для нового агента

Это решение масштабируется, легко поддерживается и предоставляет лучший пользовательский опыт.