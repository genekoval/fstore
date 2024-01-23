ALTER TABLE bucket
ALTER COLUMN bucket_id
SET DEFAULT gen_random_uuid();
