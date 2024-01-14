use serde::{Deserialize, Serialize};
use time::Date;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct Bucket {
    pub id: Uuid,
    pub name: String,
    pub created: Date,
    pub size: i64,
    pub space_used: i64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Object {
    pub id: Uuid,
    pub hash: String,
    pub size: u64,
    pub r#type: String,
    pub subtype: String,
    pub added: Date,
}

impl Object {
    pub fn media_type(&self) -> String {
        format!("{}/{}", self.r#type, self.subtype)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ObjectError {
    pub object_id: Uuid,
    pub message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RemoveResult {
    pub objects_removed: u64,
    pub space_freed: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StoreTotals {
    pub buckets: u64,
    pub objects: u64,
    pub space_used: u64,
}
