use anyhow::Result;
use clap::{Args, Subcommand};
use ai::{MODEL_REGISTRY, ModelType};
use tracing::{info, warn, error};

#[derive(Debug, Args)]
pub struct ModelsCommand {
    #[command(subcommand)]
    command: ModelsSubcommand,
}

#[derive(Debug, Subcommand)]
enum ModelsSubcommand {
    /// Показать информацию о доступных моделях
    #[command(visible_alias = "ls")]
    List {
        /// Фильтр по типу модели
        #[arg(short, long)]
        model_type: Option<String>,
        
        /// Показать только доступные модели
        #[arg(short, long)]
        available_only: bool,
    },
    
    /// Диагностика моделей и конфигурации
    #[command(visible_alias = "diag")]
    Diagnose,
    
    /// Показать информацию о конкретной модели
    #[command(visible_alias = "info")]
    Show {
        /// Имя модели
        model_name: String,
    },
    
    /// Показать рекомендации по моделям
    #[command(visible_alias = "rec")]
    Recommendations,
    
    /// Проверить пути и конфигурацию моделей
    #[command(visible_alias = "check")]
    Check,
}

impl ModelsCommand {
    pub async fn execute(self) -> Result<()> {
        match self.command {
            ModelsSubcommand::List { model_type, available_only } => {
                Self::list_models(model_type, available_only)
            }
            ModelsSubcommand::Diagnose => Self::diagnose_models(),
            ModelsSubcommand::Show { model_name } => Self::show_model(&model_name),
            ModelsSubcommand::Recommendations => Self::show_recommendations(),
            ModelsSubcommand::Check => Self::check_models(),
        }
    }
    
    /// Показать список моделей
    fn list_models(model_type_filter: Option<String>, available_only: bool) -> Result<()> {
        info!("📋 Список зарегистрированных моделей:");
        
        // Парсим фильтр типа модели
        let filter_type = match model_type_filter.as_deref() {
            Some("embedding") | Some("emb") => Some(ModelType::Embedding),
            Some("reranker") | Some("rerank") => Some(ModelType::Reranker),
            Some(unknown) => {
                warn!("⚠️ Неизвестный тип модели: {}. Доступные: embedding, reranker", unknown);
                None
            }
            None => None,
        };
        
        let models = MODEL_REGISTRY.get_available_models(filter_type);
        
        if models.is_empty() {
            warn!("❌ Не найдено моделей по указанным критериям");
            return Ok(());
        }
        
        // Группируем по типам
        let mut embedding_models = Vec::new();
        let mut reranker_models = Vec::new();
        
        for model in models {
            if available_only && !MODEL_REGISTRY.is_model_available(&model.name) {
                continue;
            }
            
            match model.model_type {
                ModelType::Embedding => embedding_models.push(model),
                ModelType::Reranker => reranker_models.push(model),
            }
        }
        
        // Показываем embedding модели
        if !embedding_models.is_empty() {
            info!("\n🔤 Embedding модели:");
            for model in embedding_models {
                let status = if MODEL_REGISTRY.is_model_available(&model.name) {
                    "✅ Доступна"
                } else {
                    "❌ Недоступна"
                };
                let default_mark = if model.is_default { " [По умолчанию]" } else { "" };
                
                info!("  📦 {}{}", model.name, default_mark);
                info!("     Статус: {}", status);
                info!("     Размерность: {}", model.embedding_dim);
                info!("     Макс. длина: {}", model.max_length);
                info!("     Описание: {}", model.description);
                info!("");
            }
        }
        
        // Показываем reranker модели
        if !reranker_models.is_empty() {
            info!("🔄 Reranker модели:");
            for model in reranker_models {
                let status = if MODEL_REGISTRY.is_model_available(&model.name) {
                    "✅ Доступна"
                } else {
                    "❌ Недоступна"
                };
                let default_mark = if model.is_default { " [По умолчанию]" } else { "" };
                
                info!("  📦 {}{}", model.name, default_mark);
                info!("     Статус: {}", status);
                info!("     Макс. длина: {}", model.max_length);
                info!("     Описание: {}", model.description);
                info!("");
            }
        }
        
        Ok(())
    }
    
    /// Диагностика моделей
    fn diagnose_models() -> Result<()> {
        info!("🔍 Выполняем диагностику системы моделей...");
        
        MODEL_REGISTRY.diagnose_models()?;
        
        info!("\n📂 Пути к моделям:");
        info!("  - Основная директория: models/");
        info!("  - Переменная окружения: MAGRAY_MODELS_DIR");
        
        Ok(())
    }
    
    /// Показать информацию о конкретной модели
    fn show_model(model_name: &str) -> Result<()> {
        if let Some(model_info) = MODEL_REGISTRY.get_model_info(model_name) {
            info!("📦 Информация о модели: {}", model_name);
            info!("  🏷️ Тип: {:?}", model_info.model_type);
            info!("  📏 Размерность: {}", model_info.embedding_dim);
            info!("  📐 Макс. длина: {}", model_info.max_length);
            info!("  🎯 По умолчанию: {}", if model_info.is_default { "Да" } else { "Нет" });
            info!("  📝 Описание: {}", model_info.description);
            
            let model_path = MODEL_REGISTRY.get_model_path(model_name);
            info!("  📂 Путь: {}", model_path.display());
            
            let is_available = MODEL_REGISTRY.is_model_available(model_name);
            info!("  ✅ Доступна: {}", if is_available { "Да" } else { "Нет" });
            
            if is_available {
                let model_file = model_path.join("model.onnx");
                let tokenizer_file = model_path.join("tokenizer.json");
                
                if let Ok(model_metadata) = std::fs::metadata(&model_file) {
                    info!("  📊 Размер модели: {:.1} MB", 
                        model_metadata.len() as f64 / 1024.0 / 1024.0);
                }
                
                if tokenizer_file.exists() {
                    info!("  🔤 Токенизатор: Присутствует");
                }
            } else {
                error!("❌ Модель не найдена в указанном пути");
                info!("💡 Для загрузки модели используйте соответствующие скрипты загрузки");
            }
        } else {
            error!("❌ Модель '{}' не зарегистрирована", model_name);
            info!("💡 Используйте 'magray models list' для просмотра доступных моделей");
        }
        
        Ok(())
    }
    
    /// Показать рекомендации
    fn show_recommendations() -> Result<()> {
        info!("💡 Рекомендации по настройке моделей:");
        
        let recommendations = MODEL_REGISTRY.get_recommendations();
        
        if recommendations.is_empty() {
            info!("✅ Все модели настроены корректно!");
        } else {
            for (i, recommendation) in recommendations.iter().enumerate() {
                info!("  {}. {}", i + 1, recommendation);
            }
        }
        
        // Дополнительные рекомендации
        info!("\n🎯 Общие рекомендации:");
        info!("  • Используйте Qwen3 модели для лучшей поддержки русского языка");
        info!("  • BGE модели подходят для мультиязычных задач");
        info!("  • Убедитесь что модели находятся в папке models/ в корне проекта");
        info!("  • Для GPU ускорения используйте --features gpu при сборке");
        
        Ok(())
    }
    
    /// Проверить конфигурацию моделей
    fn check_models() -> Result<()> {
        info!("🔍 Проверка конфигурации моделей...");
        
        // Проверяем дефолтную конфигурацию
        let ai_config = ai::AiConfig::default();
        info!("📂 Директория моделей: {}", ai_config.models_dir.display());
        
        info!("🔤 Настройки embedding:");
        info!("  - Модель: {}", ai_config.embedding.model_name);
        info!("  - Batch size: {}", ai_config.embedding.batch_size);
        info!("  - Max length: {}", ai_config.embedding.max_length);
        info!("  - Use GPU: {}", ai_config.embedding.use_gpu);
        
        info!("🔄 Настройки reranking:");
        info!("  - Модель: {}", ai_config.reranking.model_name);
        info!("  - Batch size: {}", ai_config.reranking.batch_size);
        info!("  - Max length: {}", ai_config.reranking.max_length);
        info!("  - Use GPU: {}", ai_config.reranking.use_gpu);
        
        // Проверяем доступность моделей по умолчанию
        let embedding_available = MODEL_REGISTRY.is_model_available(&ai_config.embedding.model_name);
        let reranking_available = MODEL_REGISTRY.is_model_available(&ai_config.reranking.model_name);
        
        info!("\n📊 Статус моделей по умолчанию:");
        info!("  - Embedding ({}): {}", 
            ai_config.embedding.model_name,
            if embedding_available { "✅ Доступна" } else { "❌ Недоступна" }
        );
        info!("  - Reranking ({}): {}", 
            ai_config.reranking.model_name,
            if reranking_available { "✅ Доступна" } else { "❌ Недоступна" }
        );
        
        if !embedding_available || !reranking_available {
            warn!("\n⚠️ Некоторые модели по умолчанию недоступны");
            info!("💡 Используйте 'magray models recommendations' для получения инструкций");
        } else {
            info!("\n✅ Все модели по умолчанию доступны!");
        }
        
        Ok(())
    }
}