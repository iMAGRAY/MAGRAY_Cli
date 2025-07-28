use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;


pub mod file_ops;
pub mod git_ops;
pub mod web_ops;
pub mod shell_ops;
pub mod ai_router;

// Экспорты для внешнего использования
pub use ai_router::{SmartRouter, ActionPlan, PlannedAction};

// Базовые типы для системы инструментов
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInput {
    pub command: String,
    pub args: HashMap<String, String>,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    pub success: bool,
    pub result: String,
    pub formatted_output: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    pub usage: String,
    pub examples: Vec<String>,
    pub input_schema: String,
}

// Трейт для всех инструментов
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    fn spec(&self) -> ToolSpec;
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput>;
    fn supports_natural_language(&self) -> bool { true }
    async fn parse_natural_language(&self, query: &str) -> Result<ToolInput>;
}

// Реестр инструментов
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            tools: HashMap::new(),
        };
        
        // Регистрируем базовые инструменты
        registry.register("file_read", Box::new(file_ops::FileReader::new()));
        registry.register("file_write", Box::new(file_ops::FileWriter::new()));
        registry.register("dir_list", Box::new(file_ops::DirLister::new()));
        registry.register("git_status", Box::new(git_ops::GitStatus::new()));
        registry.register("git_commit", Box::new(git_ops::GitCommit::new()));
        registry.register("web_search", Box::new(web_ops::WebSearch::new()));
        registry.register("shell_exec", Box::new(shell_ops::ShellExec::new()));
        
        registry
    }
    
    pub fn register(&mut self, name: &str, tool: Box<dyn Tool>) {
        self.tools.insert(name.to_string(), tool);
    }
    
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }
    
    pub fn list_tools(&self) -> Vec<ToolSpec> {
        self.tools.values().map(|tool| tool.spec()).collect()
    }
    
    // Умный поиск инструмента по описанию
    pub async fn find_tool_for_query(&self, query: &str) -> Option<&dyn Tool> {
        // Простая эвристика для выбора инструмента
        let query_lower = query.to_lowercase();
        
        if query_lower.contains("read") || query_lower.contains("show") || query_lower.contains("cat") {
            return self.get("file_read");
        }
        if query_lower.contains("write") || query_lower.contains("create") || query_lower.contains("save") {
            return self.get("file_write");  
        }
        if query_lower.contains("list") || query_lower.contains("ls") || query_lower.contains("directory") {
            return self.get("dir_list");
        }
        if query_lower.contains("git status") || query_lower.contains("git st") {
            return self.get("git_status");
        }
        if query_lower.contains("commit") || query_lower.contains("git commit") {
            return self.get("git_commit");
        }
        if query_lower.contains("search") || query_lower.contains("google") || query_lower.contains("find") {
            return self.get("web_search");
        }
        if query_lower.contains("run") || query_lower.contains("execute") || query_lower.contains("shell") {
            return self.get("shell_exec");
        }
        
        None
    }
    
    // Выполнение инструмента с естественным языком
    pub async fn execute_natural(&self, query: &str) -> Result<ToolOutput> {
        if let Some(tool) = self.find_tool_for_query(query).await {
            let input = tool.parse_natural_language(query).await?;
            tool.execute(input).await
        } else {
            Ok(ToolOutput {
                success: false,
                result: format!("Не удалось найти подходящий инструмент для запроса: {}", query),
                formatted_output: None,
                metadata: HashMap::new(),
            })
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}