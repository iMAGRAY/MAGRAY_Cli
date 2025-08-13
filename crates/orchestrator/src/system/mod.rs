//! Actor System Core
//!
//! This module provides the main ActorSystem, supervision patterns, and system management
//! components for the multi-agent orchestration system.

pub mod actor_system;
pub mod message;
pub mod supervisor;

pub use actor_system::{ActorSystem, ActorSystemError, SystemConfig};
pub use message::{MessageFilter, MessageRouter, RoutingTable};
pub use supervisor::{BackoffPolicy, RestartStrategy, Supervisor, SupervisorConfig};
