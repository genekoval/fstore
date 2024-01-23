use chrono::{DateTime, Local};
use sqlx::{Encode, Postgres};
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct Bucket {
    pub bucket_id: Uuid,
    pub name: String,
    pub date_created: DateTime<Local>,
    pub object_count: i64,
    pub space_used: i64,
}

impl From<Bucket> for fstore::Bucket {
    fn from(value: Bucket) -> Self {
        fstore::Bucket {
            id: value.bucket_id,
            name: value.name,
            created: value.date_created,
            object_count: value.object_count,
            space_used: value.space_used,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct Object {
    pub object_id: Uuid,
    pub hash: String,
    pub size: i64,
    pub r#type: String,
    pub subtype: String,
    pub date_added: DateTime<Local>,
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

#[derive(Debug, sqlx::FromRow)]
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

#[derive(Debug, sqlx::FromRow)]
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

#[derive(Debug, sqlx::FromRow)]
pub struct ObjectError {
    pub object_id: Uuid,
    pub message: String,
}

impl From<ObjectError> for fstore::ObjectError {
    fn from(value: ObjectError) -> Self {
        fstore::ObjectError {
            object_id: value.object_id,
            message: value.message,
        }
    }
}

impl sqlx::Type<Postgres> for ObjectError {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("object_error")
    }
}

impl<'r> sqlx::Decode<'r, Postgres> for ObjectError {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        let mut decoder = sqlx::postgres::types::PgRecordDecoder::new(value)?;

        let object_id = decoder.try_decode::<Uuid>()?;
        let message = decoder.try_decode::<String>()?;

        Ok(Self { object_id, message })
    }
}

impl<'r> sqlx::Encode<'r, Postgres> for ObjectError {
    fn encode_by_ref(
        &self,
        buf: &mut <Postgres as sqlx::database::HasArguments<'r>>::ArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        let _ = self.object_id.encode_by_ref(buf);
        <std::string::String as Encode<'_, Postgres>>::encode_by_ref(
            &self.message,
            buf,
        )
    }
}

pub struct ObjectErrorSlice<'a>(&'a [ObjectError]);

impl<'a> sqlx::Type<Postgres> for ObjectErrorSlice<'a> {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("_object_error")
    }
}

impl<'r> sqlx::Encode<'r, Postgres> for ObjectErrorSlice<'r> {
    fn encode_by_ref(
        &self,
        buf: &mut sqlx::postgres::PgArgumentBuffer,
    ) -> sqlx::encode::IsNull {
        self.0.encode(buf)
    }
}

pub struct ObjectErrorVec(Vec<ObjectError>);

impl sqlx::Type<Postgres> for ObjectErrorVec {
    fn type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("_object_error")
    }
}

impl<'r> sqlx::Decode<'r, Postgres> for ObjectErrorVec {
    fn decode(
        value: sqlx::postgres::PgValueRef<'r>,
    ) -> Result<Self, Box<dyn std::error::Error + 'static + Send + Sync>> {
        Ok(Self(Vec::<ObjectError>::decode(value)?))
    }
}
