use chrono::{DateTime, Local};
use sqlx::{
    encode::IsNull,
    error::BoxDynError,
    postgres::{
        types::PgRecordEncoder, PgArgumentBuffer, PgHasArrayType, PgTypeInfo,
    },
    Encode, FromRow, Postgres, Type,
};
use uuid::Uuid;

pub type Timestamp = DateTime<Local>;

#[derive(Debug, FromRow)]
pub struct Bucket {
    pub bucket_id: Uuid,
    pub name: String,
    pub date_created: Timestamp,
    pub object_count: i64,
    pub space_used: i64,
}

impl From<Bucket> for fstore::Bucket {
    fn from(value: Bucket) -> Self {
        fstore::Bucket {
            id: value.bucket_id,
            name: value.name,
            created: value.date_created,
            object_count: value.object_count.try_into().unwrap(),
            space_used: value.space_used.try_into().unwrap(),
        }
    }
}

#[derive(Debug, FromRow)]
pub struct Object {
    pub object_id: Uuid,
    pub hash: String,
    pub size: i64,
    pub r#type: String,
    pub subtype: String,
    pub date_added: Timestamp,
}

impl From<Object> for fstore::Object {
    fn from(value: Object) -> Self {
        fstore::Object {
            id: value.object_id,
            hash: value.hash,
            size: value.size.try_into().unwrap(),
            r#type: value.r#type,
            subtype: value.subtype,
            added: value.date_added,
        }
    }
}

#[derive(Debug, FromRow)]
pub struct RemoveResult {
    pub objects_removed: i64,
    pub space_freed: i64,
}

impl From<RemoveResult> for fstore::RemoveResult {
    fn from(value: RemoveResult) -> Self {
        fstore::RemoveResult {
            objects_removed: value.objects_removed.try_into().unwrap(),
            space_freed: value.space_freed.try_into().unwrap(),
        }
    }
}

#[derive(Debug, FromRow)]
pub struct StoreTotals {
    pub buckets: i64,
    pub objects: i64,
    pub space_used: i64,
}

impl From<StoreTotals> for fstore::StoreTotals {
    fn from(value: StoreTotals) -> Self {
        fstore::StoreTotals {
            buckets: value.buckets.try_into().unwrap(),
            objects: value.objects.try_into().unwrap(),
            space_used: value.space_used.try_into().unwrap(),
        }
    }
}

#[derive(Debug, FromRow)]
#[sqlx(type_name = "object_error")]
pub struct ObjectError {
    pub object_id: Uuid,
    pub message: String,
}

impl Encode<'_, Postgres> for ObjectError {
    fn encode_by_ref(
        &self,
        buf: &mut PgArgumentBuffer,
    ) -> Result<IsNull, BoxDynError> {
        let mut encoder = PgRecordEncoder::new(buf);

        encoder.encode(self.object_id)?;
        encoder.encode(&self.message)?;

        encoder.finish();
        Ok(IsNull::No)
    }
}

impl Type<Postgres> for ObjectError {
    fn type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("object_error")
    }
}

impl PgHasArrayType for ObjectError {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        PgTypeInfo::with_name("_object_error")
    }
}

impl From<ObjectError> for fstore::ObjectError {
    fn from(value: ObjectError) -> Self {
        fstore::ObjectError {
            object_id: value.object_id,
            message: value.message,
        }
    }
}
