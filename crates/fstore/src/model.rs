pub use uuid::Uuid;

use chrono::Local;
use serde::{Deserialize, Serialize};

pub type DateTime = chrono::DateTime<Local>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct About {
    pub version: Version,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Version {
    pub number: String,
    pub branch: String,
    pub build_time: String,
    pub build_os: String,
    pub build_type: String,
    pub commit_hash: String,
    pub commit_date: String,
    pub rust_version: String,
    pub rust_channel: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bucket {
    pub id: Uuid,
    pub name: String,
    pub created: DateTime,
    pub object_count: u64,
    pub space_used: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Object {
    pub id: Uuid,
    pub hash: String,
    pub size: u64,
    pub r#type: String,
    pub subtype: String,
    pub extension: Option<String>,
    pub added: DateTime,
}

impl Object {
    pub fn media_type(&self) -> String {
        format!("{}/{}", self.r#type, self.subtype)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ObjectSummary {
    pub media_type: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectError {
    pub object_id: Uuid,
    pub message: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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
