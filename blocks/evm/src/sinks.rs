use substreams::errors::Error;
use substreams::pb::substreams::Clock;
use substreams_database_change::pb::database::DatabaseChanges;
use substreams_ethereum::pb::eth::v2::Block;

use crate::blocks::insert_blocks;
use crate::transactions::insert_transactions;

#[substreams::handlers::map]
pub fn ch_out(clock: Clock, block: Block) -> Result<DatabaseChanges, Error> {
    let mut tables: DatabaseChanges = DatabaseChanges::default();

    // TABLE::blocks
    insert_blocks(&mut tables, &clock, &block, false);

    // TABLE::transactions
    insert_transactions(&mut tables, &clock, &block);

    Ok(tables)
}

#[substreams::handlers::map]
pub fn map_blocks(clock: Clock, block: Block) -> Result<DatabaseChanges, Error> {
    let mut tables: DatabaseChanges = DatabaseChanges::default();

    // TABLE::blocks
    insert_blocks(&mut tables, &clock, &block, true);

    Ok(tables)
}

// // TO-DO: Implement the `graph_out` function using EntityChanges
// #[substreams::handlers::map]
// pub fn graph_out(clock: Clock, block: Block) -> Result<DatabaseChanges, Error> {
//     let mut tables: DatabaseChanges = DatabaseChanges::default();
//     insert_blocks(&mut tables, &clock, &block);
//     // TO-DO: Convert DatabaseChanges to EntityChanges
//     Ok(tables)
// }
