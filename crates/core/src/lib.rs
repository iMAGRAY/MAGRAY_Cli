//! Core domain models and contracts for MAGRAY
//!
//! This crate contains the domain layer following Domain-Driven Design principles:
//! - Task, Intent, Plan, ToolSpec, MemoryRecord, Capability
//! - Contracts: Tool, LlmClient, VectorIndex, DocStore, Policy
//! - EventBus infrastructure
//!
//! Following ARCHITECTURE_PLAN_ADVANCED.md requirements

pub mod contracts;
pub mod domain;
pub mod events;

pub use contracts::*;
pub use domain::*;
pub use events::*;

/// Re-export common types
pub use anyhow::{Error, Result};
pub use chrono::{DateTime, Utc};
pub use serde::{Deserialize, Serialize};
pub use uuid::Uuid;
