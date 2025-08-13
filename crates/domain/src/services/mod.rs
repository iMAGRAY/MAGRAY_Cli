//! Domain Services - Pure business logic services
//!
//! Содержит сложную business logic которая не вписывается в entities
//! Оперирует entities и value objects, использует repository abstractions

pub mod memory_domain_service;
pub mod promotion_domain_service;
pub mod search_domain_service;

pub use memory_domain_service::{MemoryDomainService, MemoryDomainServiceTrait};
pub use promotion_domain_service::{PromotionDomainService, PromotionDomainServiceTrait};
pub use search_domain_service::{SearchDomainService, SearchDomainServiceTrait};
