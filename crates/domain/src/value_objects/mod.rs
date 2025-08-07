//! Domain Value Objects - Immutable domain concepts
//!
//! Value objects представляют business concepts без identity.
//! Immutable по определению.

mod layer_type;
mod score_threshold;
mod access_pattern;
mod promotion_criteria;

pub use layer_type::LayerType;
pub use score_threshold::ScoreThreshold;
pub use access_pattern::AccessPattern;
pub use promotion_criteria::PromotionCriteria;