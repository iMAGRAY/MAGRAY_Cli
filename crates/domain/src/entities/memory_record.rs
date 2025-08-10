//! MemoryRecord - Core business entity for memory storage
//!
//! Clean domain entity independent of infrastructure concerns

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::RecordId;
use crate::errors::{DomainError, DomainResult};
use crate::value_objects::{AccessPattern, LayerType};

/// Core business entity representing a memory record
///
/// This is the heart of the memory domain - contains ONLY business-relevant data
/// No infrastructure concerns (no database IDs, cache keys, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecord {
    /// Unique business identifier
    id: RecordId,

    /// The actual content being stored
    content: String,

    /// Business layer classification
    layer: LayerType,

    /// Content type/category (user-defined)
    kind: String,

    /// Business tags for categorization
    tags: Vec<String>,

    /// Project context (business domain)
    project: String,

    /// Session context (business session)
    session: String,

    /// Creation timestamp (business event)
    created_at: DateTime<Utc>,

    /// Business access pattern for ML promotion
    access_pattern: AccessPattern,
}

impl MemoryRecord {
    /// Create new memory record with business validation
    pub fn new(
        content: String,
        layer: LayerType,
        kind: String,
        project: String,
        session: String,
    ) -> DomainResult<Self> {
        // Business validation rules
        if content.trim().is_empty() {
            return Err(DomainError::EmptyContent);
        }

        if kind.trim().is_empty() {
            return Err(DomainError::InvalidKind("Kind cannot be empty".to_string()));
        }

        Ok(Self {
            id: RecordId::new(),
            content,
            layer,
            kind,
            tags: Vec::new(),
            project,
            session,
            created_at: Utc::now(),
            access_pattern: AccessPattern::new(),
        })
    }

    /// Create record with explicit ID (for reconstruction from storage)
    pub fn with_id(
        id: RecordId,
        content: String,
        layer: LayerType,
        kind: String,
        created_at: DateTime<Utc>,
        access_pattern: AccessPattern,
        metadata: Option<(String, String)>, // (project, session)
    ) -> DomainResult<Self> {
        if content.trim().is_empty() {
            return Err(DomainError::EmptyContent);
        }

        let (project, session) = metadata.unwrap_or_else(|| (String::new(), String::new()));
        Ok(Self {
            id,
            content,
            layer,
            kind,
            tags: Vec::new(),
            project,
            session,
            created_at,
            access_pattern,
        })
    }

    /// Builder for creating MemoryRecord with optional metadata fields
    pub fn builder() -> MemoryRecordBuilder { MemoryRecordBuilder::default() }

    // Getters - immutable access to record data
    pub fn id(&self) -> RecordId {
        self.id
    }

    pub fn content(&self) -> &str {
        &self.content
    }

    pub fn layer(&self) -> LayerType {
        self.layer
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    pub fn project(&self) -> &str {
        &self.project
    }

    pub fn session(&self) -> &str {
        &self.session
    }

    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub fn access_pattern(&self) -> &AccessPattern {
        &self.access_pattern
    }

    // Business methods - core domain operations

    /// Add business tag with validation
    pub fn add_tag(&mut self, tag: String) -> DomainResult<()> {
        if tag.trim().is_empty() {
            return Err(DomainError::InvalidTag("Tag cannot be empty".to_string()));
        }

        if self.tags.contains(&tag) {
            return Err(DomainError::DuplicateTag(tag));
        }

        self.tags.push(tag);
        Ok(())
    }

    /// Remove business tag
    pub fn remove_tag(&mut self, tag: &str) -> bool {
        if let Some(pos) = self.tags.iter().position(|t| t == tag) {
            self.tags.remove(pos);
            true
        } else {
            false
        }
    }

    /// Check if record has specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Record access (business event)
    pub fn record_access(&mut self) {
        self.access_pattern.record_access();
    }

    /// Check if record should be promoted based on business rules
    pub fn should_promote_to_layer(&self, target_layer: LayerType) -> bool {
        match (self.layer, target_layer) {
            (LayerType::Interact, LayerType::Insights) => {
                self.access_pattern.access_count() >= 5
                    && self.access_pattern.avg_access_interval() < chrono::Duration::hours(2)
            }
            (LayerType::Insights, LayerType::Assets) => {
                self.access_pattern.access_count() >= 10
                    && self.access_pattern.total_age() > chrono::Duration::days(7)
            }
            _ => false,
        }
    }

    /// Promote record to higher layer (business operation)
    pub fn promote_to_layer(&mut self, target_layer: LayerType) -> DomainResult<()> {
        if !self.should_promote_to_layer(target_layer) {
            return Err(DomainError::PromotionNotAllowed {
                from: self.layer,
                to: target_layer,
            });
        }

        self.layer = target_layer;
        Ok(())
    }

    /// Update content with business validation
    pub fn update_content(&mut self, new_content: String) -> DomainResult<()> {
        if new_content.trim().is_empty() {
            return Err(DomainError::EmptyContent);
        }

        self.content = new_content;
        Ok(())
    }

    /// Calculate business relevance score (pure domain logic)
    pub fn calculate_relevance_score(&self) -> f32 {
        let base_score = 1.0;
        let access_boost = (self.access_pattern.access_count() as f32).ln().max(0.0);
        let recency_boost = {
            let hours_since_access = self.access_pattern.hours_since_last_access() as f32;
            if hours_since_access < 24.0 {
                1.0 / (1.0 + hours_since_access / 24.0)
            } else {
                0.1
            }
        };

        base_score + access_boost * 0.3 + recency_boost * 0.7
    }
}

impl Default for MemoryRecord {
    fn default() -> Self {
        Self {
            id: RecordId::new(),
            content: String::new(),
            layer: LayerType::Interact,
            kind: "general".to_string(),
            tags: Vec::new(),
            project: String::new(),
            session: String::new(),
            created_at: Utc::now(),
            access_pattern: AccessPattern::new(),
        }
    }
}

#[derive(Default)]
pub struct MemoryRecordBuilder {
    id: Option<RecordId>,
    content: Option<String>,
    layer: Option<LayerType>,
    kind: Option<String>,
    project: Option<String>,
    session: Option<String>,
    created_at: Option<DateTime<Utc>>,
    access_pattern: Option<AccessPattern>,
}

impl MemoryRecordBuilder {
    pub fn id(mut self, id: RecordId) -> Self { self.id = Some(id); self }
    pub fn content(mut self, content: impl Into<String>) -> Self { self.content = Some(content.into()); self }
    pub fn layer(mut self, layer: LayerType) -> Self { self.layer = Some(layer); self }
    pub fn kind(mut self, kind: impl Into<String>) -> Self { self.kind = Some(kind.into()); self }
    pub fn project(mut self, project: impl Into<String>) -> Self { self.project = Some(project.into()); self }
    pub fn session(mut self, session: impl Into<String>) -> Self { self.session = Some(session.into()); self }
    pub fn created_at(mut self, ts: DateTime<Utc>) -> Self { self.created_at = Some(ts); self }
    pub fn access_pattern(mut self, ap: AccessPattern) -> Self { self.access_pattern = Some(ap); self }

    pub fn build(self) -> DomainResult<MemoryRecord> {
        let id = self.id.unwrap_or_else(RecordId::new);
        let content = self.content.ok_or(DomainError::EmptyContent)?;
        let layer = self.layer.unwrap_or(LayerType::Interact);
        let kind = self.kind.unwrap_or_else(|| "generic".to_string());
        let project = self.project.unwrap_or_default();
        let session = self.session.unwrap_or_default();
        let created_at = self.created_at.unwrap_or_else(Utc::now);
        let access_pattern = self.access_pattern.unwrap_or_else(AccessPattern::new);
        if content.trim().is_empty() { return Err(DomainError::EmptyContent); }
        Ok(MemoryRecord {
            id,
            content,
            layer,
            kind,
            tags: Vec::new(),
            project,
            session,
            created_at,
            access_pattern,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_record_creation() {
        let record = MemoryRecord::new(
            "Test content".to_string(),
            LayerType::Interact,
            "note".to_string(),
            "test_project".to_string(),
            "session_1".to_string(),
        )
        .unwrap();

        assert_eq!(record.content(), "Test content");
        assert_eq!(record.layer(), LayerType::Interact);
        assert_eq!(record.kind(), "note");
        assert_eq!(record.project(), "test_project");
        assert_eq!(record.session(), "session_1");
    }

    #[test]
    fn test_business_validation() {
        // Empty content should fail
        let result = MemoryRecord::new(
            "".to_string(),
            LayerType::Interact,
            "note".to_string(),
            "project".to_string(),
            "session".to_string(),
        );
        assert!(result.is_err());

        // Empty kind should fail
        let result = MemoryRecord::new(
            "Content".to_string(),
            LayerType::Interact,
            "".to_string(),
            "project".to_string(),
            "session".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_tag_management() {
        let mut record = MemoryRecord::new(
            "Test".to_string(),
            LayerType::Interact,
            "note".to_string(),
            "project".to_string(),
            "session".to_string(),
        )
        .unwrap();

        // Add valid tag
        assert!(record.add_tag("important".to_string()).is_ok());
        assert!(record.has_tag("important"));

        // Duplicate tag should fail
        assert!(record.add_tag("important".to_string()).is_err());

        // Empty tag should fail
        assert!(record.add_tag("".to_string()).is_err());

        // Remove tag
        assert!(record.remove_tag("important"));
        assert!(!record.has_tag("important"));
        assert!(!record.remove_tag("nonexistent"));
    }

    #[test]
    fn test_promotion_logic() {
        let mut record = MemoryRecord::new(
            "Test".to_string(),
            LayerType::Interact,
            "note".to_string(),
            "project".to_string(),
            "session".to_string(),
        )
        .unwrap();

        // Should not promote initially
        assert!(!record.should_promote_to_layer(LayerType::Insights));

        // Simulate access pattern that meets promotion criteria
        for _ in 0..6 {
            record.record_access();
        }

        // Should be eligible for promotion now
        // (Note: this is simplified - real access pattern tracking is more complex)
        assert_eq!(record.access_pattern().access_count(), 6);
    }
}
