use crate::chunking::{UniversalChunker, ChunkingStrategy, ContentChunk};
use crate::{MemoryCoordinator, MemMeta, ExecutionContext};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tracing::{info, debug, warn};
use walkdir::WalkDir;

/// События процесса индексации
#[derive(Debug, Clone)]
pub enum IngestionEvent {
    Started { total_files: usize },
    FileProcessed { path: PathBuf, chunks: usize },
    FileSkipped { path: PathBuf, reason: String },
    ChunkIngested { chunk_id: String },
    Completed { total_chunks: usize, duration: std::time::Duration },
    Error { path: PathBuf, error: String },
}

/// Настройки для индексации
#[derive(Debug, Clone)]
pub struct IngestionConfig {
    /// Паттерны файлов для включения
    pub include_patterns: Vec<String>,
    /// Паттерны файлов для исключения
    pub exclude_patterns: Vec<String>,
    /// Максимальный размер файла для индексации (байты)
    pub max_file_size: usize,
    /// Игнорировать бинарные файлы
    pub skip_binary: bool,
    /// Следовать симлинкам
    pub follow_symlinks: bool,
    /// Стратегия чанкинга
    pub chunking_strategy: ChunkingStrategy,
    /// Батч размер для параллельной обработки
    pub batch_size: usize,
}

impl Default for IngestionConfig {
    fn default() -> Self {
        Self {
            include_patterns: vec![
                "**/*.rs".to_string(),
                "**/*.py".to_string(),
                "**/*.js".to_string(),
                "**/*.ts".to_string(),
                "**/*.jsx".to_string(),
                "**/*.tsx".to_string(),
                "**/*.go".to_string(),
                "**/*.java".to_string(),
                "**/*.cpp".to_string(),
                "**/*.c".to_string(),
                "**/*.h".to_string(),
                "**/*.hpp".to_string(),
                "**/*.md".to_string(),
                "**/*.txt".to_string(),
                "**/*.toml".to_string(),
                "**/*.yaml".to_string(),
                "**/*.yml".to_string(),
                "**/*.json".to_string(),
            ],
            exclude_patterns: vec![
                "**/target/**".to_string(),
                "**/node_modules/**".to_string(),
                "**/.git/**".to_string(),
                "**/dist/**".to_string(),
                "**/build/**".to_string(),
                "**/*.min.js".to_string(),
                "**/*.map".to_string(),
            ],
            max_file_size: 10 * 1024 * 1024, // 10MB
            skip_binary: true,
            follow_symlinks: false,
            chunking_strategy: ChunkingStrategy::default(),
            batch_size: 10,
        }
    }
}

/// Pipeline для индексации кода и документов
pub struct IngestionPipeline {
    memory: MemoryCoordinator,
    chunker: UniversalChunker,
    config: IngestionConfig,
}

impl IngestionPipeline {
    pub fn new(
        memory: MemoryCoordinator,
        config: IngestionConfig,
    ) -> Self {
        let chunker = UniversalChunker::new(config.chunking_strategy.clone());
        
        Self {
            memory,
            chunker,
            config,
        }
    }

    /// Индексировать директорию
    pub async fn ingest_directory(
        &self,
        dir_path: &Path,
        events_tx: mpsc::UnboundedSender<IngestionEvent>,
    ) -> Result<IngestionStats> {
        let start_time = std::time::Instant::now();
        let mut stats = IngestionStats::default();
        
        // Собираем файлы для обработки
        let files = self.collect_files(dir_path)?;
        let total_files = files.len();
        
        let _ = events_tx.send(IngestionEvent::Started { total_files });
        info!("Starting ingestion of {} files from {:?}", total_files, dir_path);
        
        // Обрабатываем файлы батчами
        for batch in files.chunks(self.config.batch_size) {
            let tasks: Vec<_> = batch
                .iter()
                .map(|file_path| {
                    let file_path = file_path.clone();
                    let events_tx = events_tx.clone();
                    self.process_file(file_path, events_tx)
                })
                .collect();
            
            // Ждем завершения батча
            let results = futures::future::join_all(tasks).await;
            
            for result in results {
                match result {
                    Ok(file_stats) => {
                        stats.files_processed += 1;
                        stats.chunks_created += file_stats.chunks_created;
                        stats.total_bytes += file_stats.bytes_processed;
                    }
                    Err(e) => {
                        stats.files_failed += 1;
                        warn!("Failed to process file: {}", e);
                    }
                }
            }
        }
        
        let duration = start_time.elapsed();
        stats.duration = duration;
        
        let _ = events_tx.send(IngestionEvent::Completed {
            total_chunks: stats.chunks_created,
            duration,
        });
        
        info!(
            "Ingestion completed: {} files, {} chunks in {:?}",
            stats.files_processed, stats.chunks_created, duration
        );
        
        Ok(stats)
    }

    /// Индексировать один файл
    pub async fn ingest_file(
        &self,
        file_path: &Path,
        events_tx: mpsc::UnboundedSender<IngestionEvent>,
    ) -> Result<FileIngestionStats> {
        self.process_file(file_path.to_path_buf(), events_tx).await
    }

    /// Обработать файл
    async fn process_file(
        &self,
        file_path: PathBuf,
        events_tx: mpsc::UnboundedSender<IngestionEvent>,
    ) -> Result<FileIngestionStats> {
        let mut stats = FileIngestionStats::default();
        
        // Проверяем размер файла
        let metadata = tokio::fs::metadata(&file_path).await?;
        if metadata.len() > self.config.max_file_size as u64 {
            let _ = events_tx.send(IngestionEvent::FileSkipped {
                path: file_path.clone(),
                reason: "File too large".to_string(),
            });
            return Ok(stats);
        }
        
        stats.bytes_processed = metadata.len() as usize;
        
        // Чанкуем файл
        let chunks = match self.chunker.chunk_file(&file_path).await {
            Ok(chunks) => chunks,
            Err(e) => {
                let _ = events_tx.send(IngestionEvent::Error {
                    path: file_path.clone(),
                    error: e.to_string(),
                });
                return Err(e);
            }
        };
        
        // Индексируем каждый чанк
        for chunk in chunks {
            if let Err(e) = self.ingest_chunk(&chunk).await {
                warn!("Failed to ingest chunk {}: {}", chunk.id, e);
                continue;
            }
            
            let _ = events_tx.send(IngestionEvent::ChunkIngested {
                chunk_id: chunk.id.clone(),
            });
            
            stats.chunks_created += 1;
        }
        
        let _ = events_tx.send(IngestionEvent::FileProcessed {
            path: file_path,
            chunks: stats.chunks_created,
        });
        
        Ok(stats)
    }

    /// Индексировать чанк
    async fn ingest_chunk(&self, chunk: &ContentChunk) -> Result<()> {
        let ctx = ExecutionContext::default();
        
        // Подготавливаем метаданные
        let mut meta = MemMeta::default();
        meta.content_type = "text/chunk".to_string();
        meta.size_bytes = chunk.content.len();
        meta.tags = chunk.tags.clone();
        
        // Добавляем дополнительные метаданные
        meta.extra.insert(
            "chunk_type".to_string(),
            serde_json::to_value(&chunk.chunk_type)?,
        );
        meta.extra.insert(
            "token_count".to_string(),
            serde_json::json!(chunk.token_count),
        );
        
        if let Some(ref parent_id) = chunk.parent_id {
            meta.extra.insert(
                "parent_id".to_string(),
                serde_json::json!(parent_id),
            );
        }
        
        if !chunk.related_ids.is_empty() {
            meta.extra.insert(
                "related_ids".to_string(),
                serde_json::to_value(&chunk.related_ids)?,
            );
        }
        
        // Сохраняем в memory систему
        // Используем smart_put для автоматического выбора слоя
        let result = self.memory
            .smart_put(&chunk.id, chunk.content.as_bytes(), meta.clone(), &ctx)
            .await?;
        
        // Добавляем в семантический индекс
        let text_for_indexing = if let Some(ref context) = chunk.context {
            format!("{}\n\n{}", context, chunk.content)
        } else {
            chunk.content.clone()
        };
        
        self.memory
            .semantic_index(&text_for_indexing, result.mem_ref.as_ref().unwrap(), &meta)
            .await?;
        
        debug!("Ingested chunk {} successfully", chunk.id);
        
        Ok(())
    }

    /// Собрать файлы для обработки
    fn collect_files(&self, dir_path: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let include_patterns: Vec<_> = self.config.include_patterns
            .iter()
            .map(|p| glob::Pattern::new(p))
            .collect::<Result<_, _>>()
            .context("Invalid include pattern")?;
        
        let exclude_patterns: Vec<_> = self.config.exclude_patterns
            .iter()
            .map(|p| glob::Pattern::new(p))
            .collect::<Result<_, _>>()
            .context("Invalid exclude pattern")?;
        
        for entry in WalkDir::new(dir_path)
            .follow_links(self.config.follow_symlinks)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            
            if !path.is_file() {
                continue;
            }
            
            let path_str = path.to_string_lossy();
            
            // Проверяем exclude паттерны
            let excluded = exclude_patterns.iter().any(|p| p.matches(&path_str));
            if excluded {
                continue;
            }
            
            // Проверяем include паттерны
            let included = include_patterns.iter().any(|p| p.matches(&path_str));
            if included {
                files.push(path.to_path_buf());
            }
        }
        
        Ok(files)
    }

    /// Обновить существующий индекс
    pub async fn update_index(&self, file_path: &Path) -> Result<()> {
        // TODO: Определить измененные чанки и обновить только их
        // Пока переиндексируем весь файл
        let (tx, _rx) = mpsc::unbounded_channel();
        self.ingest_file(file_path, tx).await?;
        Ok(())
    }

    /// Удалить файл из индекса
    pub async fn remove_from_index(&self, _file_path: &Path) -> Result<()> {
        // TODO: Удалить все чанки файла из памяти
        // Нужно хранить маппинг файл -> чанки
        Ok(())
    }
}

/// Статистика индексации
#[derive(Debug, Default)]
pub struct IngestionStats {
    pub files_processed: usize,
    pub files_failed: usize,
    pub chunks_created: usize,
    pub total_bytes: usize,
    pub duration: std::time::Duration,
}

#[derive(Debug, Default)]
struct FileIngestionStats {
    pub chunks_created: usize,
    pub bytes_processed: usize,
}

/// Файловый наблюдатель для автоматической переиндексации
pub struct FileWatcher {
    pipeline: IngestionPipeline,
    watch_paths: Vec<PathBuf>,
}

impl FileWatcher {
    pub fn new(pipeline: IngestionPipeline, watch_paths: Vec<PathBuf>) -> Self {
        Self {
            pipeline,
            watch_paths,
        }
    }

    /// Запустить наблюдение за изменениями
    pub async fn watch(&self) -> Result<()> {
        // TODO: Использовать notify для отслеживания изменений
        // и автоматической переиндексации
        Ok(())
    }
}