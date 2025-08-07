//! Domain Services - Pure business logic services
//!
//! Содержит сложную business logic которая не вписывается в entities
//! Оперирует entities и value objects, использует repository abstractions

mod memory_domain_service;
mod search_domain_service;
mod promotion_domain_service;

pub use memory_domain_service::MemoryDomainService;
pub use search_domain_service::SearchDomainService;
pub use promotion_domain_service::PromotionDomainService;