// @component: {"k":"C","id":"task_analyzer","t":"AI-powered task complexity and priority analyzer","m":{"cur":5,"tgt":90,"u":"%"},"f":["ai","analysis","nlp","complexity","priority"]}

use anyhow::Result;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, info};

use super::{ResourceRequirements, TaskComplexity, TaskPriority};

/// AI-powered task analyzer for determining complexity and resource requirements
pub struct TaskAnalyzer {
    // NLP models would go here in a full implementation
    complexity_keywords: HashMap<String, TaskComplexity>,
    priority_keywords: HashMap<String, TaskPriority>,
}

impl Default for TaskAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskAnalyzer {
    pub fn new() -> Self {
        let mut complexity_keywords = HashMap::new();
        let mut priority_keywords = HashMap::new();

        // Complexity analysis keywords
        complexity_keywords.insert("простой".to_string(), TaskComplexity::Simple);
        complexity_keywords.insert("simple".to_string(), TaskComplexity::Simple);
        complexity_keywords.insert("покажи".to_string(), TaskComplexity::Simple);
        complexity_keywords.insert("show".to_string(), TaskComplexity::Simple);
        complexity_keywords.insert("list".to_string(), TaskComplexity::Simple);
        complexity_keywords.insert("read".to_string(), TaskComplexity::Simple);

        complexity_keywords.insert("анализ".to_string(), TaskComplexity::Medium);
        complexity_keywords.insert("analyze".to_string(), TaskComplexity::Medium);
        complexity_keywords.insert("создай".to_string(), TaskComplexity::Medium);
        complexity_keywords.insert("create".to_string(), TaskComplexity::Medium);
        complexity_keywords.insert("write".to_string(), TaskComplexity::Medium);
        complexity_keywords.insert("find".to_string(), TaskComplexity::Medium);

        complexity_keywords.insert("архитектура".to_string(), TaskComplexity::Complex);
        complexity_keywords.insert("architecture".to_string(), TaskComplexity::Complex);
        complexity_keywords.insert("рефакторинг".to_string(), TaskComplexity::Complex);
        complexity_keywords.insert("refactor".to_string(), TaskComplexity::Complex);
        complexity_keywords.insert("optimize".to_string(), TaskComplexity::Complex);
        complexity_keywords.insert("performance".to_string(), TaskComplexity::Complex);

        complexity_keywords.insert("машинное обучение".to_string(), TaskComplexity::Expert);
        complexity_keywords.insert("machine learning".to_string(), TaskComplexity::Expert);
        complexity_keywords.insert("нейронные сети".to_string(), TaskComplexity::Expert);
        complexity_keywords.insert("neural network".to_string(), TaskComplexity::Expert);
        complexity_keywords.insert(
            "искусственный интеллект".to_string(),
            TaskComplexity::Expert,
        );
        complexity_keywords.insert(
            "artificial intelligence".to_string(),
            TaskComplexity::Expert,
        );

        // Priority analysis keywords
        priority_keywords.insert("срочно".to_string(), TaskPriority::Critical);
        priority_keywords.insert("urgent".to_string(), TaskPriority::Critical);
        priority_keywords.insert("критично".to_string(), TaskPriority::Critical);
        priority_keywords.insert("critical".to_string(), TaskPriority::Critical);
        priority_keywords.insert("emergency".to_string(), TaskPriority::Emergency);

        priority_keywords.insert("важно".to_string(), TaskPriority::High);
        priority_keywords.insert("important".to_string(), TaskPriority::High);
        priority_keywords.insert("приоритет".to_string(), TaskPriority::High);
        priority_keywords.insert("priority".to_string(), TaskPriority::High);

        priority_keywords.insert("обычно".to_string(), TaskPriority::Normal);
        priority_keywords.insert("normal".to_string(), TaskPriority::Normal);
        priority_keywords.insert("когда-нибудь".to_string(), TaskPriority::Low);
        priority_keywords.insert("later".to_string(), TaskPriority::Low);
        priority_keywords.insert("low".to_string(), TaskPriority::Low);

        Self {
            complexity_keywords,
            priority_keywords,
        }
    }

    /// Analyze task content to determine complexity and priority
    pub async fn analyze_task(
        &self,
        content: &str,
        context: &HashMap<String, String>,
    ) -> Result<(TaskPriority, TaskComplexity)> {
        debug!("🔍 Analyzing task: {}", content);

        let content_lower = content.to_lowercase();

        // Analyze complexity
        let complexity = self.analyze_complexity(&content_lower, context).await;

        // Analyze priority
        let priority = self.analyze_priority(&content_lower, context).await;

        info!(
            "📊 Task analysis complete: priority={:?}, complexity={:?}",
            priority, complexity
        );

        Ok((priority, complexity))
    }

    /// Analyze task complexity using keyword matching and heuristics
    async fn analyze_complexity(
        &self,
        content: &str,
        context: &HashMap<String, String>,
    ) -> TaskComplexity {
        let mut complexity_score = 0;
        let mut matched_complexity = TaskComplexity::Simple;

        for (keyword, complexity) in &self.complexity_keywords {
            if content.contains(keyword) {
                let keyword_score = match complexity {
                    TaskComplexity::Simple => 1,
                    TaskComplexity::Medium => 2,
                    TaskComplexity::Complex => 3,
                    TaskComplexity::Expert => 4,
                };

                if keyword_score > complexity_score {
                    complexity_score = keyword_score;
                    matched_complexity = complexity.clone();
                }
            }
        }

        // Heuristic analysis
        let word_count = content.split_whitespace().count();
        let has_technical_terms = content.contains("api")
            || content.contains("database")
            || content.contains("algorithm")
            || content.contains("optimization");
        let has_multiple_steps = content.contains(" и ")
            || content.contains(" then ")
            || content.contains("after")
            || content.contains("потом");

        // Apply heuristics
        let heuristic_complexity = if word_count > 50 || has_technical_terms || has_multiple_steps {
            match (has_technical_terms, has_multiple_steps, word_count > 100) {
                (true, true, true) => TaskComplexity::Expert,
                (true, true, _) => TaskComplexity::Complex,
                (true, _, _) | (_, true, _) => TaskComplexity::Medium,
                _ => TaskComplexity::Simple,
            }
        } else {
            TaskComplexity::Simple
        };

        // Consider context clues
        let context_complexity =
            if context.contains_key("urgent") || context.contains_key("critical") {
                TaskComplexity::Medium // Urgent tasks typically need more careful handling
            } else {
                TaskComplexity::Simple
            };

        // Return the highest complexity determined
        let complexities = [matched_complexity, heuristic_complexity, context_complexity];
        complexities
            .into_iter()
            .max_by_key(|c| match c {
                TaskComplexity::Simple => 1,
                TaskComplexity::Medium => 2,
                TaskComplexity::Complex => 3,
                TaskComplexity::Expert => 4,
            })
            .unwrap_or(TaskComplexity::Simple)
    }

    /// Analyze task priority using keyword matching and context
    async fn analyze_priority(
        &self,
        content: &str,
        context: &HashMap<String, String>,
    ) -> TaskPriority {
        let mut priority_score = 0;
        let mut matched_priority = TaskPriority::Normal;

        for (keyword, priority) in &self.priority_keywords {
            if content.contains(keyword) {
                let keyword_score = match priority {
                    TaskPriority::Low => 1,
                    TaskPriority::Normal => 2,
                    TaskPriority::High => 3,
                    TaskPriority::Critical => 4,
                    TaskPriority::Emergency => 5,
                };

                if keyword_score > priority_score {
                    priority_score = keyword_score;
                    matched_priority = priority.clone();
                }
            }
        }

        // Context-based priority adjustment
        let high_due_admin = context.get("user_role") == Some(&"admin".to_string());
        let high_due_deadline = context.contains_key("deadline");
        let high_due_retry = context
            .get("retry_count")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0)
            > 0;
        let context_priority = if high_due_admin || high_due_deadline || high_due_retry {
            TaskPriority::High
        } else {
            TaskPriority::Normal
        };

        // Return the higher priority
        if priority_score == 0 {
            context_priority
        } else {
            match (matched_priority, context_priority) {
                (TaskPriority::Emergency, _) | (_, TaskPriority::Emergency) => {
                    TaskPriority::Emergency
                }
                (TaskPriority::Critical, _) | (_, TaskPriority::Critical) => TaskPriority::Critical,
                (TaskPriority::High, _) | (_, TaskPriority::High) => TaskPriority::High,
                (TaskPriority::Normal, _) | (_, TaskPriority::Normal) => TaskPriority::Normal,
                _ => TaskPriority::Low,
            }
        }
    }

    /// Estimate resource requirements based on complexity and priority
    pub async fn estimate_resources(
        &self,
        complexity: &TaskComplexity,
        priority: &TaskPriority,
    ) -> ResourceRequirements {
        let (cpu_intensity, memory_usage, io_operations, network_usage, duration) = match complexity
        {
            TaskComplexity::Simple => (0.1f32, 0.1f32, 0.1f32, 0.1f32, Duration::from_millis(100)),
            TaskComplexity::Medium => (0.3f32, 0.2f32, 0.3f32, 0.2f32, Duration::from_millis(500)),
            TaskComplexity::Complex => (0.6f32, 0.5f32, 0.6f32, 0.4f32, Duration::from_secs(2)),
            TaskComplexity::Expert => (0.9f32, 0.8f32, 0.8f32, 0.6f32, Duration::from_secs(10)),
        };

        // Apply priority multiplier
        let priority_multiplier = match priority {
            TaskPriority::Low => 0.8f32,
            TaskPriority::Normal => 1.0f32,
            TaskPriority::High => 1.2f32,
            TaskPriority::Critical => 1.5f32,
            TaskPriority::Emergency => 2.0f32,
        };

        ResourceRequirements {
            cpu_intensity: (cpu_intensity * priority_multiplier).min(1.0f32),
            memory_usage: (memory_usage * priority_multiplier).min(1.0f32),
            io_operations: (io_operations * priority_multiplier).min(1.0f32),
            network_usage: (network_usage * priority_multiplier).min(1.0f32),
            estimated_duration: Duration::from_millis(
                (duration.as_millis() as f32 * priority_multiplier) as u64,
            ),
        }
    }

    /// Get task analysis statistics
    pub fn get_analysis_stats(&self) -> String {
        format!(
            "Task Analyzer Statistics:\n\
             • Complexity patterns: {} registered\n\
             • Priority patterns: {} registered\n\
             • Analysis method: Keyword matching + Heuristics\n\
             • Context awareness: Enabled",
            self.complexity_keywords.len(),
            self.priority_keywords.len()
        )
    }
}
