CREATE TABLE bucket (
    -- The bucket's unique internal identifier.
    bucket_id       uuid DEFAULT gen_random_uuid() PRIMARY KEY,

    -- The bucket's user-assigned name.
    name            text UNIQUE NOT NULL,

    -- The time this bucket was first created.
    date_created    timestamptz NOT NULL DEFAULT NOW()
);

CREATE TABLE object (
    -- The object's unique identifier.
    object_id       uuid PRIMARY KEY,

    -- SHA256 checksum of the object's contents.
    hash            text UNIQUE NOT NULL,

    -- Size of the object's contents in bytes.
    size            bigint NOT NULL,

    -- MIME type of the object's contents.
    "type"          text NOT NULL,
    subtype         text NOT NULL,

    -- The time this object was first added to the object store.
    date_added      timestamptz NOT NULL DEFAULT NOW()
);

CREATE TABLE bucket_object (
    bucket_id       uuid REFERENCES bucket ON DELETE CASCADE,
    object_id       uuid REFERENCES object ON DELETE CASCADE,
    date_added      timestamptz NOT NULL DEFAULT NOW(),

    PRIMARY KEY (bucket_id, object_id)
);

CREATE TABLE object_error (
    object_id       uuid PRIMARY KEY REFERENCES object ON DELETE CASCADE,
    message         text NOT NULL
);
