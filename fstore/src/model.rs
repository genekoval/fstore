use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bucket {
    pub id: Uuid,
    pub name: String,
    pub created: DateTime<Local>,
    pub object_count: i64,
    pub space_used: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    pub id: Uuid,
    pub hash: String,
    pub size: u64,
    pub r#type: String,
    pub subtype: String,
    pub added: DateTime<Local>,
}

impl Object {
    pub fn media_type(&self) -> String {
        format!("{}/{}", self.r#type, self.subtype)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectError {
    pub object_id: Uuid,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveResult {
    pub objects_removed: u64,
    pub space_freed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreTotals {
    pub buckets: u64,
    pub objects: u64,
    pub space_used: u64,
}
