use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{info, warn};

pub struct ModelRegistry {
    models_dir: PathBuf,
    available_models: HashMap<String, ModelInfo>,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub model_type: ModelType,
    pub embedding_dim: usize,
    pub max_length: usize,
    pub description: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModelType {
    Embedding,
    Reranker,
}

impl ModelRegistry {
    /// Создать новый реестр моделей
    pub fn new(models_dir: PathBuf) -> Self {
        let mut registry = Self {
            models_dir,
            available_models: HashMap::new(),
        };

        // Регистрируем доступные модели
        registry.register_default_models();
        registry
    }

    /// Регистрировать стандартные модели
    fn register_default_models(&mut self) {
        // Qwen3 модели (рекомендуемые)
        self.register_model(ModelInfo {
            name: "qwen3emb".to_string(),
            model_type: ModelType::Embedding,
            embedding_dim: 1024,
            max_length: 512,
            description: "Qwen3 embedding model - оптимизированный для русского языка".to_string(),
            is_default: true,
        });

        self.register_model(ModelInfo {
            name: "qwen3_reranker".to_string(),
            model_type: ModelType::Reranker,
            embedding_dim: 0, // Reranker не имеет embedding dim
            max_length: 512,
            description: "Qwen3 reranker - семантическое переранжирование".to_string(),
            is_default: true,
        });
    }

    /// Зарегистрировать модель
    pub fn register_model(&mut self, model: ModelInfo) {
        info!(
            "📝 Регистрируем модель: {} ({:?})",
            model.name, model.model_type
        );
        self.available_models.insert(model.name.clone(), model);
    }

    /// Получить информацию о модели
    pub fn get_model_info(&self, name: &str) -> Option<&ModelInfo> {
        self.available_models.get(name)
    }

    /// Получить модель по умолчанию для типа
    pub fn get_default_model(&self, model_type: ModelType) -> Option<&ModelInfo> {
        self.available_models
            .values()
            .find(|model| model.model_type == model_type && model.is_default)
    }

    /// Получить путь к модели
    pub fn get_model_path(&self, name: &str) -> PathBuf {
        self.models_dir.join(name)
    }

    /// Проверить доступность модели
    pub fn is_model_available(&self, name: &str) -> bool {
        let model_path = self.get_model_path(name);
        let model_file = model_path.join("model.onnx");
        let tokenizer_file = model_path.join("tokenizer.json");

        model_file.exists() && tokenizer_file.exists()
    }

    /// Получить список доступных моделей по типу
    pub fn get_available_models(&self, model_type: Option<ModelType>) -> Vec<&ModelInfo> {
        self.available_models
            .values()
            .filter(|model| {
                if let Some(ref desired_type) = model_type {
                    &model.model_type == desired_type
                } else {
                    true
                }
            })
            .filter(|model| self.is_model_available(&model.name))
            .collect()
    }

    /// Выполнить автодиагностику моделей
    pub fn diagnose_models(&self) -> Result<()> {
        info!("🔍 Диагностика доступных моделей...");

        let embedding_models = self.get_available_models(Some(ModelType::Embedding));
        let reranker_models = self.get_available_models(Some(ModelType::Reranker));

        info!("📊 Доступные embedding модели: {}", embedding_models.len());
        for model in &embedding_models {
            info!("  ✅ {} - {}", model.name, model.description);
        }

        info!("📊 Доступные reranker модели: {}", reranker_models.len());
        for model in &reranker_models {
            info!("  ✅ {} - {}", model.name, model.description);
        }

        // Проверяем наличие моделей по умолчанию
        if let Some(default_embedding) = self.get_default_model(ModelType::Embedding) {
            if self.is_model_available(&default_embedding.name) {
                info!(
                    "✅ Embedding модель по умолчанию доступна: {}",
                    default_embedding.name
                );
            } else {
                warn!(
                    "⚠️ Embedding модель по умолчанию недоступна: {}",
                    default_embedding.name
                );
            }
        }

        if let Some(default_reranker) = self.get_default_model(ModelType::Reranker) {
            if self.is_model_available(&default_reranker.name) {
                info!(
                    "✅ Reranker модель по умолчанию доступна: {}",
                    default_reranker.name
                );
            } else {
                warn!(
                    "⚠️ Reranker модель по умолчанию недоступна: {}",
                    default_reranker.name
                );
            }
        }

        // Проверяем проблемные папки
        let old_models_dir = PathBuf::from("crates/memory/models");
        if old_models_dir.exists() {
            warn!("⚠️ Найдена старая папка моделей: {:?}", old_models_dir);
            warn!(
                "   Рекомендуется переместить модели в централизованную папку: {:?}",
                self.models_dir
            );
        }

        Ok(())
    }

    /// Получить рекомендации по моделям
    pub fn get_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Проверяем доступность моделей по умолчанию
        if let Some(default_embedding) = self.get_default_model(ModelType::Embedding) {
            if !self.is_model_available(&default_embedding.name) {
                recommendations.push(format!(
                    "Загрузите embedding модель по умолчанию: {} в папку {}",
                    default_embedding.name,
                    self.get_model_path(&default_embedding.name).display()
                ));
            }
        }

        if let Some(default_reranker) = self.get_default_model(ModelType::Reranker) {
            if !self.is_model_available(&default_reranker.name) {
                recommendations.push(format!(
                    "Загрузите reranker модель по умолчанию: {} в папку {}",
                    default_reranker.name,
                    self.get_model_path(&default_reranker.name).display()
                ));
            }
        }

        // Проверяем дублирование
        let old_models_dir = PathBuf::from("crates/memory/models");
        if old_models_dir.exists() {
            recommendations.push(format!(
                "Удалите дублированную папку моделей: {:?} и используйте централизованную: {:?}",
                old_models_dir, self.models_dir
            ));
        }

        recommendations
    }
}

lazy_static::lazy_static! {
    /// Глобальный реестр моделей
    pub static ref MODEL_REGISTRY: ModelRegistry = {
        let models_dir = std::env::var("MAGRAY_MODELS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("models"));

        ModelRegistry::new(models_dir)
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_model_registry() {
        let temp_dir = TempDir::new().unwrap();
        let registry = ModelRegistry::new(temp_dir.path().to_path_buf());

        // Проверяем модели по умолчанию
        let default_embedding = registry.get_default_model(ModelType::Embedding);
        assert!(default_embedding.is_some());
        assert_eq!(default_embedding.unwrap().name, "qwen3emb");

        let default_reranker = registry.get_default_model(ModelType::Reranker);
        assert!(default_reranker.is_some());
        assert_eq!(default_reranker.unwrap().name, "qwen3_reranker");

        // Проверяем получение информации о модели
        let qwen3_info = registry.get_model_info("qwen3emb");
        assert!(qwen3_info.is_some());
        assert_eq!(qwen3_info.unwrap().embedding_dim, 1024);
    }

    #[test]
    fn test_model_availability() {
        let temp_dir = TempDir::new().unwrap();
        let registry = ModelRegistry::new(temp_dir.path().to_path_buf());

        // Модель не должна быть доступна без файлов
        assert!(!registry.is_model_available("qwen3emb"));

        // Создаём файлы модели
        let model_dir = temp_dir.path().join("qwen3emb");
        std::fs::create_dir_all(&model_dir).unwrap();
        std::fs::write(model_dir.join("model.onnx"), b"dummy").unwrap();
        std::fs::write(model_dir.join("tokenizer.json"), b"{}").unwrap();

        // Теперь модель должна быть доступна
        assert!(registry.is_model_available("qwen3emb"));
    }
}
