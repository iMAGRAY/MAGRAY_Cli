//! Utility modules для memory crate
//!
//! СОДЕРЖИТ:
//! - Error handling utilities для устранения .unwrap() patterns
//! - Performance optimization utilities
//! - Common patterns consolidation

pub mod error_utils;

pub use error_utils::{production_helpers, test_helpers, ErrorUtils};
