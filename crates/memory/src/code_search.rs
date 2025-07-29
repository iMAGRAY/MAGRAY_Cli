use crate::{
    MemoryCoordinator, ChunkType,
    IngestionPipeline, IngestionConfig, IngestionEvent, MemSearchResult, ExecutionContext,
};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Результат поиска кода с контекстом
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSearchResult {
    /// Путь к файлу
    pub file_path: String,
    /// Язык программирования
    pub language: String,
    /// Тип сущности (функция, класс и т.д.)
    pub entity_type: String,
    /// Имя сущности
    pub entity_name: Option<String>,
    /// Начальная строка
    pub line_start: usize,
    /// Конечная строка
    pub line_end: usize,
    /// Фрагмент кода
    pub code_snippet: String,
    /// Контекст до фрагмента
    pub context_before: Option<String>,
    /// Контекст после фрагмента
    pub context_after: Option<String>,
    /// Оценка релевантности
    pub relevance_score: f32,
}

/// API для поиска кода с семантическим пониманием
pub struct CodeSearchAPI {
    memory: Arc<MemoryCoordinator>,
    indexed_paths: Vec<PathBuf>,
}

impl CodeSearchAPI {
    /// Создать новый API для поиска кода
    pub fn new(memory: Arc<MemoryCoordinator>) -> Self {
        Self {
            memory,
            indexed_paths: Vec::new(),
        }
    }
    
    /// Индексировать директорию с кодом
    pub async fn index_directory(&mut self, path: &Path) -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        
        // Создаем новый pipeline для каждой индексации
        let ingestion = IngestionPipeline::new(
            Arc::clone(&self.memory),
            IngestionConfig::default(),
        );
        
        let path = path.to_path_buf();
        let path_clone = path.clone();
        
        // Запускаем индексацию в отдельной задаче
        let ingestion_handle = tokio::spawn(async move {
            ingestion.ingest_directory(&path_clone, tx).await
        });
        
        // Обрабатываем события индексации
        while let Some(event) = rx.recv().await {
            match event {
                IngestionEvent::FileProcessed { path, chunks, .. } => {
                    tracing::info!("Indexed {} with {} chunks", path.display(), chunks);
                }
                IngestionEvent::FileSkipped { path, reason } => {
                    tracing::debug!("Skipped {}: {}", path.display(), reason);
                }
                IngestionEvent::Error { path, error } => {
                    tracing::warn!("Error indexing {}: {}", path.display(), error);
                }
                _ => {}
            }
        }
        
        // Ждем завершения индексации
        ingestion_handle.await??;
        self.indexed_paths.push(path);
        
        Ok(())
    }
    
    /// Поиск кода по семантическому запросу
    pub async fn search_code(
        &self,
        query: &str,
        top_k: usize,
        include_context: bool,
    ) -> Result<Vec<CodeSearchResult>> {
        let ctx = ExecutionContext::default();
        
        // Семантический поиск через memory coordinator
        let search_results = self.memory.semantic_search(query, top_k * 2, &ctx).await?;
        
        // Преобразуем результаты в CodeSearchResult
        let mut code_results = Vec::new();
        
        for result in search_results {
            if let Some(code_result) = self.convert_to_code_result(result, include_context).await? {
                code_results.push(code_result);
            }
            
            if code_results.len() >= top_k {
                break;
            }
        }
        
        Ok(code_results)
    }
    
    /// Поиск определения функции или класса
    pub async fn find_definition(
        &self,
        entity_name: &str,
        entity_type: Option<&str>,
    ) -> Result<Vec<CodeSearchResult>> {
        let query = match entity_type {
            Some(t) => format!("{} {} definition implementation", t, entity_name),
            None => format!("{} function class struct impl definition", entity_name),
        };
        
        let results = self.search_code(&query, 10, true).await?;
        
        // Фильтруем только точные совпадения имени
        Ok(results.into_iter()
            .filter(|r| {
                r.entity_name.as_ref()
                    .map(|n| n == entity_name || n.ends_with(&format!("::{}", entity_name)))
                    .unwrap_or(false)
            })
            .collect())
    }
    
    /// Поиск использований функции или типа
    pub async fn find_usages(
        &self,
        entity_name: &str,
        exclude_definitions: bool,
    ) -> Result<Vec<CodeSearchResult>> {
        let query = format!("{} usage call invoke new", entity_name);
        
        let mut results = self.search_code(&query, 50, false).await?;
        
        if exclude_definitions {
            results.retain(|r| {
                !matches!(r.entity_type.as_str(), "function" | "struct" | "class" | "impl")
                    || r.entity_name.as_ref().map(|n| n != entity_name).unwrap_or(true)
            });
        }
        
        Ok(results)
    }
    
    /// Поиск похожего кода (для рефакторинга)
    pub async fn find_similar_code(
        &self,
        code_snippet: &str,
        threshold: f32,
    ) -> Result<Vec<CodeSearchResult>> {
        // Используем сам код как запрос
        let results = self.search_code(code_snippet, 20, true).await?;
        
        // Фильтруем по порогу схожести
        Ok(results.into_iter()
            .filter(|r| r.relevance_score >= threshold)
            .collect())
    }
    
    /// Получить контекст для файла и позиции
    pub async fn get_file_context(
        &self,
        file_path: &Path,
        line: usize,
        context_lines: usize,
    ) -> Result<String> {
        let content = tokio::fs::read_to_string(file_path).await
            .context("Failed to read file")?;
        
        let lines: Vec<&str> = content.lines().collect();
        let start = line.saturating_sub(context_lines);
        let end = (line + context_lines).min(lines.len());
        
        let context = lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:4}: {}", start + i + 1, line))
            .collect::<Vec<_>>()
            .join("\n");
        
        Ok(context)
    }
    
    /// Построить граф зависимостей для сущности
    pub async fn build_dependency_graph(
        &self,
        entity_name: &str,
    ) -> Result<HashMap<String, Vec<String>>> {
        let mut graph = HashMap::new();
        let mut to_process = vec![entity_name.to_string()];
        let mut processed = std::collections::HashSet::new();
        
        while let Some(current) = to_process.pop() {
            if processed.contains(&current) {
                continue;
            }
            processed.insert(current.clone());
            
            // Находим определение
            let definitions = self.find_definition(&current, None).await?;
            
            for def in definitions {
                // Извлекаем зависимости из кода
                let deps = self.extract_dependencies(&def.code_snippet)?;
                
                graph.insert(current.clone(), deps.clone());
                
                // Добавляем новые зависимости для обработки
                for dep in deps {
                    if !processed.contains(&dep) {
                        to_process.push(dep);
                    }
                }
            }
        }
        
        Ok(graph)
    }
    
    /// Преобразовать результат поиска в CodeSearchResult
    async fn convert_to_code_result(
        &self,
        search_result: MemSearchResult,
        include_context: bool,
    ) -> Result<Option<CodeSearchResult>> {
        // Извлекаем метаданные из MemMeta
        let chunk_type = search_result.meta.extra.get("chunk_type")
            .and_then(|v| serde_json::from_value::<ChunkType>(v.clone()).ok());
        
        if let Some(ChunkType::Code { language, entity_type, file_path, line_start, line_end }) = chunk_type {
            let entity_name = search_result.meta.extra.get("entity_name")
                .and_then(|v| v.as_str())
                .map(String::from);
            
            let code_snippet = search_result.snippet.unwrap_or_default();
            
            let (context_before, context_after) = if include_context {
                let path = Path::new(&file_path);
                if path.exists() {
                    let before = self.get_file_context(path, line_start, 3).await.ok();
                    let after = self.get_file_context(path, line_end, 3).await.ok();
                    (before, after)
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };
            
            Ok(Some(CodeSearchResult {
                file_path,
                language,
                entity_type: format!("{:?}", entity_type),
                entity_name,
                line_start,
                line_end,
                code_snippet,
                context_before,
                context_after,
                relevance_score: search_result.score,
            }))
        } else {
            Ok(None)
        }
    }
    
    /// Извлечь зависимости из фрагмента кода
    fn extract_dependencies(&self, code: &str) -> Result<Vec<String>> {
        let mut deps = Vec::new();
        
        // Простой парсер для Rust
        // В реальности здесь должен быть полноценный AST парсер
        for line in code.lines() {
            let line = line.trim();
            
            // use statements
            if line.starts_with("use ") {
                if let Some(path) = line.strip_prefix("use ")
                    .and_then(|s| s.strip_suffix(";"))
                    .map(|s| s.trim()) {
                    deps.push(path.to_string());
                }
            }
            
            // Function calls (простой паттерн)
            if let Some(pos) = line.find('(') {
                if pos > 0 {
                    let func_part = &line[..pos];
                    if let Some(func_name) = func_part.split_whitespace().last() {
                        if !func_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                            continue;
                        }
                        deps.push(func_name.to_string());
                    }
                }
            }
        }
        
        deps.sort();
        deps.dedup();
        
        Ok(deps)
    }
}

/// Интерактивный построитель запросов для поиска кода
pub struct CodeQueryBuilder {
    query_parts: Vec<String>,
    filters: HashMap<String, String>,
}

impl CodeQueryBuilder {
    pub fn new() -> Self {
        Self {
            query_parts: Vec::new(),
            filters: HashMap::new(),
        }
    }
    
    /// Добавить текстовую часть запроса
    pub fn with_text(mut self, text: &str) -> Self {
        self.query_parts.push(text.to_string());
        self
    }
    
    /// Фильтр по языку программирования
    pub fn language(mut self, lang: &str) -> Self {
        self.filters.insert("language".to_string(), lang.to_string());
        self
    }
    
    /// Фильтр по типу сущности
    pub fn entity_type(mut self, entity_type: &str) -> Self {
        self.filters.insert("entity_type".to_string(), entity_type.to_string());
        self
    }
    
    /// Фильтр по пути файла
    pub fn in_path(mut self, path_pattern: &str) -> Self {
        self.filters.insert("path".to_string(), path_pattern.to_string());
        self
    }
    
    /// Построить финальный запрос
    pub fn build(self) -> String {
        let mut query = self.query_parts.join(" ");
        
        // Добавляем фильтры к запросу
        for (key, value) in self.filters {
            query.push_str(&format!(" {}:{}", key, value));
        }
        
        query
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_code_query_builder() {
        let query = CodeQueryBuilder::new()
            .with_text("find all async functions")
            .language("rust")
            .entity_type("function")
            .in_path("src/")
            .build();
        
        assert!(query.contains("find all async functions"));
        assert!(query.contains("language:rust"));
        assert!(query.contains("entity_type:function"));
        assert!(query.contains("path:src/"));
    }
}