create table block_meta
(
    id          text not null constraint block_meta_pk primary key,
    at          text,
    number      integer,
    hash        text,
    parent_hash text,
    uncle_hash  text,
    receipt_root text,
    gas_limit text,
    gas_used text,
    timestamp   text,
    size        int
);

create table cursors
(
    id         text not null constraint cursor_pk primary key,
    cursor     text,
    block_num  bigint,
    block_id   text
);

create table transactions
(
    id         text not null constraint transactions_pk primary key,
    status     text,
    gas_used  bigint,
    gas_limit   text,
    hash       text
);
