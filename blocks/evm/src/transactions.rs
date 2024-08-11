use common::blocks::insert_timestamp;
use common::keys::transaction_keys;
use common::utils::bytes_to_hex;
use common::utils::optional_bigint_to_string;
use substreams::pb::substreams::Clock;
use substreams_database_change::pb::database::TableChange;
use substreams_database_change::pb::database::{table_change, DatabaseChanges};
use substreams_ethereum::pb::eth::v2::Block;
use substreams_ethereum::pb::eth::v2::TransactionTrace;

use crate::logs::insert_log;
use crate::traces::insert_trace;

pub fn transaction_type_to_string(r#type: i32) -> String {
    match r#type {
        0 => "Legacy".to_string(),
        1 => "AccessList".to_string(),
        2 => "DynamicFee".to_string(),
        3 => "Blob".to_string(),
        100 => "ArbitrumDeposit".to_string(),
        101 => "ArbitrumUnsigned".to_string(),
        102 => "ArbitrumContract".to_string(),
        104 => "ArbitrumRetry".to_string(),
        105 => "ArbitrumSubmitRetryable".to_string(),
        106 => "ArbitrumInternal".to_string(),
        120 => "ArbitrumLegacy".to_string(),
        126 => "OptimismDeposit".to_string(),
        _ => "Unknown".to_string(),
    }
}

pub fn transaction_status_to_string(status: i32) -> String {
    match status {
        0 => "Unknown".to_string(),
        1 => "Succeeded".to_string(),
        2 => "Failed".to_string(),
        3 => "Reverted".to_string(),
        _ => "Unknown".to_string(),
    }
}

pub fn is_transaction_success(status: i32) -> bool {
    status == 1
}

pub fn insert_transactions(tables: &mut DatabaseChanges, clock: &Clock, block: &Block) {
    for transaction in block.transaction_traces.iter() {
        insert_transaction(tables, clock, &transaction, &block);
    }
}

// https://github.com/streamingfast/firehose-ethereum/blob/1bcb32a8eb3e43347972b6b5c9b1fcc4a08c751e/proto/sf/ethereum/type/v2/type.proto#L658
// DetailLevel: BASE & EXTENDED
pub fn insert_transaction(tables: &mut DatabaseChanges, clock: &Clock, transaction: &TransactionTrace, block: &Block) {
    let index = transaction.index;
    let hash = bytes_to_hex(transaction.hash.clone());
    let from = bytes_to_hex(transaction.from.clone()); // EVM Address
    let to = bytes_to_hex(transaction.to.clone()); // EVM Address
    let nonce = transaction.nonce;
    let gas_price = optional_bigint_to_string(transaction.gas_price.clone(), "0"); // UInt256
    let gas_limit = transaction.gas_limit;
    let value = optional_bigint_to_string(transaction.value.clone(), "0"); // UInt256
    let input = bytes_to_hex(transaction.input.clone());
    let v = bytes_to_hex(transaction.v.clone());
    let r = bytes_to_hex(transaction.r.clone());
    let s = bytes_to_hex(transaction.s.clone());
    let gas_used = transaction.gas_used;
    let r#type = transaction_type_to_string(transaction.r#type);
    let type_code = transaction.r#type;
    let max_fee_per_gas = optional_bigint_to_string(transaction.max_fee_per_gas.clone(), "0"); // UInt256
    let max_priority_fee_per_gas = optional_bigint_to_string(transaction.max_priority_fee_per_gas.clone(), "0"); // UInt256
    let begin_ordinal = transaction.begin_ordinal;
    let end_ordinal = transaction.end_ordinal;
    let success = is_transaction_success(transaction.status);
    let status = transaction_status_to_string(transaction.status);
    let status_code = transaction.status;

    // transaction receipt
    let receipt = transaction.receipt.clone().unwrap();
    let blob_gas_price = optional_bigint_to_string(receipt.clone().blob_gas_price, "0");
    let blob_gas_used = receipt.blob_gas_used();
    let cumulative_gas_used = receipt.cumulative_gas_used;
    let logs_bloom = bytes_to_hex(receipt.logs_bloom);
    let state_root = bytes_to_hex(receipt.state_root);

    // block roots
    let header = block.header.clone().unwrap();
    let transactions_root = bytes_to_hex(header.transactions_root.clone());
    let receipts_root = bytes_to_hex(header.receipt_root.clone());

    let keys = transaction_keys(&clock, &hash);
    let row = tables
        .push_change_composite("transactions", keys, 0, table_change::Operation::Create)
        .change("index", ("", index.to_string().as_str()))
        .change("hash", ("", hash.as_str()))
        .change("from", ("", from.as_str()))
        .change("to", ("", to.as_str()))
        .change("nonce", ("", nonce.to_string().as_str()))
        .change("gas_price", ("", gas_price.to_string().as_str()))
        .change("gas_limit", ("", gas_limit.to_string().as_str()))
        .change("value", ("", value.as_str()))
        .change("input", ("", input.as_str()))
        .change("v", ("", v.as_str()))
        .change("r", ("", r.as_str()))
        .change("s", ("", s.as_str()))
        .change("gas_used", ("", gas_used.to_string().as_str()))
        .change("r", ("", r.as_str()))
        .change("type", ("", r#type.as_str()))
        .change("type_code", ("", type_code.to_string().as_str()))
        .change("max_fee_per_gas", ("", max_fee_per_gas.as_str()))
        .change("max_priority_fee_per_gas", ("", max_priority_fee_per_gas.as_str()))
        .change("begin_ordinal", ("", begin_ordinal.to_string().as_str()))
        .change("end_ordinal", ("", end_ordinal.to_string().as_str()))
        .change("success", ("", success.to_string().as_str()))
        .change("status", ("", status.as_str()))
        .change("status_code", ("", status_code.to_string().as_str()))

        // transaction receipt
        .change("blob_gas_price", ("", blob_gas_price.as_str()))
        .change("blob_gas_used", ("", blob_gas_used.to_string().as_str()))
        .change("cumulative_gas_used", ("", cumulative_gas_used.to_string().as_str()))
        .change("logs_bloom", ("", logs_bloom.as_str()))
        .change("state_root", ("", state_root.as_str()))

        // block roots
        .change("transactions_root", ("", transactions_root.as_str()))
        .change("receipts_root", ("", receipts_root.as_str()))
        ;

    insert_timestamp(row, clock, false);

    // TABLE::traces
    for call in transaction.calls() {
        insert_trace(tables, clock, call.call, call.transaction);
    }

    // TABLE::logs
    // Only required DetailLevel=BASE since traces are not available in BASE
    let detail_level = block.detail_level;
    if detail_level == 2 {
        for log in receipt.logs {
            insert_log(tables, clock, &log, transaction);
        }
    }
}

pub fn insert_transaction_metadata(row: &mut TableChange, transaction: &TransactionTrace, is_transaction: bool) {
    let tx_index = transaction.index;
    let tx_hash = bytes_to_hex(transaction.hash.clone());
    let from = bytes_to_hex(transaction.from.clone()); // does trace contain `from`?
    let to = bytes_to_hex(transaction.to.clone()); // does trace contain `to`?
    let tx_status = transaction_status_to_string(transaction.status);
    let tx_status_code = transaction.status;
    let tx_success = is_transaction_success(transaction.status);
    let prefix = if is_transaction { "" } else { "tx_" };

    row.change("tx_index", ("", tx_index.to_string().as_str()))
        .change("tx_hash", ("", tx_hash.as_str()))
        .change(format!("{}from", prefix).as_str(), ("", from.as_str()))
        .change(format!("{}to", prefix).as_str(), ("", to.as_str()))
        .change("tx_status", ("", tx_status.as_str()))
        .change("tx_status_code", ("", tx_status_code.to_string().as_str()))
        .change("tx_success", ("", tx_success.to_string().as_str()));
}