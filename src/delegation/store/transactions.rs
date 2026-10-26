use super::open;
use anyhow::Result;
use rusqlite::Connection;

pub fn transaction<T>(operation: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
    let db = open()?;
    let tx = super::super::contention::retry("begin transaction", || {
        Ok(rusqlite::Transaction::new_unchecked(
            &db,
            rusqlite::TransactionBehavior::Immediate,
        )?)
    })?;
    let result = operation(&tx)?;
    // Retry COMMIT on the same transaction without replaying its callback.
    super::super::contention::retry("commit transaction", || Ok(tx.execute_batch("COMMIT")?))?;
    drop(tx);
    if let Err(error) = super::super::delivery::flush(&db) {
        eprintln!("Delegation journal delivery remains pending: {error}");
    }
    Ok(result)
}

pub fn read<T>(operation: impl FnOnce(&Connection) -> Result<T>) -> Result<T> {
    let db = open()?;
    let tx = rusqlite::Transaction::new_unchecked(&db, rusqlite::TransactionBehavior::Deferred)?;
    let value = operation(&tx)?;
    tx.commit()?;
    Ok(value)
}
