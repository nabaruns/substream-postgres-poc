CREATE SCHEMA new_schema;

CREATE TABLE my_schema.blocks (
    id          text not null constraint block_meta_pk primary key,
    number      integer,
    parent_hash text,
    receipt_root text,
    gas_limit text,
    gas_used text,
    timestamp   text,
    size        int
);
