use sqlx::{Encode, Postgres};

use time::Date;

use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub struct Bucket {
    pub bucket_id: Uuid,
    pub name: String,
    pub date_created: Date,
    pub size: i64,
    pub space_used: i64,
}

impl Into<fstore::Bucket> for Bucket {
    fn into(self) -> fstore::Bucket {
        fstore::Bucket {
            id: self.bucket_id,
            name: self.name,
            created: self.date_created,
            size: self.size,
            space_used: self.space_used,
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct Object {
    pub object_id: Uuid,
    pub hash: String,
    pub size: i64,
    pub r#type: String,
    pub subtype: String,
    pub date_added: Date,
}

impl Into<fstore::Object> for Object {
    fn into(self) -> fstore::Object {
        fstore::Object {
            id: self.object_id,
            hash: self.hash,
            size: self.size.try_into().unwrap(),
            r#type: self.r#type,
            subtype: self.subtype,
            added: self.date_added,
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct RemoveResult {
    pub objects_removed: i64,
    pub space_freed: i64,
}

impl Into<fstore::RemoveResult> for RemoveResult {
    fn into(self) -> fstore::RemoveResult {
        fstore::RemoveResult {
            objects_removed: self.objects_removed.try_into().unwrap(),
            space_freed: self.space_freed.try_into().unwrap(),
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct StoreTotals {
    pub buckets: i64,
    pub objects: i64,
    pub space_used: i64,
}

impl Into<fstore::StoreTotals> for StoreTotals {
    fn into(self) -> fstore::StoreTotals {
        fstore::StoreTotals {
            buckets: self.buckets.try_into().unwrap(),
            objects: self.objects.try_into().unwrap(),
            space_used: self.space_used.try_into().unwrap(),
        }
    }
}

#[derive(sqlx::FromRow)]
pub struct ObjectError {
    pub object_id: Uuid,
    pub message: String,
}

impl Into<fstore::ObjectError> for ObjectError {
    fn into(self) -> fstore::ObjectError {
        fstore::ObjectError {
            object_id: self.object_id,
            message: self.message,
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
