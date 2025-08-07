//! Memory Commands for CQRS Implementation
//!
//! Команды для изменения состояния memory системы

use serde::{Deserialize, Serialize};
use validator::Validate;
use crate::{ApplicationResult, RequestContext};
use crate::dtos::{
    StoreMemoryRequest, StoreMemoryResponse, 
    BatchStoreMemoryRequest, BatchStoreMemoryResponse,
    PromoteRecordsRequest, PromoteRecordsResponse
};
use domain::value_objects::layer_type::LayerType;

/// Command to store a single memory record
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct StoreMemoryCommand {
    #[validate(length(min = 1, max = 100000))]
    pub content: String,
    pub metadata: Option<serde_json::Value>,
    pub project: Option<String>,
    pub target_layer: Option<LayerType>,
    pub priority: Option<u8>,
    pub tags: Vec<String>,
}

/// Command to store multiple memory records
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct BatchStoreMemoryCommand {
    #[validate(length(min = 1, max = 100))]
    pub records: Vec<StoreMemoryRequest>,
    pub parallel_processing: bool,
    pub failure_tolerance: crate::dtos::FailureTolerance,
    pub progress_reporting: bool,
}

/// Command to promote memory records between layers
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct PromoteRecordsCommand {
    #[validate(length(min = 1))]
    pub criteria: Vec<crate::dtos::PromotionCriterion>,
    pub max_candidates: Option<usize>,
    pub dry_run: bool,
}

/// Command to delete a memory record
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DeleteMemoryCommand {
    #[validate(length(min = 1))]
    pub record_id: String,
    pub force_delete: bool,
    pub cascade_delete: bool,
}

/// Command to update memory record metadata
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateMemoryMetadataCommand {
    #[validate(length(min = 1))]
    pub record_id: String,
    pub metadata: serde_json::Value,
    pub tags: Option<Vec<String>>,
    pub project: Option<String>,
}

/// Command to backup memory layers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMemoryCommand {
    pub layers: Vec<LayerType>,
    pub backup_location: String,
    pub compression_enabled: bool,
    pub include_metadata: bool,
}

/// Command to optimize memory indices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizeIndicesCommand {
    pub layers: Vec<LayerType>,
    pub optimization_type: OptimizationType,
    pub force_rebuild: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationType {
    Compact,
    Reindex,
    Vacuum,
    Full,
}

// Command implementations using the CQRS macro
crate::impl_cqrs!(command StoreMemoryCommand => StoreMemoryResponse);
crate::impl_cqrs!(command BatchStoreMemoryCommand => BatchStoreMemoryResponse);
crate::impl_cqrs!(command PromoteRecordsCommand => PromoteRecordsResponse);
crate::impl_cqrs!(command DeleteMemoryCommand => DeleteMemoryResponse);
crate::impl_cqrs!(command UpdateMemoryMetadataCommand => UpdateMemoryMetadataResponse);
crate::impl_cqrs!(command BackupMemoryCommand => BackupMemoryResponse);
crate::impl_cqrs!(command OptimizeIndicesCommand => OptimizeIndicesResponse);

/// Response for delete memory command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteMemoryResponse {
    pub record_id: String,
    pub deleted: bool,
    pub cascade_count: usize,
    pub freed_space_bytes: u64,
}

/// Response for update metadata command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateMemoryMetadataResponse {
    pub record_id: String,
    pub updated: bool,
    pub previous_metadata: Option<serde_json::Value>,
    pub metadata_size_bytes: u64,
}

/// Response for backup command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMemoryResponse {
    pub backup_id: String,
    pub backup_location: String,
    pub backed_up_records: usize,
    pub backup_size_bytes: u64,
    pub backup_duration_ms: u64,
    pub compression_ratio: Option<f32>,
}

/// Response for optimize indices command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizeIndicesResponse {
    pub optimization_id: String,
    pub optimized_layers: Vec<LayerType>,
    pub optimization_duration_ms: u64,
    pub space_saved_bytes: u64,
    pub performance_improvement_percent: f32,
}

impl From<StoreMemoryRequest> for StoreMemoryCommand {
    fn from(request: StoreMemoryRequest) -> Self {
        Self {
            content: request.content,
            metadata: request.metadata,
            project: request.project,
            target_layer: request.target_layer,
            priority: request.priority,
            tags: request.tags,
        }
    }
}

impl From<BatchStoreMemoryRequest> for BatchStoreMemoryCommand {
    fn from(request: BatchStoreMemoryRequest) -> Self {
        Self {
            records: request.records,
            parallel_processing: request.options.parallel_processing,
            failure_tolerance: request.options.failure_tolerance,
            progress_reporting: request.options.progress_reporting,
        }
    }
}

impl From<PromoteRecordsRequest> for PromoteRecordsCommand {
    fn from(request: PromoteRecordsRequest) -> Self {
        Self {
            criteria: request.criteria,
            max_candidates: request.max_candidates,
            dry_run: request.dry_run,
        }
    }
}

impl StoreMemoryCommand {
    pub fn new(content: &str) -> Self {
        Self {
            content: content.to_string(),
            metadata: None,
            project: None,
            target_layer: None,
            priority: None,
            tags: Vec::new(),
        }
    }

    pub fn with_project(mut self, project: &str) -> Self {
        self.project = Some(project.to_string());
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = Some(priority);
        self
    }

    pub fn with_layer(mut self, layer: LayerType) -> Self {
        self.target_layer = Some(layer);
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

impl BatchStoreMemoryCommand {
    pub fn new(records: Vec<StoreMemoryRequest>) -> Self {
        Self {
            records,
            parallel_processing: true,
            failure_tolerance: crate::dtos::FailureTolerance::Partial,
            progress_reporting: false,
        }
    }

    pub fn with_serial_processing(mut self) -> Self {
        self.parallel_processing = false;
        self
    }

    pub fn with_strict_failure_tolerance(mut self) -> Self {
        self.failure_tolerance = crate::dtos::FailureTolerance::Strict;
        self
    }

    pub fn with_progress_reporting(mut self) -> Self {
        self.progress_reporting = true;
        self
    }
}

impl PromoteRecordsCommand {
    pub fn new(criteria: Vec<crate::dtos::PromotionCriterion>) -> Self {
        Self {
            criteria,
            max_candidates: None,
            dry_run: false,
        }
    }

    pub fn with_limit(mut self, max_candidates: usize) -> Self {
        self.max_candidates = Some(max_candidates);
        self
    }

    pub fn as_dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }
}

impl DeleteMemoryCommand {
    pub fn new(record_id: &str) -> Self {
        Self {
            record_id: record_id.to_string(),
            force_delete: false,
            cascade_delete: false,
        }
    }

    pub fn with_force(mut self) -> Self {
        self.force_delete = true;
        self
    }

    pub fn with_cascade(mut self) -> Self {
        self.cascade_delete = true;
        self
    }
}

impl UpdateMemoryMetadataCommand {
    pub fn new(record_id: &str, metadata: serde_json::Value) -> Self {
        Self {
            record_id: record_id.to_string(),
            metadata,
            tags: None,
            project: None,
        }
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = Some(tags);
        self
    }

    pub fn with_project(mut self, project: &str) -> Self {
        self.project = Some(project.to_string());
        self
    }
}

impl BackupMemoryCommand {
    pub fn new(backup_location: &str) -> Self {
        Self {
            layers: vec![LayerType::Cache, LayerType::Index, LayerType::Storage],
            backup_location: backup_location.to_string(),
            compression_enabled: true,
            include_metadata: true,
        }
    }

    pub fn for_layers(mut self, layers: Vec<LayerType>) -> Self {
        self.layers = layers;
        self
    }

    pub fn without_compression(mut self) -> Self {
        self.compression_enabled = false;
        self
    }

    pub fn without_metadata(mut self) -> Self {
        self.include_metadata = false;
        self
    }
}

impl OptimizeIndicesCommand {
    pub fn new(optimization_type: OptimizationType) -> Self {
        Self {
            layers: vec![LayerType::Index],
            optimization_type,
            force_rebuild: false,
        }
    }

    pub fn for_layers(mut self, layers: Vec<LayerType>) -> Self {
        self.layers = layers;
        self
    }

    pub fn with_force_rebuild(mut self) -> Self {
        self.force_rebuild = true;
        self
    }

    pub fn full_optimization() -> Self {
        Self {
            layers: vec![LayerType::Cache, LayerType::Index, LayerType::Storage],
            optimization_type: OptimizationType::Full,
            force_rebuild: true,
        }
    }
}

impl Default for OptimizationType {
    fn default() -> Self {
        Self::Compact
    }
}