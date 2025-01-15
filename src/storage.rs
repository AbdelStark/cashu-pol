use crate::types::{BurnProof, EpochState, MintProof, PolError};
use bincode::{deserialize, serialize};
use redb::{Database, ReadableTable, TableDefinition};
use std::path::Path;

const EPOCHS_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("epochs");
const CURRENT_EPOCH_TABLE: TableDefinition<&str, u64> = TableDefinition::new("current_epoch");

pub struct Storage {
    db: Database,
}

impl Storage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, PolError> {
        let db = Database::create(path).map_err(|e| PolError::DatabaseError(e.to_string()))?;

        // Create tables if they don't exist
        let write_txn = db
            .begin_write()
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        write_txn
            .open_table(EPOCHS_TABLE)
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;
        write_txn
            .open_table(CURRENT_EPOCH_TABLE)
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        write_txn
            .commit()
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        Ok(Self { db })
    }

    pub fn save_epoch(&self, epoch_state: &EpochState) -> Result<(), PolError> {
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        let mut table = write_txn
            .open_table(EPOCHS_TABLE)
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        let data = serialize(epoch_state).map_err(|e| PolError::DatabaseError(e.to_string()))?;
        table
            .insert(epoch_state.epoch_id, data.as_slice())
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        write_txn
            .commit()
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub fn get_epoch(&self, epoch_id: u64) -> Result<Option<EpochState>, PolError> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        let table = read_txn
            .open_table(EPOCHS_TABLE)
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        if let Some(data) = table
            .get(epoch_id)
            .map_err(|e| PolError::DatabaseError(e.to_string()))?
        {
            let epoch_state =
                deserialize(data.value()).map_err(|e| PolError::DatabaseError(e.to_string()))?;
            Ok(Some(epoch_state))
        } else {
            Ok(None)
        }
    }

    pub fn list_epochs(&self) -> Result<Vec<EpochState>, PolError> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        let table = read_txn
            .open_table(EPOCHS_TABLE)
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        let mut epochs = Vec::new();
        for result in table
            .iter()
            .map_err(|e| PolError::DatabaseError(e.to_string()))?
        {
            let (_, data) = result.map_err(|e| PolError::DatabaseError(e.to_string()))?;
            let epoch_state =
                deserialize(data.value()).map_err(|e| PolError::DatabaseError(e.to_string()))?;
            epochs.push(epoch_state);
        }

        Ok(epochs)
    }

    pub fn delete_epoch(&self, epoch_id: u64) -> Result<(), PolError> {
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        let mut table = write_txn
            .open_table(EPOCHS_TABLE)
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        table
            .remove(epoch_id)
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        write_txn
            .commit()
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub fn save_current_epoch(&self, epoch_id: u64) -> Result<(), PolError> {
        let write_txn = self
            .db
            .begin_write()
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        let mut table = write_txn
            .open_table(CURRENT_EPOCH_TABLE)
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        table
            .insert("current", epoch_id)
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        write_txn
            .commit()
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    pub fn get_current_epoch(&self) -> Result<Option<u64>, PolError> {
        let read_txn = self
            .db
            .begin_read()
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        let table = read_txn
            .open_table(CURRENT_EPOCH_TABLE)
            .map_err(|e| PolError::DatabaseError(e.to_string()))?;

        Ok(table
            .get("current")
            .map_err(|e| PolError::DatabaseError(e.to_string()))?
            .map(|v| v.value()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BurnProof, MintProof};
    use bitcoin::Amount;
    use cdk::nuts::nut00::Proof;
    use chrono::Utc;
    use std::collections::HashSet;
    use tempfile::tempdir;

    #[test]
    fn test_storage_operations() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let storage = Storage::new(&db_path).unwrap();

        // Create test epoch state
        let epoch_state = EpochState {
            epoch_id: 1,
            start_time: Utc::now(),
            mint_proofs: HashSet::new(),
            burn_proofs: HashSet::new(),
        };

        // Test saving and retrieving epoch
        storage.save_epoch(&epoch_state).unwrap();
        let retrieved = storage.get_epoch(1).unwrap().unwrap();
        assert_eq!(retrieved.epoch_id, epoch_state.epoch_id);

        // Test listing epochs
        let epochs = storage.list_epochs().unwrap();
        assert_eq!(epochs.len(), 1);
        assert_eq!(epochs[0].epoch_id, epoch_state.epoch_id);

        // Test current epoch
        storage.save_current_epoch(1).unwrap();
        assert_eq!(storage.get_current_epoch().unwrap(), Some(1));

        // Test deleting epoch
        storage.delete_epoch(1).unwrap();
        assert!(storage.get_epoch(1).unwrap().is_none());
    }
}
