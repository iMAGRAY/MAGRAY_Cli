//! Error handling utilities - устранение .unwrap() и .expect() patterns
//!
//! РЕШАЕТ ПРОБЛЕМЫ:
//! - Избыточные .unwrap() calls в тестах и production коде
//! - Consistent error handling patterns
//! - Reduced boilerplate для error conversion

use anyhow::{anyhow, Result};

/// Utility trait для улучшенного error handling
pub trait ErrorUtils<T> {
    /// Convert result с контекстным сообщением об ошибке
    fn with_context_msg(self, msg: &str) -> Result<T>;

    /// Convert result с formatted context message
    fn with_context_fmt(self, msg: &str, args: &[&dyn std::fmt::Display]) -> Result<T>;

    /// Convert с generic fallback error message
    fn or_error(self, error_msg: &str) -> Result<T>;
}

impl<T, E: std::fmt::Display> ErrorUtils<T> for Result<T, E> {
    fn with_context_msg(self, msg: &str) -> Result<T> {
        self.map_err(|e| anyhow!("{}: {}", msg, e))
    }

    fn with_context_fmt(self, msg: &str, args: &[&dyn std::fmt::Display]) -> Result<T> {
        self.map_err(|e| {
            let formatted_args: Vec<String> = args.iter().map(|a| format!("{}", a)).collect();
            anyhow!("{} [{}]: {}", msg, formatted_args.join(", "), e)
        })
    }

    fn or_error(self, error_msg: &str) -> Result<T> {
        self.map_err(|e| anyhow!("{}: {}", error_msg, e))
    }
}

impl<T> ErrorUtils<T> for Option<T> {
    fn with_context_msg(self, msg: &str) -> Result<T> {
        self.ok_or_else(|| anyhow!("{}", msg))
    }

    fn with_context_fmt(self, msg: &str, args: &[&dyn std::fmt::Display]) -> Result<T> {
        let formatted_args: Vec<String> = args.iter().map(|a| format!("{}", a)).collect();
        self.ok_or_else(|| anyhow!("{} [{}]", msg, formatted_args.join(", ")))
    }

    fn or_error(self, error_msg: &str) -> Result<T> {
        self.ok_or_else(|| anyhow!("{}", error_msg))
    }
}

/// Test helper functions для reduce boilerplate в тестах
pub mod test_helpers {
    use super::*;
    use tempfile::TempDir;

    /// Create temp directory с proper error handling
    pub fn create_temp_dir(context: &str) -> Result<TempDir> {
        TempDir::new().with_context_msg(&format!("Failed to create temp directory for {}", context))
    }

    /// JSON serialization с контекстом
    pub fn serialize_json<T: serde::Serialize>(value: &T, context: &str) -> Result<String> {
        serde_json::to_string_pretty(value)
            .with_context_msg(&format!("Failed to serialize JSON for {}", context))
    }

    /// JSON deserialization с контекстом
    pub fn deserialize_json<T: serde::de::DeserializeOwned>(
        json: &str,
        context: &str,
    ) -> Result<T> {
        serde_json::from_str(json)
            .with_context_msg(&format!("Failed to deserialize JSON for {}", context))
    }

    /// Database tree operations с контекстом
    pub fn insert_to_tree<K, V>(tree: &sled::Tree, key: K, value: V, context: &str) -> Result<()>
    where
        K: AsRef<[u8]>,
        V: Into<sled::IVec>,
    {
        tree.insert(key, value)
            .with_context_msg(&format!("Failed to insert to tree for {}", context))?;
        Ok(())
    }

    /// Tree flush с контекстом
    pub fn flush_tree(tree: &sled::Tree, context: &str) -> Result<()> {
        tree.flush()
            .with_context_msg(&format!("Failed to flush tree for {}", context))
            .map(|_| ())
    }
}

/// Production helper functions для реального кода
pub mod production_helpers {
    use super::*;
    use std::path::Path;

    /// Database operations с полным error handling
    pub fn open_database<P: AsRef<Path>>(path: P, context: &str) -> Result<sled::Db> {
        let path = path.as_ref();
        sled::open(path).with_context_fmt("Failed to open database", &[&path.display(), &context])
    }

    /// File system operations
    pub fn create_dir_all<P: AsRef<Path>>(path: P, context: &str) -> Result<()> {
        let path = path.as_ref();
        std::fs::create_dir_all(path)
            .with_context_fmt("Failed to create directory", &[&path.display(), &context])
    }

    /// Concurrent task error aggregation
    pub async fn join_tasks<T>(
        handles: Vec<tokio::task::JoinHandle<Result<T>>>,
        operation_name: &str,
    ) -> Result<Vec<T>> {
        let mut results = Vec::with_capacity(handles.len());
        let mut errors = Vec::new();

        for (index, handle) in handles.into_iter().enumerate() {
            match handle.await {
                Ok(Ok(result)) => results.push(result),
                Ok(Err(task_error)) => {
                    errors.push(format!("Task {}: {}", index, task_error));
                }
                Err(join_error) => {
                    errors.push(format!("Task {} join error: {}", index, join_error));
                }
            }
        }

        if !errors.is_empty() {
            return Err(anyhow!(
                "Operation '{}' failed with {} errors: [{}]",
                operation_name,
                errors.len(),
                errors.join("; ")
            ));
        }

        Ok(results)
    }
}

#[cfg(feature = "persistence")]
pub fn open_db_with_context(path: &std::path::Path) -> Result<sled::Db> {
    use crate::utils::fmt_utils::ContextExt;
    sled::open(path).with_context_fmt("Failed to open database", &[&path.display(), &"sled open"])        
}

#[cfg(feature = "persistence")]
pub fn flush_tree(tree: &sled::Tree, context: &str) -> Result<()> {
    use crate::utils::fmt_utils::ContextExt;
    tree.flush().with_context_fmt("Failed to flush tree", &[&context])
}

#[cfg(feature = "persistence")]
pub fn insert_with_context<K, V>(
    tree: &sled::Tree,
    key: K,
    value: V,
    context: &str,
) -> Result<()>
where
    K: AsRef<[u8]>,
    V: Into<sled::IVec>,
{
    use crate::utils::fmt_utils::ContextExt;
    tree.insert(key, value)
        .with_context_fmt("Failed to insert into sled", &[&context])?;
    Ok(())
}

/// Macro для упрощения error conversion в тестах
#[macro_export]
macro_rules! test_ok {
    ($expr:expr) => {
        $expr.map_err(|e| anyhow::anyhow!("Test assertion failed: {}", e))?
    };
    ($expr:expr, $context:expr) => {
        $expr.with_context_msg($context)?
    };
}

/// Macro для создания processor instances с error handling
#[macro_export]
macro_rules! create_processor {
    ($processor_type:ty, $config:expr) => {
        <$processor_type>::new($config)
            .with_context_msg(concat!("Failed to create ", stringify!($processor_type)))
    };
    ($processor_type:ty, $config:expr, $context:expr) => {
        <$processor_type>::new($config).with_context_msg(&format!(
            "Failed to create {} for {}",
            stringify!($processor_type),
            $context
        ))
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_error_utils_result() -> Result<()> {
        let result: Result<i32, &str> = Err("test error");

        let converted = result.with_context_msg("Operation failed");
        assert!(converted.is_err());
        assert!(converted
            .unwrap_err()
            .to_string()
            .contains("Operation failed"));

        Ok(())
    }

    #[test]
    fn test_error_utils_option() -> Result<()> {
        let option: Option<i32> = None;

        let converted = option.with_context_msg("Value not found");
        assert!(converted.is_err());
        assert!(converted
            .unwrap_err()
            .to_string()
            .contains("Value not found"));

        Ok(())
    }

    #[test]
    fn test_create_temp_dir() -> Result<()> {
        let _temp_dir = test_helpers::create_temp_dir("test_create_temp_dir")?;
        Ok(())
    }

    #[test]
    fn test_json_operations() -> Result<()> {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestData {
            value: i32,
        }

        let data = TestData { value: 42 };
        let json = test_helpers::serialize_json(&data, "test_json_operations")?;
        let parsed: TestData = test_helpers::deserialize_json(&json, "test_json_operations")?;

        assert_eq!(data, parsed);

        Ok(())
    }

    #[test]
    fn test_format_args() -> Result<()> {
        let result: Result<i32, &str> = Err("test error");
        let thread_id = 1;
        let operation = "test";

        let converted =
            result.with_context_fmt("Thread operation failed", &[&thread_id, &operation]);

        assert!(converted.is_err());
        let error_msg = converted.unwrap_err().to_string();
        assert!(error_msg.contains("Thread operation failed"));
        assert!(error_msg.contains("1"));
        assert!(error_msg.contains("test"));

        Ok(())
    }

    #[tokio::test]
    async fn test_join_tasks() -> Result<()> {
        use production_helpers::join_tasks;

        // Create successful tasks
        let handles: Vec<_> = (0..3)
            .map(|i| tokio::spawn(async move { Ok(i * 2) }))
            .collect();

        let results = join_tasks(handles, "test_multiplication").await?;
        assert_eq!(results, vec![0, 2, 4]);

        Ok(())
    }

    #[tokio::test]
    async fn test_join_tasks_with_errors() {
        use production_helpers::join_tasks;

        // Create mixed successful and failed tasks
        let handles: Vec<_> = (0..3)
            .map(|i| {
                tokio::spawn(async move {
                    if i == 1 {
                        Err(anyhow!("Task {} failed", i))
                    } else {
                        Ok(i * 2)
                    }
                })
            })
            .collect();

        let result = join_tasks(handles, "test_mixed").await;
        assert!(result.is_err());

        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("test_mixed"));
        assert!(error_msg.contains("Task 1 failed"));
    }
}
