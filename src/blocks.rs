use substreams::pb::substreams::Clock;
use substreams_antelope::pb::Block;
use substreams_entity_change::tables::Tables;

use crate::utils::{block_date_to_month, block_date_to_year, block_time_to_date};

use super::transactions::insert_transaction;

// https://github.com/pinax-network/firehose-antelope/blob/534ca5bf2aeda67e8ef07a1af8fc8e0fe46473ee/proto/sf/antelope/type/v1/type.proto#L21
pub fn insert_blocks(params: &String, tables: &mut Tables, clock: &Clock, block: &Block) {
    // header
    let header = block.header.clone().unwrap_or_default();
    let previous = &header.previous;
    let producer = &header.producer;

    // timestamp
    let timestamp = clock.clone().timestamp.unwrap();
    // TO-DO: Convert as Subgraph Timestamp type
    // https://github.com/pinax-network/antelope-transactions-subgraph/issues/2
    // let nanos = timestamp.nanos;
    // let microseconds = seconds * 1_000_000 + i64::from(nanos) / 1_000;
    let date = block_time_to_date(timestamp.to_string().as_str());
    let month = block_date_to_month(&date);
    let year = block_date_to_year(&date);
    let seconds = timestamp.seconds;
    let block_time = seconds;
    let block_number = clock.number.to_string();
    let block_hash = &clock.id;

    // TABLE::Transaction
    let mut is_match = false;
    for transaction in block.transaction_traces() {
        if insert_transaction(&params, tables, clock, &transaction) { is_match = true;}
    }

    // TABLE::Block
    if is_match {
        tables.create_row("Block", &block_hash)
            .set("previous", previous)
            .set("producer", producer)
            .set("date", date)
            .set("month", month)
            .set("year", year)
            .set_bigint("time", &block_time.to_string())
            .set_bigint("number", &block_number.to_string())
        ;
    }

}