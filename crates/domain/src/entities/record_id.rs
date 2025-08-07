//! RecordId - Unique identifier for memory records
//!
//! Value object representing record identity in domain

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for memory records
///
/// This is a domain value object that represents record identity
/// Independent of storage implementation (UUID is just internal representation)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecordId(Uuid);

impl RecordId {
    /// Create new unique RecordId
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create RecordId from string representation
    /// Used for deserialization and external ID handling
    pub fn from_string(id: &str) -> Result<Self, crate::errors::DomainError> {
        let uuid = Uuid::parse_str(id)
            .map_err(|_| crate::errors::DomainError::InvalidRecordId(id.to_string()))?;
        Ok(Self(uuid))
    }

    /// Get string representation
    pub fn as_string(&self) -> String {
        self.0.to_string()
    }

    /// Get internal UUID representation
    /// NOTE: This should only be used by infrastructure layer
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for RecordId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RecordId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for RecordId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_id_creation() {
        let id1 = RecordId::new();
        let id2 = RecordId::new();

        // Each ID should be unique
        assert_ne!(id1, id2);

        // Should be able to convert to string and back
        let id_str = id1.to_string();
        let id3 = RecordId::from_string(&id_str).unwrap();
        assert_eq!(id1, id3);
    }

    #[test]
    fn test_record_id_validation() {
        // Valid UUID string should work
        let valid_id = "550e8400-e29b-41d4-a716-446655440000";
        assert!(RecordId::from_string(valid_id).is_ok());

        // Invalid string should fail
        let invalid_id = "not-a-uuid";
        assert!(RecordId::from_string(invalid_id).is_err());
    }
}
