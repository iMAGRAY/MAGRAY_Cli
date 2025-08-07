//! Domain Entities - Core business objects
//!
//! Entities содержат identity и business rules.
//! Независимы от infrastructure concerns.

mod embedding_vector;
mod memory_record;
mod record_id;
mod search_query;

pub use embedding_vector::EmbeddingVector;
pub use memory_record::MemoryRecord;
pub use record_id::RecordId;
pub use search_query::SearchQuery;
