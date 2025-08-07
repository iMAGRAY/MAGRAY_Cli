//! Domain Value Objects - Immutable domain concepts
//!
//! Value objects представляют business concepts без identity.
//! Immutable по определению.

mod access_pattern;
mod layer_type;
mod promotion_criteria;
mod score_threshold;

pub use access_pattern::AccessPattern;
pub use layer_type::LayerType;
pub use promotion_criteria::PromotionCriteria;
pub use score_threshold::ScoreThreshold;
