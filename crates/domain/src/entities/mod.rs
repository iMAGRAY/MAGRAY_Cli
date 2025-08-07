//! Domain Entities - Core business objects
//!
//! Entities содержат identity и business rules.
//! Независимы от infrastructure concerns.

mod memory_record;
mod embedding_vector;
mod search_query;
mod record_id;

pub use memory_record::MemoryRecord;
pub use embedding_vector::EmbeddingVector;
pub use search_query::SearchQuery;
pub use record_id::RecordId;