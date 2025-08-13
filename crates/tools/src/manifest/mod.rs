// Tool Manifest Validation Module
// P1.2.2 Tool Manifest Validation Implementation
// P1.2.3 Enhanced with Capability System Integration

pub mod schema;
pub mod validation;

pub use schema::*;
pub use validation::*;

// Re-export capability system for manifest integration
pub use crate::capabilities::{Capability, CapabilityError, CapabilitySpec};
