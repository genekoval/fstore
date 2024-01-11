mod model;

pub use model::*;

use sql_helper_macros::{database, transaction};
use uuid::Uuid;

database! {
    add_object(
        bucket_id: &Uuid,
        object_id: &Uuid,
        hash: &str,
        size: i64,
        ty: &str,
        subtype: &str,
    ) -> Object;

    create_bucket(name: &str) -> Bucket;

    fetch_bucket(name: &str) -> Bucket;

    fetch_buckets() -> Vec<Bucket>;

    fetch_store_totals() -> StoreTotals;

    get_errors() -> Vec<ObjectError>;

    get_object(bucket_id: &Uuid, object_id: &Uuid) -> Option<Object>;

    remove_bucket(bucket_id: &Uuid);

    remove_object(bucket_id: &Uuid, object_id: &Uuid) -> Option<Object>;

    remove_objects(bucket_id: &Uuid, objects: &[Uuid]) -> RemoveResult;

    rename_bucket(bucket_id: &Uuid, name: &str);

    update_object_errors<'a>(records: ObjectErrorSlice<'a>);
}

transaction! {
    remove_orphan_objects() -> Vec<Object>;
}
