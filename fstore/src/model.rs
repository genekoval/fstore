use time::Date;
use uuid::Uuid;

pub struct Bucket {
    pub id: Uuid,
    pub name: String,
    pub created: Date,
    pub size: i64,
    pub space_used: i64,
}

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

pub struct ObjectError {
    pub object_id: Uuid,
    pub message: String,
}

pub struct RemoveResult {
    pub objects_removed: u64,
    pub space_freed: u64,
}

pub struct StoreTotals {
    pub buckets: u64,
    pub objects: u64,
    pub space_used: u64,
}
