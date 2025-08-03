use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn, error};
use uuid::Uuid;

use crate::{
    MemoryService, Layer, Record,
    types::SearchOptions,
};

/// Streaming API для real-time обработки embeddings
/// @component: {"k":"C","id":"streaming_api","t":"Real-time memory processing","m":{"cur":95,"tgt":100,"u":"%"},"f":["streaming","real-time","async"]}
pub struct StreamingMemoryAPI {
    service: Arc<MemoryService>,
    /// Активные streaming sessions
    sessions: Arc<RwLock<std::collections::HashMap<String, StreamingSession>>>,
    /// Конфигурация streaming
    config: StreamingConfig,
}

/// Конфигурация streaming API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Максимальное количество активных sessions
    pub max_concurrent_sessions: usize,
    /// Размер буфера для batch обработки
    pub buffer_size: usize,
    /// Таймаут для flush буфера
    pub flush_timeout_ms: u64,
    /// Максимальный размер одного сообщения
    pub max_message_size: usize,
    /// Включить auto-promotion в streaming режиме
    pub enable_auto_promotion: bool,
    /// Интервал для ML promotion в streaming
    pub promotion_interval_sec: u64,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            max_concurrent_sessions: 100,
            buffer_size: 50,
            flush_timeout_ms: 1000,
            max_message_size: 1024 * 1024, // 1MB
            enable_auto_promotion: true,
            promotion_interval_sec: 30,
        }
    }
}

/// Активная streaming session
#[derive(Debug)]
struct StreamingSession {
    id: String,
    created_at: Instant,
    last_activity: Instant,
    /// Буфер для batch обработки
    buffer: Vec<StreamingRequest>,
    /// Канал для отправки результатов
    result_sender: mpsc::UnboundedSender<StreamingResponse>,
    /// Статистика session
    stats: StreamingStats,
    /// Конфигурация конкретной session
    session_config: SessionConfig,
}

/// Конфигурация конкретной session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub target_layer: Layer,
    pub enable_search: bool,
    pub enable_ml_promotion: bool,
    pub auto_flush: bool,
    pub priority: StreamingPriority,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            target_layer: Layer::Interact,
            enable_search: true,
            enable_ml_promotion: true,
            auto_flush: true,
            priority: StreamingPriority::Normal,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StreamingPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Запрос в streaming API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingRequest {
    pub request_id: String,
    pub session_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub operation: StreamingOperation,
}

/// Типы операций в streaming API
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamingOperation {
    /// Вставка новой записи
    Insert {
        text: String,
        layer: Option<Layer>,
        tags: Vec<String>,
        project: Option<String>,
    },
    /// Поиск по тексту
    Search {
        query: String,
        options: SearchOptions,
    },
    /// Batch операция
    BatchInsert {
        records: Vec<StreamingInsertRecord>,
    },
    /// Управление session
    SessionControl {
        action: SessionAction,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingInsertRecord {
    pub text: String,
    pub layer: Option<Layer>,
    pub tags: Vec<String>,
    pub project: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionAction {
    Flush,
    SetConfig(SessionConfig),
    GetStats,
    Close,
}

/// Ответ от streaming API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingResponse {
    pub request_id: String,
    pub session_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub result: StreamingResult,
    pub processing_time_ms: u64,
}

/// Результат операции
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StreamingResult {
    Success {
        operation_id: String,
        details: serde_json::Value,
    },
    Error {
        error_code: String,
        message: String,
    },
    InsertResult {
        record_id: Uuid,
        layer: Layer,
        embedding_time_ms: u64,
    },
    SearchResult {
        results: Vec<StreamingSearchResult>,
        total_found: usize,
        search_time_ms: u64,
    },
    BatchResult {
        inserted_count: usize,
        failed_count: usize,
        batch_time_ms: u64,
    },
    SessionStats {
        stats: StreamingStats,
    },
    PromotionResult {
        promoted_records: usize,
        promotion_time_ms: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingSearchResult {
    pub record_id: Uuid,
    pub text: String,
    pub layer: Layer,
    pub score: f32,
    pub tags: Vec<String>,
}

/// Статистика streaming session
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StreamingStats {
    pub total_requests: u64,
    pub successful_operations: u64,
    pub failed_operations: u64,
    pub total_inserts: u64,
    pub total_searches: u64,
    pub total_batch_operations: u64,
    pub avg_processing_time_ms: f64,
    pub buffer_utilization: f64,
    pub session_duration_sec: u64,
    pub last_promotion_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl StreamingMemoryAPI {
    /// Создать новый streaming API
    pub async fn new(service: Arc<MemoryService>, config: StreamingConfig) -> Result<Self> {
        info!("🌊 Инициализация Streaming Memory API");
        info!("  - Max concurrent sessions: {}", config.max_concurrent_sessions);
        info!("  - Buffer size: {}", config.buffer_size);
        info!("  - Flush timeout: {}ms", config.flush_timeout_ms);
        info!("  - Auto promotion: {}", config.enable_auto_promotion);

        let api = Self {
            service,
            sessions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            config,
        };

        // Запускаем фоновые задачи
        api.start_background_tasks().await?;

        Ok(api)
    }

    /// Создать новую streaming session
    pub async fn create_session(
        &self,
        session_id: String,
        session_config: SessionConfig,
    ) -> Result<mpsc::UnboundedReceiver<StreamingResponse>> {
        let mut sessions = self.sessions.write().await;

        // Проверяем лимит sessions
        if sessions.len() >= self.config.max_concurrent_sessions {
            return Err(anyhow::anyhow!(
                "Maximum concurrent sessions limit reached: {}",
                self.config.max_concurrent_sessions
            ));
        }

        // Проверяем что session не существует
        if sessions.contains_key(&session_id) {
            return Err(anyhow::anyhow!("Session already exists: {}", session_id));
        }

        let (result_sender, result_receiver) = mpsc::unbounded_channel();

        let session = StreamingSession {
            id: session_id.clone(),
            created_at: Instant::now(),
            last_activity: Instant::now(),
            buffer: Vec::with_capacity(self.config.buffer_size),
            result_sender,
            stats: StreamingStats::default(),
            session_config,
        };

        sessions.insert(session_id.clone(), session);

        info!("✅ Created streaming session: {}", session_id);
        Ok(result_receiver)
    }

    /// Обработать streaming запрос
    pub async fn process_request(&self, request: StreamingRequest) -> Result<()> {
        let start_time = Instant::now();

        debug!("📨 Processing streaming request: {} for session {}", 
               request.request_id, request.session_id);

        // Валидация размера сообщения
        let request_size = serde_json::to_string(&request)?.len();
        if request_size > self.config.max_message_size {
            self.send_error_response(
                &request.request_id,
                &request.session_id,
                "MESSAGE_TOO_LARGE",
                &format!("Message size {} exceeds limit {}", request_size, self.config.max_message_size),
                start_time,
            ).await?;
            return Ok(());
        }

        let mut sessions = self.sessions.write().await;
        let session = match sessions.get_mut(&request.session_id) {
            Some(s) => s,
            None => {
                warn!("Session not found: {}", request.session_id);
                return Err(anyhow::anyhow!("Session not found: {}", request.session_id));
            }
        };

        session.last_activity = Instant::now();
        session.stats.total_requests += 1;

        // Обрабатываем запрос
        let result = match self.handle_operation(&request.operation, session).await {
            Ok(result) => {
                session.stats.successful_operations += 1;
                result
            }
            Err(e) => {
                session.stats.failed_operations += 1;
                error!("Failed to process streaming request: {}", e);
                StreamingResult::Error {
                    error_code: "PROCESSING_ERROR".to_string(),
                    message: e.to_string(),
                }
            }
        };

        // Отправляем ответ
        let processing_time = start_time.elapsed().as_millis() as u64;
        session.stats.avg_processing_time_ms = 
            (session.stats.avg_processing_time_ms * (session.stats.total_requests - 1) as f64 + processing_time as f64) / session.stats.total_requests as f64;

        let response = StreamingResponse {
            request_id: request.request_id,
            session_id: request.session_id,
            timestamp: chrono::Utc::now(),
            result,
            processing_time_ms: processing_time,
        };

        if let Err(e) = session.result_sender.send(response) {
            warn!("Failed to send streaming response: {}", e);
        }

        Ok(())
    }

    /// Обработка конкретной операции
    async fn handle_operation(
        &self,
        operation: &StreamingOperation,
        session: &mut StreamingSession,
    ) -> Result<StreamingResult> {
        match operation {
            StreamingOperation::Insert { text, layer, tags, project } => {
                self.handle_insert(text, layer, tags, project, session).await
            }
            StreamingOperation::Search { query, options } => {
                self.handle_search(query, options, session).await
            }
            StreamingOperation::BatchInsert { records } => {
                self.handle_batch_insert(records, session).await
            }
            StreamingOperation::SessionControl { action } => {
                self.handle_session_control(action, session).await
            }
        }
    }

    /// Обработка вставки записи
    async fn handle_insert(
        &self,
        text: &str,
        layer: &Option<Layer>,
        tags: &[String],
        project: &Option<String>,
        session: &mut StreamingSession,
    ) -> Result<StreamingResult> {
        let start_time = Instant::now();

        let record = Record {
            id: Uuid::new_v4(),
            text: text.to_string(),
            embedding: vec![], // Будет заполнено автоматически
            layer: layer.unwrap_or(session.session_config.target_layer),
            kind: "text".to_string(),
            tags: tags.to_vec(),
            project: project.clone().unwrap_or_else(|| "streaming".to_string()),
            session: session.id.clone(),
            score: 0.5,
            access_count: 1,
            ts: chrono::Utc::now(),
            last_access: chrono::Utc::now(),
        };

        self.service.insert(record.clone()).await?;
        session.stats.total_inserts += 1;

        let embedding_time = start_time.elapsed().as_millis() as u64;

        Ok(StreamingResult::InsertResult {
            record_id: record.id,
            layer: record.layer,
            embedding_time_ms: embedding_time,
        })
    }

    /// Обработка поиска
    async fn handle_search(
        &self,
        query: &str,
        options: &SearchOptions,
        session: &mut StreamingSession,
    ) -> Result<StreamingResult> {
        if !session.session_config.enable_search {
            return Ok(StreamingResult::Error {
                error_code: "SEARCH_DISABLED".to_string(),
                message: "Search is disabled for this session".to_string(),
            });
        }

        let start_time = Instant::now();

        let mut search_builder = self.service.search(query);
        search_builder = search_builder.with_layers(&options.layers);
        search_builder = search_builder.top_k(options.top_k);

        let records = search_builder.execute().await?;
        session.stats.total_searches += 1;

        let search_time = start_time.elapsed().as_millis() as u64;

        let results: Vec<StreamingSearchResult> = records
            .into_iter()
            .map(|r| StreamingSearchResult {
                record_id: r.id,
                text: r.text,
                layer: r.layer,
                score: r.score,
                tags: r.tags,
            })
            .collect();

        let total_found = results.len();

        Ok(StreamingResult::SearchResult {
            results,
            total_found,
            search_time_ms: search_time,
        })
    }

    /// Обработка batch вставки
    async fn handle_batch_insert(
        &self,
        records: &[StreamingInsertRecord],
        session: &mut StreamingSession,
    ) -> Result<StreamingResult> {
        let start_time = Instant::now();

        let mut inserted_count = 0;
        let mut failed_count = 0;

        for insert_record in records {
            let record = Record {
                id: Uuid::new_v4(),
                text: insert_record.text.clone(),
                embedding: vec![],
                layer: insert_record.layer.unwrap_or(session.session_config.target_layer),
                kind: "text".to_string(),
                tags: insert_record.tags.clone(),
                project: insert_record.project.clone().unwrap_or_else(|| "streaming".to_string()),
                session: session.id.clone(),
                score: 0.5,
                access_count: 1,
                ts: chrono::Utc::now(),
                last_access: chrono::Utc::now(),
            };

            match self.service.insert(record).await {
                Ok(_) => inserted_count += 1,
                Err(e) => {
                    failed_count += 1;
                    warn!("Failed to insert record in batch: {}", e);
                }
            }
        }

        session.stats.total_batch_operations += 1;
        session.stats.total_inserts += inserted_count as u64;

        let batch_time = start_time.elapsed().as_millis() as u64;

        Ok(StreamingResult::BatchResult {
            inserted_count,
            failed_count,
            batch_time_ms: batch_time,
        })
    }

    /// Обработка управления session
    async fn handle_session_control(
        &self,
        action: &SessionAction,
        session: &mut StreamingSession,
    ) -> Result<StreamingResult> {
        match action {
            SessionAction::Flush => {
                // Принудительный flush буфера
                debug!("Flushing session buffer: {}", session.id);
                session.buffer.clear();
                Ok(StreamingResult::Success {
                    operation_id: "flush".to_string(),
                    details: serde_json::json!({"message": "Buffer flushed"}),
                })
            }
            SessionAction::SetConfig(new_config) => {
                session.session_config = new_config.clone();
                Ok(StreamingResult::Success {
                    operation_id: "set_config".to_string(),
                    details: serde_json::json!({"message": "Configuration updated"}),
                })
            }
            SessionAction::GetStats => {
                session.stats.session_duration_sec = session.created_at.elapsed().as_secs();
                Ok(StreamingResult::SessionStats {
                    stats: session.stats.clone(),
                })
            }
            SessionAction::Close => {
                Ok(StreamingResult::Success {
                    operation_id: "close".to_string(),
                    details: serde_json::json!({"message": "Session will be closed"}),
                })
            }
        }
    }

    /// Отправка error response
    async fn send_error_response(
        &self,
        request_id: &str,
        session_id: &str,
        error_code: &str,
        message: &str,
        start_time: Instant,
    ) -> Result<()> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.get(session_id) {
            let response = StreamingResponse {
                request_id: request_id.to_string(),
                session_id: session_id.to_string(),
                timestamp: chrono::Utc::now(),
                result: StreamingResult::Error {
                    error_code: error_code.to_string(),
                    message: message.to_string(),
                },
                processing_time_ms: start_time.elapsed().as_millis() as u64,
            };

            if let Err(e) = session.result_sender.send(response) {
                warn!("Failed to send error response: {}", e);
            }
        }
        Ok(())
    }

    /// Закрыть session
    pub async fn close_session(&self, session_id: &str) -> Result<StreamingStats> {
        let mut sessions = self.sessions.write().await;
        
        match sessions.remove(session_id) {
            Some(mut session) => {
                session.stats.session_duration_sec = session.created_at.elapsed().as_secs();
                info!("🔒 Closed streaming session: {} (duration: {}s)", 
                      session_id, session.stats.session_duration_sec);
                Ok(session.stats)
            }
            None => Err(anyhow::anyhow!("Session not found: {}", session_id))
        }
    }

    /// Получить статистику всех sessions
    pub async fn get_global_stats(&self) -> Result<GlobalStreamingStats> {
        let sessions = self.sessions.read().await;
        
        let mut stats = GlobalStreamingStats {
            active_sessions: sessions.len(),
            total_requests: 0,
            total_inserts: 0,
            total_searches: 0,
            total_batch_operations: 0,
            avg_processing_time_ms: 0.0,
            oldest_session_age_sec: 0,
        };

        let mut total_processing_time = 0.0;
        let mut oldest_session_time: Option<Instant> = None;

        for session in sessions.values() {
            stats.total_requests += session.stats.total_requests;
            stats.total_inserts += session.stats.total_inserts;
            stats.total_searches += session.stats.total_searches;
            stats.total_batch_operations += session.stats.total_batch_operations;
            total_processing_time += session.stats.avg_processing_time_ms * session.stats.total_requests as f64;

            match oldest_session_time {
                None => oldest_session_time = Some(session.created_at),
                Some(oldest) => {
                    if session.created_at < oldest {
                        oldest_session_time = Some(session.created_at);
                    }
                }
            }
        }

        if stats.total_requests > 0 {
            stats.avg_processing_time_ms = total_processing_time / stats.total_requests as f64;
        }

        if let Some(oldest) = oldest_session_time {
            stats.oldest_session_age_sec = oldest.elapsed().as_secs();
        }

        Ok(stats)
    }

    /// Запуск фоновых задач
    async fn start_background_tasks(&self) -> Result<()> {
        let sessions_clone = Arc::clone(&self.sessions);
        let service_clone = Arc::clone(&self.service);
        let config = self.config.clone();

        // Задача для автоматической очистки неактивных sessions
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                
                let mut sessions = sessions_clone.write().await;
                let mut to_remove = Vec::new();
                
                for (session_id, session) in sessions.iter() {
                    // Удаляем sessions неактивные более 1 часа
                    if session.last_activity.elapsed() > Duration::from_secs(3600) {
                        to_remove.push(session_id.clone());
                    }
                }
                
                for session_id in to_remove {
                    if let Some(session) = sessions.remove(&session_id) {
                        info!("🧹 Removed inactive session: {} (inactive for {}s)", 
                              session_id, session.last_activity.elapsed().as_secs());
                    }
                }
            }
        });

        // Задача для автоматического ML promotion
        if config.enable_auto_promotion {
            let service_for_promotion = Arc::clone(&service_clone);
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(config.promotion_interval_sec));
                loop {
                    interval.tick().await;
                    
                    // Используем standard promotion вместо ML для избежания Send проблем
                    match service_for_promotion.run_promotion_cycle().await {
                        Ok(stats) => {
                            if stats.interact_to_insights > 0 || stats.insights_to_assets > 0 {
                                info!("🧠 Streaming auto-promotion: {} to Insights, {} to Assets", 
                                      stats.interact_to_insights, stats.insights_to_assets);
                            }
                        }
                        Err(e) => {
                            warn!("Failed streaming auto-promotion: {}", e);
                        }
                    }
                }
            });
        }

        Ok(())
    }
}

/// Глобальная статистика streaming API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalStreamingStats {
    pub active_sessions: usize,
    pub total_requests: u64,
    pub total_inserts: u64,
    pub total_searches: u64,
    pub total_batch_operations: u64,
    pub avg_processing_time_ms: f64,
    pub oldest_session_age_sec: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{default_config, MemoryService};

    #[tokio::test]
    async fn test_streaming_config() {
        let config = StreamingConfig::default();
        assert_eq!(config.max_concurrent_sessions, 100);
        assert_eq!(config.buffer_size, 50);
        assert_eq!(config.flush_timeout_ms, 1000);
    }

    #[tokio::test]
    async fn test_session_config() {
        let config = SessionConfig::default();
        assert_eq!(config.target_layer, Layer::Interact);
        assert!(config.enable_search);
        assert!(config.enable_ml_promotion);
    }

    #[test]
    fn test_streaming_priority() {
        assert_eq!(StreamingPriority::Normal, StreamingPriority::Normal);
        assert_ne!(StreamingPriority::High, StreamingPriority::Low);
    }
}