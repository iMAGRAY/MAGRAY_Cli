//! Data Transfer Objects (DTOs)
//!
//! DTOs изолируют Application Layer от Domain entities
//! и предоставляют стабильные contracts для внешних клиентов.

pub mod analytics;
pub mod memory;
pub mod promotion;
pub mod promotion_criterion;
pub mod search;

// Re-export common DTOs
pub use analytics::*;
pub use memory::*;
pub use promotion::*;
pub use promotion_criterion::*;
pub use search::*;

use serde::{Deserialize, Serialize};
use validator::Validate;

/// Base response wrapper
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub request_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ResponseMetadata {
    pub execution_time_ms: u64,
    pub resource_usage: Option<ResourceUsage>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResourceUsage {
    pub memory_mb: f64,
    pub cpu_percent: f64,
    pub gpu_memory_mb: Option<f64>,
    pub cache_hit_rate: f64,
}

/// Pagination parameters
#[derive(Debug, Serialize, Deserialize, Clone, Validate)]
pub struct PaginationParams {
    #[validate(range(min = 1, max = 1000))]
    pub limit: u32,

    #[validate(range(min = 0))]
    pub offset: u32,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            limit: 50,
            offset: 0,
        }
    }
}

/// Pagination response metadata
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaginationMeta {
    pub total_count: u64,
    pub page_count: u32,
    pub current_page: u32,
    pub has_next: bool,
    pub has_previous: bool,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T, request_id: String) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            request_id,
            timestamp: chrono::Utc::now(),
            metadata: ResponseMetadata::default(),
        }
    }

    pub fn error(error: String, request_id: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            request_id,
            timestamp: chrono::Utc::now(),
            metadata: ResponseMetadata::default(),
        }
    }

    pub fn with_metadata(mut self, metadata: ResponseMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}
