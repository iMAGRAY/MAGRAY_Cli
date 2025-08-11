//! Task Domain Entity - Основная сущность задач в системе MAGRAY
//!
//! АРХИТЕКТУРНОЕ СООТВЕТСТВИЕ: Соответствует ARCHITECTURE_PLAN_ADVANCED.md
//! - Domain Layer: Task, Intent, Plan, ToolSpec
//! - Чистое ядро без зависимостей от инфраструктуры
//! - Применение DDD принципов

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use uuid::Uuid;

use super::intent::Intent;

/// Уникальный идентификатор задачи
pub type TaskId = Uuid;

/// Состояние выполнения задачи
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Задача создана, но еще не запланирована
    Created,
    /// Задача запланирована к выполнению
    Planned,
    /// Задача выполняется
    InProgress,
    /// Задача приостановлена
    Paused,
    /// Задача завершена успешно
    Completed,
    /// Задача завершена с ошибкой
    Failed { reason: String },
    /// Задача отменена
    Cancelled { reason: String },
}

/// Приоритет задачи
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Medium
    }
}

/// Тип задачи для категоризации
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskType {
    /// Разработка кода
    Development,
    /// Рефакторинг существующего кода
    Refactoring,
    /// Создание документации
    Documentation,
    /// Анализ и исследование
    Analysis,
    /// Тестирование
    Testing,
    /// Развертывание и CI/CD
    Deployment,
    /// Работа с файловой системой
    FileSystem,
    /// Сетевые операции
    Network,
    /// Пользовательский тип
    Custom(String),
}

/// Контекст выполнения задачи
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskContext {
    /// Рабочая директория
    pub working_directory: Option<String>,
    /// Переменные окружения
    pub environment: HashMap<String, String>,
    /// Проект к которому относится задача
    pub project_name: Option<String>,
    /// Языки программирования задействованные в задаче
    pub languages: Vec<String>,
    /// Связанные файлы
    pub related_files: Vec<String>,
    /// Дополнительные метаданные
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for TaskContext {
    fn default() -> Self {
        Self {
            working_directory: None,
            environment: HashMap::new(),
            project_name: None,
            languages: Vec::new(),
            related_files: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

/// Требование к выполнению задачи
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    /// Идентификатор требования
    pub id: String,
    /// Описание требования
    pub description: String,
    /// Является ли требование обязательным
    pub mandatory: bool,
    /// Критерии выполнения
    pub acceptance_criteria: Vec<String>,
}

/// Ограничение на выполнение задачи
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraint {
    /// Тип ограничения
    pub constraint_type: ConstraintType,
    /// Описание ограничения
    pub description: String,
    /// Значение ограничения
    pub value: serde_json::Value,
}

/// Типы ограничений
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintType {
    /// Временное ограничение
    TimeLimit,
    /// Ограничение по ресурсам
    ResourceLimit,
    /// Технологическое ограничение
    TechnologyConstraint,
    /// Политика безопасности
    SecurityPolicy,
    /// Пользовательское ограничение
    Custom(String),
}

/// Результат выполнения задачи
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Статус завершения
    pub status: TaskStatus,
    /// Сообщение о результате
    pub message: Option<String>,
    /// Созданные артефакты
    pub artifacts: Vec<String>,
    /// Метрики выполнения
    pub metrics: TaskMetrics,
    /// Время завершения
    pub completed_at: DateTime<Utc>,
    /// Дополнительные данные
    pub data: HashMap<String, serde_json::Value>,
}

/// Метрики выполнения задачи
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetrics {
    /// Время начала выполнения
    pub started_at: DateTime<Utc>,
    /// Время завершения
    pub completed_at: Option<DateTime<Utc>>,
    /// Фактическое время выполнения
    pub actual_duration: Option<Duration>,
    /// Количество использованных ресурсов
    pub resource_usage: HashMap<String, f64>,
    /// Количество выполненных операций
    pub operations_count: u64,
}

/// Основная сущность задачи
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Уникальный идентификатор
    pub id: TaskId,
    
    /// Намерение пользователя, породившее задачу
    pub intent: Intent,
    
    /// Текущий статус задачи
    pub status: TaskStatus,
    
    /// Приоритет выполнения
    pub priority: TaskPriority,
    
    /// Тип задачи
    pub task_type: TaskType,
    
    /// Зависимости от других задач
    pub dependencies: Vec<TaskId>,
    
    /// Задачи, которые зависят от этой
    pub dependents: Vec<TaskId>,
    
    /// Оценочное время выполнения
    pub estimated_effort: Option<Duration>,
    
    /// Контекст выполнения
    pub context: TaskContext,
    
    /// Требования к выполнению
    pub requirements: Vec<Requirement>,
    
    /// Ограничения
    pub constraints: Vec<Constraint>,
    
    /// Время создания
    pub created_at: DateTime<Utc>,
    
    /// Время последнего обновления
    pub updated_at: DateTime<Utc>,
    
    /// Результат выполнения (если завершена)
    pub result: Option<TaskResult>,
    
    /// Метки для категоризации
    pub tags: Vec<String>,
    
    /// Дополнительные метаданные
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Task {
    /// Создать новую задачу
    pub fn new(intent: Intent) -> Self {
        let now = Utc::now();
        
        Self {
            id: TaskId::new_v4(),
            intent,
            status: TaskStatus::Created,
            priority: TaskPriority::default(),
            task_type: TaskType::Development,
            dependencies: Vec::new(),
            dependents: Vec::new(),
            estimated_effort: None,
            context: TaskContext::default(),
            requirements: Vec::new(),
            constraints: Vec::new(),
            created_at: now,
            updated_at: now,
            result: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }
    
    /// Обновить статус задачи
    pub fn update_status(&mut self, new_status: TaskStatus) {
        self.status = new_status;
        self.updated_at = Utc::now();
    }
    
    /// Добавить зависимость
    pub fn add_dependency(&mut self, dependency_id: TaskId) {
        if !self.dependencies.contains(&dependency_id) {
            self.dependencies.push(dependency_id);
            self.updated_at = Utc::now();
        }
    }
    
    /// Удалить зависимость
    pub fn remove_dependency(&mut self, dependency_id: TaskId) {
        if let Some(pos) = self.dependencies.iter().position(|&id| id == dependency_id) {
            self.dependencies.remove(pos);
            self.updated_at = Utc::now();
        }
    }
    
    /// Проверить, готова ли задача к выполнению
    pub fn is_ready_for_execution(&self) -> bool {
        matches!(self.status, TaskStatus::Planned) && self.dependencies.is_empty()
    }
    
    /// Проверить, завершена ли задача
    pub fn is_completed(&self) -> bool {
        matches!(self.status, TaskStatus::Completed | TaskStatus::Failed { .. } | TaskStatus::Cancelled { .. })
    }
    
    /// Получить время выполнения
    pub fn get_actual_duration(&self) -> Option<Duration> {
        self.result.as_ref()?.metrics.actual_duration
    }
    
    /// Добавить требование
    pub fn add_requirement(&mut self, requirement: Requirement) {
        self.requirements.push(requirement);
        self.updated_at = Utc::now();
    }
    
    /// Добавить ограничение
    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
        self.updated_at = Utc::now();
    }
    
    /// Установить результат выполнения
    pub fn set_result(&mut self, result: TaskResult) {
        self.status = result.status.clone();
        self.result = Some(result);
        self.updated_at = Utc::now();
    }
    
    /// Получить описание задачи
    pub fn get_description(&self) -> &str {
        &self.intent.description
    }
    
    /// Получить краткое представление задачи
    pub fn get_summary(&self) -> String {
        format!(
            "[{}] {} ({})", 
            self.id.to_string().split('-').next().unwrap_or("unknown"),
            self.get_description(),
            match &self.status {
                TaskStatus::Created => "created",
                TaskStatus::Planned => "planned", 
                TaskStatus::InProgress => "in progress",
                TaskStatus::Paused => "paused",
                TaskStatus::Completed => "completed",
                TaskStatus::Failed { .. } => "failed",
                TaskStatus::Cancelled { .. } => "cancelled",
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::intent::{IntentCategory};
    
    #[test]
    fn test_task_creation() {
        let intent = Intent {
            category: IntentCategory::CodeGeneration,
            description: "Create a REST API".to_string(),
            requirements: Vec::new(),
            constraints: Vec::new(),
        };
        
        let task = Task::new(intent);
        
        assert_eq!(task.status, TaskStatus::Created);
        assert_eq!(task.priority, TaskPriority::Medium);
        assert!(task.dependencies.is_empty());
        assert!(!task.is_completed());
    }
    
    #[test] 
    fn test_task_status_update() {
        let intent = Intent {
            category: IntentCategory::CodeGeneration,
            description: "Create a REST API".to_string(),
            requirements: Vec::new(),
            constraints: Vec::new(),
        };
        
        let mut task = Task::new(intent);
        let initial_updated = task.updated_at;
        
        // Небольшая задержка чтобы время точно изменилось
        std::thread::sleep(std::time::Duration::from_millis(1));
        
        task.update_status(TaskStatus::InProgress);
        
        assert_eq!(task.status, TaskStatus::InProgress);
        assert!(task.updated_at > initial_updated);
    }
    
    #[test]
    fn test_task_dependencies() {
        let intent = Intent {
            category: IntentCategory::CodeGeneration,
            description: "Create a REST API".to_string(),
            requirements: Vec::new(),
            constraints: Vec::new(),
        };
        
        let mut task = Task::new(intent);
        let dep_id = TaskId::new_v4();
        
        task.add_dependency(dep_id);
        assert_eq!(task.dependencies.len(), 1);
        assert_eq!(task.dependencies[0], dep_id);
        
        // Повторное добавление не должно дублировать
        task.add_dependency(dep_id);
        assert_eq!(task.dependencies.len(), 1);
        
        task.remove_dependency(dep_id);
        assert!(task.dependencies.is_empty());
    }
    
    #[test]
    fn test_task_readiness() {
        let intent = Intent {
            category: IntentCategory::CodeGeneration,
            description: "Create a REST API".to_string(),
            requirements: Vec::new(),
            constraints: Vec::new(),
        };
        
        let mut task = Task::new(intent);
        
        // Созданная задача не готова к выполнению
        assert!(!task.is_ready_for_execution());
        
        // Запланированная задача без зависимостей готова
        task.update_status(TaskStatus::Planned);
        assert!(task.is_ready_for_execution());
        
        // Задача с зависимостями не готова
        task.add_dependency(TaskId::new_v4());
        assert!(!task.is_ready_for_execution());
    }
}