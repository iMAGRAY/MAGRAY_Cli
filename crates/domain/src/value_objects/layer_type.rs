//! LayerType - Business domain concept for memory layers
//!
//! Pure domain value object independent of storage implementation

use serde::{Deserialize, Serialize};
use std::fmt;

/// Business layers in the memory system
///
/// Represents the 3-tier memory architecture:
/// - Interact: Hot, frequently accessed data (24h TTL)
/// - Insights: Distilled knowledge (90d TTL)  
/// - Assets: Cold, permanent storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord, Default)]
pub enum LayerType {
    /// L1 - Hot context, frequent access (24h TTL)
    #[default]
    Interact,
    /// L2 - Distilled knowledge (90d TTL)
    Insights,
    /// L3 - Cold artifacts (permanent)
    Assets,
}

impl LayerType {
    /// Get human-readable layer name
    pub fn as_str(&self) -> &'static str {
        match self {
            LayerType::Interact => "interact",
            LayerType::Insights => "insights",
            LayerType::Assets => "assets",
        }
    }

    /// Get business description of the layer
    pub fn description(&self) -> &'static str {
        match self {
            LayerType::Interact => "Hot context - frequently accessed data",
            LayerType::Insights => "Distilled knowledge - important insights",
            LayerType::Assets => "Cold storage - permanent artifacts",
        }
    }

    /// Get TTL in hours for this layer (business rule)
    pub fn ttl_hours(&self) -> Option<u64> {
        match self {
            LayerType::Interact => Some(24),      // 24 hours
            LayerType::Insights => Some(90 * 24), // 90 days
            LayerType::Assets => None,            // Permanent
        }
    }

    /// Check if this layer can be promoted to target layer
    pub fn can_promote_to(&self, target: LayerType) -> bool {
        matches!((self, target), (LayerType::Interact, LayerType::Insights) | (LayerType::Insights, LayerType::Assets))
    }

    /// Get next layer in promotion hierarchy
    pub fn next_layer(&self) -> Option<LayerType> {
        match self {
            LayerType::Interact => Some(LayerType::Insights),
            LayerType::Insights => Some(LayerType::Assets),
            LayerType::Assets => None,
        }
    }

    /// Get previous layer in hierarchy
    pub fn previous_layer(&self) -> Option<LayerType> {
        match self {
            LayerType::Interact => None,
            LayerType::Insights => Some(LayerType::Interact),
            LayerType::Assets => Some(LayerType::Insights),
        }
    }

    /// Get business priority level (higher = more important)
    pub fn priority(&self) -> u8 {
        match self {
            LayerType::Interact => 1, // Highest priority - hot data
            LayerType::Insights => 2, // Medium priority - warm data
            LayerType::Assets => 3,   // Lowest priority - cold data
        }
    }

    /// Parse from string representation
    pub fn parse_str(s: &str) -> Result<Self, crate::errors::DomainError> {
        match s.to_lowercase().as_str() {
            "interact" => Ok(LayerType::Interact),
            "insights" => Ok(LayerType::Insights),
            "assets" => Ok(LayerType::Assets),
            _ => Err(crate::errors::DomainError::InvalidLayerType(s.to_string())),
        }
    }

    /// Get all possible layers in order
    pub fn all_layers() -> Vec<LayerType> {
        vec![LayerType::Interact, LayerType::Insights, LayerType::Assets]
    }
}

// Default now derived

impl fmt::Display for LayerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_ordering() {
        let layers = LayerType::all_layers();
        assert_eq!(layers[0], LayerType::Interact);
        assert_eq!(layers[1], LayerType::Insights);
        assert_eq!(layers[2], LayerType::Assets);
    }

    #[test]
    fn test_promotion_rules() {
        assert!(LayerType::Interact.can_promote_to(LayerType::Insights));
        assert!(LayerType::Insights.can_promote_to(LayerType::Assets));
        assert!(!LayerType::Assets.can_promote_to(LayerType::Interact));
        assert!(!LayerType::Interact.can_promote_to(LayerType::Assets)); // Skip layer not allowed
    }

    #[test]
    fn test_ttl_rules() {
        assert_eq!(LayerType::Interact.ttl_hours(), Some(24));
        assert_eq!(LayerType::Insights.ttl_hours(), Some(90 * 24));
        assert_eq!(LayerType::Assets.ttl_hours(), None);
    }

    #[test]
    fn test_string_conversion() {
        assert_eq!(LayerType::Interact.as_str(), "interact");
        assert_eq!(
            LayerType::parse_str("insights").unwrap(),
            LayerType::Insights
        );
        assert!(LayerType::parse_str("invalid").is_err());
    }

    #[test]
    fn test_priority_levels() {
        assert!(LayerType::Interact.priority() < LayerType::Insights.priority());
        assert!(LayerType::Insights.priority() < LayerType::Assets.priority());
    }
}
