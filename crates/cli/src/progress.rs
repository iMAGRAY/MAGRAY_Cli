use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use colored::Colorize;

/// Типы операций для адаптивных прогресс-баров
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum ProgressType {
    /// Быстрая операция (100-500ms)
    Fast,
    /// Средняя операция (0.5-5s) 
    Medium,
    /// Медленная операция (5s+)
    Slow,
    /// Backup/restore операции
    Backup,
    /// Поиск и индексация
    Search,
    /// Система памяти
    Memory,
}

/// Стили прогресс-баров для разных операций
#[derive(Debug, Clone)]
pub struct ProgressConfig {
    pub spinner_chars: &'static str,
    pub tick_interval: Duration,
    pub color: &'static str,
    pub success_message: Option<String>,
}

impl ProgressType {
    /// Получить конфигурацию для типа операции
    pub fn config(self) -> ProgressConfig {
        match self {
            ProgressType::Fast => ProgressConfig {
                spinner_chars: "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏",
                tick_interval: Duration::from_millis(80),
                color: "cyan",
                success_message: None,
            },
            ProgressType::Medium => ProgressConfig {
                spinner_chars: "⠋⠙⠚⠞⠖⠦⠴⠲⠳⠓",
                tick_interval: Duration::from_millis(120),
                color: "blue",
                success_message: None,
            },
            ProgressType::Slow => ProgressConfig {
                spinner_chars: "⠋⠙⠚⠒⠂⠂⠒⠲⠴⠦⠖⠒⠐⠐⠒⠓⠋",
                tick_interval: Duration::from_millis(150),
                color: "yellow",
                success_message: None,
            },
            ProgressType::Backup => ProgressConfig {
                spinner_chars: "📁📂📁📂",
                tick_interval: Duration::from_millis(200),
                color: "green",
                success_message: Some("✓ Operation completed!".to_string()),
            },
            ProgressType::Search => ProgressConfig {
                spinner_chars: "🔍🔎🔍🔎",
                tick_interval: Duration::from_millis(300),
                color: "magenta",
                success_message: Some("✓ Search completed!".to_string()),
            },
            ProgressType::Memory => ProgressConfig {
                spinner_chars: "🧠💭🧠💭",
                tick_interval: Duration::from_millis(250),
                color: "purple",
                success_message: Some("✓ Memory operation completed!".to_string()),
            },
        }
    }

    /// Создать оптимизированный прогресс-бар
    pub fn create_spinner(self, message: &str) -> AdaptiveSpinner {
        let config = self.config();
        let spinner = indicatif::ProgressBar::new_spinner();
        
        let template = match self {
            ProgressType::Backup | ProgressType::Search | ProgressType::Memory => {
                "{spinner} {msg}".to_string()
            },
            _ => {
                format!("{{spinner:.{}}} {{msg}}", config.color)
            }
        };
        
        spinner.set_style(
            indicatif::ProgressStyle::default_spinner()
                .tick_chars(config.spinner_chars)
                .template(&template)
                .unwrap()
        );
        
        spinner.set_message(message.to_string());
        spinner.enable_steady_tick(config.tick_interval);
        
        AdaptiveSpinner {
            spinner,
            progress_type: self,
            config,
        }
    }
}

/// Умный спиннер с адаптивным поведением
pub struct AdaptiveSpinner {
    spinner: indicatif::ProgressBar,
    #[allow(dead_code)]
    progress_type: ProgressType,
    #[allow(dead_code)]
    config: ProgressConfig,
}

impl AdaptiveSpinner {
    /// Обновить сообщение спиннера
    #[allow(dead_code)]
    pub fn set_message(&self, message: &str) {
        self.spinner.set_message(message.to_string());
    }
    
    /// Завершить с успехом
    pub fn finish_success(&self, message: Option<&str>) {
        let msg = message
            .or(self.config.success_message.as_deref())
            .unwrap_or("✓ Completed!");
            
        let colored_msg = match self.config.color {
            "green" => msg.green().to_string(),
            "blue" => msg.blue().to_string(),
            "cyan" => msg.cyan().to_string(),
            "yellow" => msg.yellow().to_string(),
            "magenta" => msg.magenta().to_string(),
            "purple" => msg.purple().to_string(),
            _ => msg.green().to_string(),
        };
        
        self.spinner.finish_with_message(colored_msg);
    }
    
    /// Завершить с ошибкой
    #[allow(dead_code)]
    pub fn finish_error(&self, message: &str) {
        let error_msg = format!("✗ {message}");
        let error_msg_str = error_msg.red().to_string();
        self.spinner.finish_with_message(error_msg_str);
    }
    
    /// Завершить и очистить
    #[allow(dead_code)]
    pub fn finish_and_clear(&self) {
        self.spinner.finish_and_clear();
    }
    
    /// Установить прогресс для операций с известной длительностью
    #[allow(dead_code)]
    pub fn set_progress(&self, current: u64, total: u64) {
        if total > 0 {
            let percentage = (current * 100) / total;
            let progress_msg = format!("{percentage}% ({current}/{total})");
            self.set_message(&progress_msg);
        }
    }
    
    /// Адаптивная задержка между операциями
    #[allow(dead_code)]
    pub fn adaptive_delay(&self) -> Duration {
        match self.progress_type {
            ProgressType::Fast => Duration::from_millis(50),
            ProgressType::Medium => Duration::from_millis(100),
            ProgressType::Slow => Duration::from_millis(200),
            ProgressType::Backup => Duration::from_millis(300),
            ProgressType::Search => Duration::from_millis(150),
            ProgressType::Memory => Duration::from_millis(250),
        }
    }
}

/// Мульти-этапный прогресс для сложных операций
#[allow(dead_code)]
pub struct MultiStageProgress {
    stages: Vec<(&'static str, ProgressType)>,
    current_stage: usize,
    current_spinner: Option<AdaptiveSpinner>,
}

#[allow(dead_code)]
impl MultiStageProgress {
    /// Создать мульти-этапный прогресс
    pub fn new(stages: Vec<(&'static str, ProgressType)>) -> Self {
        Self {
            stages,
            current_stage: 0,
            current_spinner: None,
        }
    }
    
    /// Перейти к следующему этапу
    pub fn next_stage(&mut self) -> bool {
        if let Some(spinner) = &self.current_spinner {
            spinner.finish_success(None);
        }
        
        if self.current_stage < self.stages.len() {
            let (message, progress_type) = &self.stages[self.current_stage];
            let stage_msg = format!("[{}/{}] {}", 
                                   self.current_stage + 1, 
                                   self.stages.len(), 
                                   message);
            
            self.current_spinner = Some(progress_type.create_spinner(&stage_msg));
            self.current_stage += 1;
            true
        } else {
            false
        }
    }
    
    /// Завершить все этапы
    pub fn finish(&mut self) {
        if let Some(spinner) = &self.current_spinner {
            spinner.finish_success(Some("✓ All stages completed!"));
        }
    }
    
    /// Получить текущий спиннер
    pub fn current_spinner(&self) -> Option<&AdaptiveSpinner> {
        self.current_spinner.as_ref()
    }
}

/// Конструктор для быстрого создания прогресс-баров
pub struct ProgressBuilder;

impl ProgressBuilder {
    /// Создать быстрый спиннер
    pub fn fast(message: &str) -> AdaptiveSpinner {
        ProgressType::Fast.create_spinner(message)
    }
    
    /// Создать медленный спиннер
    #[allow(dead_code)]
    pub fn slow(message: &str) -> AdaptiveSpinner {
        ProgressType::Slow.create_spinner(message)
    }
    
    /// Создать спиннер для backup операций
    pub fn backup(message: &str) -> AdaptiveSpinner {
        ProgressType::Backup.create_spinner(message)
    }
    
    /// Создать спиннер для поиска
    #[allow(dead_code)]
    pub fn search(message: &str) -> AdaptiveSpinner {
        ProgressType::Search.create_spinner(message)
    }
    
    /// Создать спиннер для операций с памятью
    pub fn memory(message: &str) -> AdaptiveSpinner {
        ProgressType::Memory.create_spinner(message)
    }
}

// @component: {"k":"C","id":"adaptive_progress","t":"Adaptive progress indicators","m":{"cur":95,"tgt":100,"u":"%"},"f":["ui","progress","adaptive"]}

