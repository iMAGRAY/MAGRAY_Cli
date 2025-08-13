//! Domain Value Objects - Immutable domain concepts
//!
//! Value objects представляют business concepts без identity.
//! Immutable по определению.

pub mod access_pattern;
pub mod layer_type;
mod promotion_criteria;
pub mod score_threshold;

pub use access_pattern::AccessPattern;
pub use layer_type::LayerType;
pub use promotion_criteria::PromotionCriteria;
pub use score_threshold::ScoreThreshold;
