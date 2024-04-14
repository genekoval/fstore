mod model;

pub use model::*;

use sqlx_helper_macros::{database, transaction};
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

    clone_bucket(original: Uuid, name: &str) -> Bucket;

    create_bucket(name: &str) -> Bucket;

    fetch_bucket(name: &str) -> Bucket;

    fetch_buckets_all() -> Vec<Bucket>;

    fetch_store_totals() -> StoreTotals;

    get_bucket_objects(bucket_id: Uuid) -> Vec<Object>;

    get_errors() -> Vec<ObjectError>;

    get_objects(bucket_id: Uuid, objects: &[Uuid]) -> Vec<Object>;

    get_object_count(before: Timestamp) -> i64;

    stream_objects(before: Timestamp) -> Stream<Object>;

    remove_bucket(bucket_id: &Uuid);

    remove_object(bucket_id: &Uuid, object_id: &Uuid) -> Option<Object>;

    remove_objects(bucket_id: &Uuid, objects: &[Uuid]) -> RemoveResult;

    rename_bucket(bucket_id: &Uuid, name: &str);

    update_object_errors(records: &[ObjectError]);
}

transaction! {
    remove_orphan_objects() -> Vec<Object>;
}
