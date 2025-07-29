use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use reqwest::Client;
use tokio::io::AsyncWriteExt;
use tracing::{info, debug, warn};
use futures_util::StreamExt;

/// Конфигурация для загрузки моделей
#[derive(Debug, Clone)]
pub struct ModelDownloadConfig {
    pub model_name: String,
    pub repo_id: String,
    pub files: Vec<String>,
    pub target_dir: PathBuf,
}

impl ModelDownloadConfig {
    /// Создает конфигурацию для Qwen3 Embedding модели
    pub fn qwen3_embedding() -> Self {
        // Попробуем альтернативные источники моделей
        Self {
            model_name: "Qwen3-Embedding-0.6B-ONNX".to_string(),
            repo_id: "Qwen/Qwen2.5-0.5B".to_string(), // Временно используем другую модель
            files: vec![
                "model.safetensors".to_string(),
                "tokenizer.json".to_string(),
                "config.json".to_string(),
                "tokenizer_config.json".to_string(),
            ],
            target_dir: PathBuf::from("models/Qwen3-Embedding-0.6B-ONNX"),
        }
    }
    
    /// Создает конфигурацию для Qwen3 Reranker модели
    pub fn qwen3_reranker() -> Self {
        // Попробуем альтернативные источники моделей
        Self {
            model_name: "Qwen3-Reranker-0.6B-ONNX".to_string(),
            repo_id: "Qwen/Qwen2.5-0.5B".to_string(), // Временно используем другую модель
            files: vec![
                "model.safetensors".to_string(),
                "tokenizer.json".to_string(),
                "config.json".to_string(),
                "tokenizer_config.json".to_string(),
            ],
            target_dir: PathBuf::from("models/Qwen3-Reranker-0.6B-ONNX"),
        }
    }
}

/// Загрузчик моделей с Hugging Face
pub struct ModelDownloader {
    client: Client,
    base_url: String,
}

impl ModelDownloader {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .context("Failed to create HTTP client")?;
            
        Ok(Self {
            client,
            base_url: "https://huggingface.co".to_string(),
        })
    }
    
    /// Проверяет, нужно ли загружать модель
    pub async fn needs_download(&self, config: &ModelDownloadConfig) -> Result<bool> {
        // Проверяем существование директории
        if !config.target_dir.exists() {
            return Ok(true);
        }
        
        // Проверяем наличие всех файлов и их размеры
        for file_name in &config.files {
            let file_path = config.target_dir.join(file_name);
            if !file_path.exists() {
                info!("Missing file: {}", file_name);
                return Ok(true);
            }
            
            // Проверяем размер файла (особенно важно для больших файлов)
            if file_name.ends_with(".onnx_data") {
                let metadata = tokio::fs::metadata(&file_path).await?;
                let size_mb = metadata.len() / (1024 * 1024);
                
                // Если файл меньше 900 МБ, вероятно он скачан не полностью
                if size_mb < 900 {
                    warn!("File {} seems incomplete: {} MB", file_name, size_mb);
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
    
    /// Загружает модель с Hugging Face
    pub async fn download(&self, config: &ModelDownloadConfig) -> Result<()> {
        info!("Downloading model: {}", config.model_name);
        
        // Создаем директорию если нужно
        tokio::fs::create_dir_all(&config.target_dir).await
            .context("Failed to create model directory")?;
        
        // Загружаем каждый файл
        for file_name in &config.files {
            let file_url = format!(
                "{}/{}/resolve/main/{}",
                self.base_url, config.repo_id, file_name
            );
            
            let target_path = config.target_dir.join(file_name);
            
            // Проверяем, не скачан ли файл уже
            if target_path.exists() {
                let metadata = tokio::fs::metadata(&target_path).await?;
                let size_mb = metadata.len() / (1024 * 1024);
                
                // Для больших файлов проверяем размер
                if file_name.ends_with(".onnx_data") && size_mb < 900 {
                    info!("Re-downloading incomplete file: {} (current: {} MB)", file_name, size_mb);
                } else {
                    debug!("File already exists: {} ({} MB)", file_name, size_mb);
                    continue;
                }
            }
            
            info!("Downloading: {} ...", file_name);
            self.download_file(&file_url, &target_path).await
                .with_context(|| format!("Failed to download {}", file_name))?;
        }
        
        info!("Model {} downloaded successfully!", config.model_name);
        Ok(())
    }
    
    /// Загружает отдельный файл
    async fn download_file(&self, url: &str, target: &Path) -> Result<()> {
        // Создаем временный файл
        let temp_path = target.with_extension("tmp");
        
        // Получаем ответ
        let response = self.client
            .get(url)
            .send()
            .await
            .context("Failed to send request")?;
        
        // Проверяем статус
        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to download file: HTTP {} for {}",
                response.status(),
                url
            ));
        }
        
        // Получаем размер файла если доступен
        let total_size = response.content_length();
        if let Some(size) = total_size {
            info!("File size: {} MB", size / (1024 * 1024));
        }
        
        // Создаем файл для записи
        let mut file = tokio::fs::File::create(&temp_path).await
            .context("Failed to create temporary file")?;
        
        // Скачиваем по частям с помощью потока
        let mut stream = response.bytes_stream();
        let mut downloaded = 0u64;
        let mut last_progress = 0;
        
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.context("Failed to read chunk")?;
            file.write_all(&chunk).await
                .context("Failed to write chunk")?;
            
            downloaded += chunk.len() as u64;
            
            // Показываем прогресс каждые 10%
            if let Some(total) = total_size {
                let progress = (downloaded * 100) / total;
                if progress >= last_progress + 10 {
                    info!("Progress: {}%", progress);
                    last_progress = progress;
                }
            }
        }
        
        // Закрываем файл
        file.flush().await.context("Failed to flush file")?;
        drop(file);
        
        // Переименовываем временный файл в финальный
        tokio::fs::rename(&temp_path, target).await
            .context("Failed to rename temporary file")?;
        
        info!("Downloaded: {} ({} MB)", target.file_name().unwrap().to_string_lossy(), downloaded / (1024 * 1024));
        Ok(())
    }
}

/// Автоматически загружает модели если их нет
pub async fn ensure_models_downloaded() -> Result<()> {
    let downloader = ModelDownloader::new()?;
    
    // Проверяем и загружаем модель эмбеддингов
    let embedding_config = ModelDownloadConfig::qwen3_embedding();
    if downloader.needs_download(&embedding_config).await? {
        info!("Qwen3 Embedding model needs to be downloaded");
        downloader.download(&embedding_config).await?;
    } else {
        info!("Qwen3 Embedding model already downloaded");
    }
    
    // Проверяем и загружаем модель reranker
    let reranker_config = ModelDownloadConfig::qwen3_reranker();
    if downloader.needs_download(&reranker_config).await? {
        info!("Qwen3 Reranker model needs to be downloaded");
        downloader.download(&reranker_config).await?;
    } else {
        info!("Qwen3 Reranker model already downloaded");
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_model_config() {
        let config = ModelDownloadConfig::qwen3_embedding();
        assert_eq!(config.model_name, "Qwen3-Embedding-0.6B-ONNX");
        assert_eq!(config.files.len(), 8);
    }
}