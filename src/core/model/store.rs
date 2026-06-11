use std::path::Path;

use redb::{Database, ReadableDatabase, TableDefinition};
use serde::{Serialize, de::DeserializeOwned};

const PLAYLISTS_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("playlists");
const TRACK_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("track_meta");

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error: {0}")]
    Database(#[from] redb::DatabaseError),
    #[error("transaction error: {0}")]
    Transaction(#[from] redb::TransactionError),
    #[error("table error: {0}")]
    Table(#[from] redb::TableError),
    #[error("storage error: {0}")]
    Storage(#[from] redb::StorageError),
    #[error("commit error: {0}")]
    Commit(#[from] redb::CommitError),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("not found: {0}")]
    NotFound(String),
}

/// Thin wrapper around redb. Callers work with typed values; this
/// module owns the encoding (JSON) and table layout.
pub struct Store {
    db: Database,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let db = Database::create(path)?;

        let txn = db.begin_write()?;
        {
            txn.open_table(PLAYLISTS_TABLE)?;
            txn.open_table(TRACK_META_TABLE)?;
        }
        txn.commit()?;

        Ok(Self { db })
    }

    pub fn get_playlist<T: DeserializeOwned>(&self, id: &str) -> Result<T, StoreError> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(PLAYLISTS_TABLE)?;
        let bytes = table
            .get(id)?
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;
        Ok(serde_json::from_slice(bytes.value())?)
    }

    pub fn save_playlist<T: Serialize>(&self, id: &str, value: &T) -> Result<(), StoreError> {
        let bytes = serde_json::to_vec(value)?;
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(PLAYLISTS_TABLE)?;
            table.insert(id, bytes.as_slice())?;
        }
        txn.commit()?;
        Ok(())
    }

    pub fn delete_playlist(&self, id: &str) -> Result<(), StoreError> {
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(PLAYLISTS_TABLE)?;
            table.remove(id)?;
        }
        txn.commit()?;
        Ok(())
    }

    pub fn get_track_meta<T: DeserializeOwned>(&self, id: &str) -> Result<T, StoreError> {
        let txn = self.db.begin_read()?;
        let table = txn.open_table(TRACK_META_TABLE)?;
        let bytes = table
            .get(id)?
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;
        Ok(serde_json::from_slice(bytes.value())?)
    }

    pub fn save_track_meta<T: Serialize>(&self, id: &str, value: &T) -> Result<(), StoreError> {
        let bytes = serde_json::to_vec(value)?;
        let txn = self.db.begin_write()?;
        {
            let mut table = txn.open_table(TRACK_META_TABLE)?;
            table.insert(id, bytes.as_slice())?;
        }
        txn.commit()?;
        Ok(())
    }
}
