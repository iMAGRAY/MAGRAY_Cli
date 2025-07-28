use serde::{Serialize, Deserialize};
use uuid::Uuid;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub id: Uuid,
    pub goal: String,
    pub params: serde_json::Value,
    pub project_id: String,
}
