mod block_timestamp;
mod pb;

use self::block_timestamp::BlockTimestamp;
use hex;    
use pb::acme::{BlockMeta, BigInt, Transaction, TransactionList};
use substreams::store::{
    self, DeltaProto, StoreNew, StoreSetIfNotExists, StoreSetIfNotExistsProto,
};
use substreams::Hex;
use substreams_ethereum::pb::eth::v2 as eth;
use substreams_ethereum::pb::eth::v2::TransactionTraceStatus;
use substreams_database_change::pb::database::{table_change::Operation, DatabaseChanges};
use substreams_ethereum::pb as ethpb;

// store for blocks data
#[substreams::handlers::store]
fn store_block_meta_start(blk: eth::Block, s: StoreSetIfNotExistsProto<BlockMeta>) {
    let (timestamp, meta) = transform_block_to_block_meta(blk);

    s.set_if_not_exists(meta.number, timestamp.start_of_day_key(), &meta);
    s.set_if_not_exists(meta.number, timestamp.start_of_month_key(), &meta);
}

// store for transaction data
#[substreams::handlers::store]
fn store_transaction_data(blk: eth::Block, s: StoreSetIfNotExistsProto<Transaction>) {
    let transaction_details_list = blk
        .transaction_traces
        .clone().into_iter()
        .filter(|trx| trx.status == TransactionTraceStatus::Succeeded as i32)
        .map(|trx| transform_transaction_to_transaction_meta(trx, &blk))
        .collect();

    let main_list = TransactionList {
        transaction_details_list,
    };
    for transaction in main_list.transaction_details_list {
        s.set_if_not_exists(transaction.blockNumber, &format!("transaction.id"), &transaction);
    }
}


// main db_out function
#[substreams::handlers::map]
fn db_out(
    block_meta_start: store::Deltas<DeltaProto<BlockMeta>>,
    store_transaction_data: store::Deltas<DeltaProto<Transaction>>
) -> Result<DatabaseChanges, substreams::errors::Error> {
    let mut database_changes: DatabaseChanges = Default::default();
    transform_block_meta_to_database_changes(&mut database_changes, block_meta_start);
    transform_transaction_data_to_database_changes(&mut database_changes, store_transaction_data);
    Ok(database_changes)
}

fn transform_block_to_block_meta(blk: ethpb::eth::v2::Block) -> (BlockTimestamp, BlockMeta) {
    let timestamp = BlockTimestamp::from_block(&blk);
    let header = blk.header.unwrap();

    (
        timestamp,
        BlockMeta {
            id: base_64_to_hex(blk.hash.clone()),
            timestamp: Some(header.timestamp.unwrap()),
            parentHash: base_64_to_hex(header.parent_hash),
            uncleHash: base_64_to_hex(header.uncle_hash),
            receiptRoot: header.receipt_root.clone(),
            gasLimit: header.gas_limit,
            hash: base_64_to_hex(blk.hash.clone()),
            gasUsed: header.gas_used,
            number: blk.number,
            nonce: header.nonce,
            size: blk.size,
        },
    )
}

fn transform_transaction_to_transaction_meta(trx:  eth::TransactionTrace, block: &eth::Block) -> Transaction {
    let header = block.header.as_ref().unwrap();
    let block_number = block.number;
    let time_stamp = header.timestamp.clone().unwrap().seconds;
    
        Transaction {
            id:  base_64_to_hex(trx.hash),
        gasUsed: trx.gas_used,
        status: trx.status.to_string(),
        index: trx.index,
        nonce: trx.nonce,
        // maxFeePerGas: option_bigint_to_number_string(trx.max_fee_per_gas),
        // maxPriorityFeePerGas: option_bigint_to_number_string(trx.max_priority_fee_per_gas),
        gasLimit: trx.gas_limit,
        to: base_64_to_hex(trx.to),
        from: base_64_to_hex(trx.from),
        // value: option_bigint_to_number_string(trx.value),
        blockNumber: block_number,
        timestamp: time_stamp,
        }
    
}


// base64 to hex
fn base_64_to_hex<T: std::convert::AsRef<[u8]>>(num:T) -> String {
    let num = hex::encode(&num);
    let num = num.to_string();
     format!("0x{}", &num)
}

// bigint to string
fn option_bigint_to_number_string(bigint: Option<BigInt>) -> String {
    bigint
        .map(|num| {
            let bytes = num.bytes;
            let mut value: u128 = 0;
            for byte in bytes {
                value = (value << 8) + u128::from(byte);
            }
            value.to_string()
        })
        .unwrap_or_else(String::new)
}

fn transform_block_meta_to_database_changes(
    changes: &mut DatabaseChanges,
    deltas: store::Deltas<DeltaProto<BlockMeta>>,
) {
    use substreams::pb::substreams::store_delta::Operation;

    for delta in deltas.deltas {
        match delta.operation {
            Operation::Create => push_create(
                changes,
                &delta.key,
                BlockTimestamp::from_key(&delta.key),
                delta.ordinal,
                delta.new_value,
            ),
            Operation::Update => push_update(
                changes,
                &delta.key,
                delta.ordinal,
                delta.old_value,
                delta.new_value,
            ),
            Operation::Delete => todo!(),
            x => panic!("unsupported opeation {:?}", x),
        }
    }
}

fn transform_transaction_data_to_database_changes(
    changes: &mut DatabaseChanges,
    deltas: store::Deltas<DeltaProto<Transaction>>,
) {
    use substreams::pb::substreams::store_delta::Operation;

    for delta in deltas.deltas {
        match delta.operation {
            Operation::Create => push_create_transaction(
                changes,
                &delta.key,
                delta.ordinal,
                delta.new_value,
            ),
            Operation::Update => push_update_transaction(
                changes,
                &delta.key,
                delta.ordinal,
                delta.old_value,
                delta.new_value,
            ),
            Operation::Delete => todo!(),
            x => panic!("unsupported opeation {:?}", x),
        }
    }
}

// consider moving back into a standalone file
//#[path = "db_out.rs"]
//mod db;
fn push_create(
    changes: &mut DatabaseChanges,
    key: &str,
    timestamp: BlockTimestamp,
    ordinal: u64,
    value: BlockMeta,
) {
    changes
        .push_change("block_meta", key, ordinal, Operation::Create)
        .change("at", (None, timestamp))
        .change("size", (None, value.size))
        .change("number", (None, value.number))
        .change("gas_limit", (None, value.gasLimit))
        .change("gas_used", (None, value.gasUsed))
        .change("id", (None, value.id))
        .change("hash", (None, value.hash))
        .change("uncle_hash", (None, value.uncleHash))
        .change("receipt_root", (None, value.receiptRoot))
        .change("parent_hash", (None, value.parentHash))
        .change("timestamp", (None, value.timestamp.unwrap()));
}

fn push_update(
    changes: &mut DatabaseChanges,
    key: &str,
    ordinal: u64,
    old_value: BlockMeta,
    new_value: BlockMeta,
) {
    changes
        .push_change("block_meta", key, ordinal, Operation::Update)
        .change("number", (old_value.number, new_value.number))
        .change("size", (old_value.size, new_value.size))
        .change("gas_limit", (old_value.number, new_value.gasLimit))
        .change("gas_used", (old_value.number, new_value.gasUsed))
        .change("id", (old_value.id, new_value.id));
        // .change("hash", (old_value.hash, new_value.hash))
        // .change(
        //     "parent_hash",
        //     (old_value.parentHash, new_value.parentHash),
        // )
        // .change(
        //     "uncle_hash",
        //     (old_value.uncleHash, new_value.uncleHash),
        // )
        // .change(
        //     "timestamp",
        //     (&old_value.timestamp.unwrap(), &new_value.timestamp.unwrap()),
        // );
}


fn push_create_transaction(
    changes: &mut DatabaseChanges,
    key: &str,
    ordinal: u64,
    value: Transaction,
) {
    changes
        .push_change("transactions", key, ordinal, Operation::Create)
        .change("status", (None, value.status))
        .change("gas_limit", (None, value.gasLimit))
        .change("gas_used", (None, value.gasUsed))
        .change("id", (None, value.id.clone()))
        .change("hash", (None, value.id.clone()));
        // .change("to", (None, value.to))
        // .change("from", (None, value.from));
}


fn push_update_transaction(
    changes: &mut DatabaseChanges,
    key: &str,
    ordinal: u64,
    old_value: Transaction,
    new_value: Transaction,
) {
    changes
        .push_change("transactions", key, ordinal, Operation::Update)
        .change("at", (old_value.timestamp, new_value.timestamp))
        .change("status", (old_value.status, new_value.status))
        .change("gas_limit", (old_value.gasLimit, new_value.gasLimit))
        .change("gas_used", (old_value.gasUsed, new_value.gasUsed))
        .change("id", (old_value.id.clone(), new_value.id.clone()))
        .change("hash", (old_value.id.clone(), new_value.id.clone()));
}
