//! Utility modules для memory crate
//!
//! СОДЕРЖИТ:
//! - Error handling utilities для устранения .unwrap() patterns
//! - Performance optimization utilities
//! - Common patterns consolidation

#[cfg(all(not(feature = "minimal"), feature = "persistence"))]
pub mod error_utils;

#[cfg(all(not(feature = "minimal"), feature = "persistence"))]
pub use error_utils::{production_helpers, test_helpers, ErrorUtils};
