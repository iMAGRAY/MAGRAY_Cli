use anyhow::{Context, Result};
#[cfg(feature = "openai")]
use futures_util::StreamExt;
use reqwest;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};

pub struct ModelDownloader {
    base_path: PathBuf,
    client: reqwest::Client,
}

/// Информация о модели
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub files: Vec<ModelFile>,
    pub total_size: u64,
}

#[derive(Debug, Clone)]
pub struct ModelFile {
    pub filename: String,
    pub url: String,
    pub size: u64,
    pub sha256: Option<String>,
}

impl ModelDownloader {
    /// Создать новый загрузчик моделей
    pub fn new(base_path: impl AsRef<Path>) -> Result<Self> {
        let client = reqwest::Client::builder()
            .user_agent("MAGRAY-CLI/1.0")
            .timeout(std::time::Duration::from_secs(300))
            .build()?;

        Ok(Self {
            base_path: base_path.as_ref().to_path_buf(),
            client,
        })
    }

    /// Проверить и загрузить модель если необходимо
    pub async fn ensure_model(&self, model_name: &str) -> Result<PathBuf> {
        let model_path = self.base_path.join(model_name);

        // Проверяем существование модели
        if self.is_model_complete(&model_path).await? {
            info!("✅ Модель {} уже загружена", model_name);
            return Ok(model_path);
        }

        // Получаем информацию о модели
        let model_info = self.get_model_info(model_name)?;

        info!(
            "📥 Загрузка модели {} ({:.1} MB)",
            model_name,
            model_info.total_size as f64 / 1024.0 / 1024.0
        );

        // Создаём директорию
        fs::create_dir_all(&model_path).await?;

        // Загружаем файлы
        for file in &model_info.files {
            self.download_file(file, &model_path).await?;
        }

        info!("✅ Модель {} успешно загружена", model_name);
        Ok(model_path)
    }

    /// Проверить что модель полностью загружена
    async fn is_model_complete(&self, model_path: &Path) -> Result<bool> {
        if !model_path.exists() {
            return Ok(false);
        }

        // Проверяем наличие модели
        let model_exists = model_path.join("model.onnx").exists();

        if !model_exists {
            return Ok(false);
        }

        // Проверяем наличие остальных файлов
        let required_files = vec!["tokenizer.json", "config.json"];

        for file in required_files {
            let file_path = model_path.join(file);
            if !file_path.exists() {
                return Ok(false);
            }

            // Проверяем что файл не пустой
            let metadata = fs::metadata(&file_path).await?;
            if metadata.len() == 0 {
                warn!("⚠️ Файл {} пустой", file);
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Получить информацию о модели
    fn get_model_info(&self, model_name: &str) -> Result<ModelInfo> {
        match model_name {
            "qwen3emb" => Ok(ModelInfo {
                name: "qwen3emb".to_string(),
                files: vec![
                    ModelFile {
                        filename: "model.onnx".to_string(),
                        url: "LOCAL_FILE".to_string(),
                        size: 0,
                        sha256: None,
                    },
                    ModelFile {
                        filename: "tokenizer.json".to_string(),
                        url: "LOCAL_FILE".to_string(),
                        size: 0,
                        sha256: None,
                    },
                    ModelFile {
                        filename: "config.json".to_string(),
                        url: "LOCAL_FILE".to_string(),
                        size: 0,
                        sha256: None,
                    },
                ],
                total_size: 0,
            }),

            "qwen3_reranker" => Ok(ModelInfo {
                name: "qwen3_reranker".to_string(),
                files: vec![
                    ModelFile {
                        filename: "model.onnx".to_string(),
                        url: "LOCAL_FILE".to_string(),
                        size: 0,
                        sha256: None,
                    },
                    ModelFile {
                        filename: "tokenizer.json".to_string(),
                        url: "LOCAL_FILE".to_string(),
                        size: 0,
                        sha256: None,
                    },
                    ModelFile {
                        filename: "config.json".to_string(),
                        url: "LOCAL_FILE".to_string(),
                        size: 0,
                        sha256: None,
                    },
                ],
                total_size: 0,
            }),

            _ => Err(anyhow::anyhow!("Неизвестная модель: {}", model_name)),
        }
    }

    /// Загрузить файл с прогрессом
    async fn download_file(&self, file: &ModelFile, dest_dir: &Path) -> Result<()> {
        let dest_path = dest_dir.join(&file.filename);

        // Если это локальный файл, просто проверяем его наличие
        if file.url == "LOCAL_FILE" {
            if dest_path.exists() {
                info!("✅ Локальный файл {} найден", file.filename);
                return Ok(());
            } else {
                return Err(anyhow::anyhow!(
                    "Локальный файл {} не найден",
                    file.filename
                ));
            }
        }

        // Проверяем существующий файл
        if dest_path.exists() {
            let metadata = fs::metadata(&dest_path).await?;
            if metadata.len() == file.size {
                info!("✅ Файл {} уже загружен", file.filename);
                return Ok(());
            } else {
                warn!(
                    "⚠️ Размер файла {} не совпадает, перезагружаем",
                    file.filename
                );
            }
        }

        info!(
            "📥 Загрузка {} ({:.1} MB)...",
            file.filename,
            file.size as f64 / 1024.0 / 1024.0
        );

        // Создаём запрос
        let response = self
            .client
            .get(&file.url)
            .send()
            .await
            .context("Ошибка при запросе файла")?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Ошибка загрузки {}: HTTP {}",
                file.filename,
                response.status()
            ));
        }

        // Получаем размер
        let total_size = response.content_length().unwrap_or(file.size);

        // Создаём временный файл
        let temp_path = dest_path.with_extension("tmp");
        let mut temp_file = tokio::fs::File::create(&temp_path).await?;

        // Загружаем с прогрессом
        let downloaded = Arc::new(AtomicU64::new(0));
        let downloaded_clone = downloaded.clone();

        // Spawn задачу для отображения прогресса
        let progress_task = tokio::spawn(async move {
            let mut last_report = std::time::Instant::now();
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

                let bytes = downloaded_clone.load(Ordering::Relaxed);
                let progress = (bytes as f64 / total_size as f64) * 100.0;

                if last_report.elapsed().as_secs() >= 2 {
                    info!(
                        "   📊 Прогресс: {:.1}% ({:.1} MB / {:.1} MB)",
                        progress,
                        bytes as f64 / 1024.0 / 1024.0,
                        total_size as f64 / 1024.0 / 1024.0
                    );
                    last_report = std::time::Instant::now();
                }

                if bytes >= total_size {
                    break;
                }
            }
        });

        // Загружаем данные
        #[cfg(feature = "openai")]
        {
            let mut stream = response.bytes_stream();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk?;
                temp_file.write_all(&chunk).await?;
                downloaded.fetch_add(chunk.len() as u64, Ordering::Relaxed);
            }
        }
        #[cfg(not(feature = "openai"))]
        {
            let bytes = response.bytes().await?;
            temp_file.write_all(&bytes).await?;
            downloaded.fetch_add(bytes.len() as u64, Ordering::Relaxed);
        }

        temp_file.flush().await?;
        drop(temp_file);

        // Ждём завершения прогресса
        progress_task.abort();

        // Переименовываем временный файл
        fs::rename(&temp_path, &dest_path).await?;

        info!("✅ {} загружен успешно", file.filename);
        Ok(())
    }

    /// Очистить кэш моделей
    pub async fn clear_cache(&self) -> Result<()> {
        if self.base_path.exists() {
            fs::remove_dir_all(&self.base_path).await?;
            info!("🧹 Кэш моделей очищен");
        }
        Ok(())
    }

    /// Получить размер кэша
    pub async fn get_cache_size(&self) -> Result<u64> {
        if !self.base_path.exists() {
            return Ok(0);
        }

        let mut total_size = 0u64;
        let mut entries = fs::read_dir(&self.base_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            if let Ok(metadata) = entry.metadata().await {
                if metadata.is_file() {
                    total_size += metadata.len();
                }
            }
        }

        Ok(total_size)
    }
}

lazy_static::lazy_static! {
    /// Глобальный загрузчик моделей
    pub static ref MODEL_DOWNLOADER: ModelDownloader = {
        let models_dir = PathBuf::from("models");
        ModelDownloader::new(models_dir)
            .expect("Не удалось создать загрузчик моделей")
    };
}

/// Убедиться что модель загружена
pub async fn ensure_model(model_name: &str) -> Result<PathBuf> {
    MODEL_DOWNLOADER.ensure_model(model_name).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_model_info() {
        let temp_dir = TempDir::new().unwrap();
        let downloader = ModelDownloader::new(temp_dir.path()).unwrap();

        let info = downloader.get_model_info("qwen3emb").unwrap();
        assert_eq!(info.name, "qwen3emb");
        assert!(!info.files.is_empty());
        assert!(info.total_size == 0);
    }

    #[tokio::test]
    async fn test_model_detection() {
        let temp_dir = TempDir::new().unwrap();
        let downloader = ModelDownloader::new(temp_dir.path()).unwrap();

        let model_path = temp_dir.path().join("qwen3emb");
        let is_complete = downloader.is_model_complete(&model_path).await.unwrap();
        assert!(!is_complete);

        // Создаём фейковые файлы
        fs::create_dir_all(&model_path).await.unwrap();
        fs::write(model_path.join("model.onnx"), b"fake")
            .await
            .unwrap();
        fs::write(model_path.join("tokenizer.json"), b"fake")
            .await
            .unwrap();
        fs::write(model_path.join("config.json"), b"fake")
            .await
            .unwrap();

        let is_complete = downloader.is_model_complete(&model_path).await.unwrap();
        assert!(is_complete);
    }
}
