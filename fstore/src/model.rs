use std::time::SystemTime;

use uuid::Uuid;

pub struct Bucket {
    pub id: Uuid,
    pub name: String,
    pub created: SystemTime,
    pub size: i64,
    pub space_used: i64,
}

pub struct Object {
    pub id: Uuid,
    pub hash: String,
    pub size: i64,
    pub ty: String,
    pub subtype: String,
    pub added: SystemTime,
}

impl Object {
    pub fn media_type(&self) -> String {
        format!("{}/{}", self.ty, self.subtype)
    }
}

pub struct StoreTotals {
    pub buckets: i64,
    pub objects: i64,
    pub space_used: i64,
}
