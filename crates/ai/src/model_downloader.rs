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

// Перенесено выше тестового модуля, чтобы избежать clippy::items_after_test_module
async fn verify_sha256(path: &Path, expected_hex: &str) -> Result<bool> {
    use sha2::{Digest, Sha256};
    use tokio::io::AsyncReadExt;
    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let result = hasher.finalize();
    let hex = format!("{result:x}");
    Ok(hex.eq_ignore_ascii_case(expected_hex))
}

impl ModelDownloader {
    /// Создать новый загрузчик моделей
    pub fn new(base_path: impl AsRef<Path>) -> Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        if let Ok(token) = std::env::var("HF_TOKEN") {
            if !token.is_empty() {
                if let Ok(value) =
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
                {
                    headers.insert(reqwest::header::AUTHORIZATION, value);
                }
            }
        }

        let client = reqwest::Client::builder()
            .user_agent("MAGRAY-CLI/1.0")
            .timeout(std::time::Duration::from_secs(300))
            .default_headers(headers)
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

        // Если есть известные контрольные суммы, проверим
        // (Сейчас нет реестра checksum'ов здесь; поле sha256 на каждом файле может быть задано из get_model_info)
        let info = self.get_model_info(
            model_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(""),
        );
        if let Ok(info) = info {
            for f in &info.files {
                if let Some(sum) = &f.sha256 {
                    let p = model_path.join(&f.filename);
                    if p.exists() {
                        if let Ok(valid) = verify_sha256(&p, sum).await {
                            if !valid {
                                warn!("Checksum mismatch for {}", p.display());
                                return Ok(false);
                            }
                        }
                    }
                }
            }
        }

        Ok(true)
    }

    /// Получить информацию о модели
    fn get_model_info(&self, model_name: &str) -> Result<ModelInfo> {
        match model_name {
            // Qwen3 Embedding 0.6B: будем пытаться вытянуть tokenizer/config с HF, а model.onnx — из нескольких кандидатных путей
            "qwen3emb" => Ok(ModelInfo {
                name: "qwen3emb".to_string(),
                files: vec![
                    ModelFile {
                        filename: "model.onnx".to_string(),
                        url: "AUTO".to_string(),
                        size: 0,
                        sha256: None,
                    },
                    ModelFile {
                        filename: "tokenizer.json".to_string(),
                        url: "AUTO".to_string(),
                        size: 0,
                        sha256: None,
                    },
                    ModelFile {
                        filename: "config.json".to_string(),
                        url: "AUTO".to_string(),
                        size: 0,
                        sha256: None,
                    },
                ],
                total_size: 0,
            }),

            // Qwen3 Reranker 0.6B
            "qwen3_reranker" => Ok(ModelInfo {
                name: "qwen3_reranker".to_string(),
                files: vec![
                    ModelFile {
                        filename: "model.onnx".to_string(),
                        url: "AUTO".to_string(),
                        size: 0,
                        sha256: None,
                    },
                    ModelFile {
                        filename: "tokenizer.json".to_string(),
                        url: "AUTO".to_string(),
                        size: 0,
                        sha256: None,
                    },
                    ModelFile {
                        filename: "config.json".to_string(),
                        url: "AUTO".to_string(),
                        size: 0,
                        sha256: None,
                    },
                ],
                total_size: 0,
            }),

            _ => Err(anyhow::anyhow!("Неизвестная модель: {}", model_name)),
        }
    }

    /// Вернуть список кандидатных URL для конкретного файла модели
    fn candidate_urls(&self, model_name: &str, filename: &str) -> Vec<String> {
        let mut out = Vec::new();
        match (model_name, filename) {
            ("qwen3emb", "tokenizer.json") => {
                out.push(
                    "https://huggingface.co/Qwen/Qwen3-Embedding-0.6B/resolve/main/tokenizer.json"
                        .to_string(),
                );
            }
            ("qwen3emb", "config.json") => {
                out.push(
                    "https://huggingface.co/Qwen/Qwen3-Embedding-0.6B/resolve/main/config.json"
                        .to_string(),
                );
            }
            ("qwen3emb", "model.onnx") => {
                out.push(
                    "https://huggingface.co/Qwen/Qwen3-Embedding-0.6B/resolve/main/model.onnx"
                        .to_string(),
                );
                out.push(
                    "https://huggingface.co/Qwen/Qwen3-Embedding-0.6B/resolve/main/onnx/model.onnx"
                        .to_string(),
                );
                out.push("https://huggingface.co/Qwen/Qwen3-Embedding-0.6B/resolve/main/onnx/encoder_model.onnx".to_string());
                out.push(
                    "https://huggingface.co/Qwen/Qwen3-Embedding-0.6B/resolve/main/model_fp16.onnx"
                        .to_string(),
                );
            }
            ("qwen3_reranker", "tokenizer.json") => {
                out.push(
                    "https://huggingface.co/Qwen/Qwen3-Reranker-0.6B/resolve/main/tokenizer.json"
                        .to_string(),
                );
            }
            ("qwen3_reranker", "config.json") => {
                out.push(
                    "https://huggingface.co/Qwen/Qwen3-Reranker-0.6B/resolve/main/config.json"
                        .to_string(),
                );
            }
            ("qwen3_reranker", "model.onnx") => {
                out.push(
                    "https://huggingface.co/Qwen/Qwen3-Reranker-0.6B/resolve/main/model.onnx"
                        .to_string(),
                );
                out.push(
                    "https://huggingface.co/Qwen/Qwen3-Reranker-0.6B/resolve/main/onnx/model.onnx"
                        .to_string(),
                );
                out.push("https://huggingface.co/Qwen/Qwen3-Reranker-0.6B/resolve/main/onnx/encoder_model.onnx".to_string());
                out.push(
                    "https://huggingface.co/Qwen/Qwen3-Reranker-0.6B/resolve/main/model_fp16.onnx"
                        .to_string(),
                );
            }
            _ => {}
        }
        out
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

        // Если AUTO — попробуем несколько кандидатных URL
        if file.url == "AUTO" {
            let model_name = dest_dir
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let candidates = self.candidate_urls(&model_name, &file.filename);
            if candidates.is_empty() {
                warn!("Нет кандидатных URL для {}/{}", model_name, file.filename);
                // Пытаемся продолжить дальше — файл может быть подготовлен скриптом отдельно
                return Ok(());
            }

            let mut last_err: Option<anyhow::Error> = None;
            for (idx, url) in candidates.iter().enumerate() {
                match self.try_download_once(url, file, &dest_path).await {
                    Ok(()) => {
                        info!("✅ {} загружен из {}", file.filename, url);
                        return Ok(());
                    }
                    Err(e) => {
                        warn!(
                            "Попытка {}/{}: не удалось скачать {} из {}: {}",
                            idx + 1,
                            candidates.len(),
                            file.filename,
                            url,
                            e
                        );
                        last_err = Some(e);
                        // небольшой backoff
                        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
                        continue;
                    }
                }
            }
            // Если ни один URL не сработал, возвращаем последнюю ошибку
            if let Some(e) = last_err {
                return Err(e);
            }
            return Err(anyhow::anyhow!("Не удалось скачать {}", file.filename));
        }

        // Проверяем существующий файл
        if dest_path.exists() {
            let metadata = fs::metadata(&dest_path).await?;
            if metadata.len() == file.size {
                // Если указан sha256 — провалидируем
                if let Some(sum) = &file.sha256 {
                    if verify_sha256(&dest_path, sum).await? {
                        info!("✅ Файл {} уже загружен (checksum ok)", file.filename);
                        return Ok(());
                    }
                    warn!(
                        "Checksum mismatch for existing {}, re-downloading",
                        file.filename
                    );
                } else {
                    info!("✅ Файл {} уже загружен", file.filename);
                    return Ok(());
                }
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

        // Попытка скачать напрямую из указанного URL
        self.try_download_once(&file.url, file, &dest_path).await
    }

    async fn try_download_once(&self, url: &str, file: &ModelFile, dest_path: &Path) -> Result<()> {
        // Создаём запрос
        let response = self
            .client
            .get(url)
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
                if total_size > 0 {
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
                }

                if total_size > 0 && bytes >= total_size {
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

        // Верификация checksum если задана
        if let Some(sum) = &file.sha256 {
            if !verify_sha256(&temp_path, sum).await? {
                return Err(anyhow::anyhow!(
                    "Checksum verification failed for {}",
                    file.filename
                ));
            }
        }

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
        let temp_dir = TempDir::new().expect("Operation should succeed");
        let downloader = ModelDownloader::new(temp_dir.path()).expect("Operation should succeed");

        let info = downloader
            .get_model_info("qwen3emb")
            .expect("Operation should succeed");
        assert_eq!(info.name, "qwen3emb");
        assert!(!info.files.is_empty());
        assert!(info.total_size == 0);
    }

    #[tokio::test]
    async fn test_model_detection() {
        let temp_dir = TempDir::new().expect("Operation should succeed");
        let downloader = ModelDownloader::new(temp_dir.path()).expect("Operation should succeed");

        let model_path = temp_dir.path().join("qwen3emb");
        let is_complete = downloader
            .is_model_complete(&model_path)
            .await
            .expect("Operation should succeed");
        assert!(!is_complete);

        // Создаём фейковые файлы
        fs::create_dir_all(&model_path)
            .await
            .expect("Operation should succeed");
        fs::write(model_path.join("model.onnx"), b"fake")
            .await
            .expect("Operation should succeed");
        fs::write(model_path.join("tokenizer.json"), b"fake")
            .await
            .expect("Operation should succeed");
        fs::write(model_path.join("config.json"), b"fake")
            .await
            .expect("Operation should succeed");

        let is_complete = downloader
            .is_model_complete(&model_path)
            .await
            .expect("Operation should succeed");
        assert!(is_complete);
    }
}
