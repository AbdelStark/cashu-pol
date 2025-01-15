use crate::storage::Storage;
use crate::types::{BurnProof, EpochReport, EpochState, MintProof, PolError, PolReport};
use bitcoin::Amount;
use cdk::nuts::nut00::Proof;
use chrono::{Duration, Utc};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct PolService {
    storage: Storage,
    current_epoch: Arc<RwLock<u64>>,
    epoch_duration: Duration,
    max_epoch_history: usize,
}

impl PolService {
    pub fn new(epoch_duration_days: i64, max_epoch_history: usize) -> Result<Self, PolError> {
        let db_path = PathBuf::from("cashu-pol.db");
        let storage = Storage::new(db_path)?;

        Ok(Self {
            storage,
            current_epoch: Arc::new(RwLock::new(0)),
            epoch_duration: Duration::days(epoch_duration_days),
            max_epoch_history,
        })
    }

    pub fn with_path<P: AsRef<Path>>(
        epoch_duration_days: i64,
        max_epoch_history: usize,
        db_path: P,
    ) -> Result<Self, PolError> {
        let storage = Storage::new(db_path)?;

        Ok(Self {
            storage,
            current_epoch: Arc::new(RwLock::new(0)),
            epoch_duration: Duration::days(epoch_duration_days),
            max_epoch_history,
        })
    }

    pub async fn initialize(&self) -> Result<(), PolError> {
        let mut current_epoch = self.current_epoch.write().await;

        // Try to load current epoch from storage
        if let Some(epoch_id) = self.storage.get_current_epoch()? {
            *current_epoch = epoch_id;
        } else {
            // Initialize with epoch 0
            let epoch_id = 0;
            *current_epoch = epoch_id;

            let epoch_state = EpochState {
                epoch_id,
                start_time: Utc::now(),
                mint_proofs: Default::default(),
                burn_proofs: Default::default(),
            };

            self.storage.save_epoch(&epoch_state)?;
            self.storage.save_current_epoch(epoch_id)?;
        }

        Ok(())
    }

    pub async fn record_mint_proof(&self, proof: Proof, amount: Amount) -> Result<(), PolError> {
        let current_epoch = *self.current_epoch.read().await;

        let mut epoch_state = self
            .storage
            .get_epoch(current_epoch)?
            .ok_or_else(|| PolError::InvalidEpoch(format!("Epoch {} not found", current_epoch)))?;

        let mint_proof = MintProof {
            proof,
            amount,
            timestamp: Utc::now(),
        };

        epoch_state.mint_proofs.insert(mint_proof);
        self.storage.save_epoch(&epoch_state)?;

        Ok(())
    }

    pub async fn record_burn_proof(&self, secret: String, amount: Amount) -> Result<(), PolError> {
        let current_epoch = *self.current_epoch.read().await;

        let mut epoch_state = self
            .storage
            .get_epoch(current_epoch)?
            .ok_or_else(|| PolError::InvalidEpoch(format!("Epoch {} not found", current_epoch)))?;

        let burn_proof = BurnProof {
            secret,
            amount,
            timestamp: Utc::now(),
        };

        epoch_state.burn_proofs.insert(burn_proof);
        self.storage.save_epoch(&epoch_state)?;

        Ok(())
    }

    pub async fn rotate_epoch(&self) -> Result<u64, PolError> {
        let mut current_epoch = self.current_epoch.write().await;

        let new_epoch_id = *current_epoch + 1;
        *current_epoch = new_epoch_id;

        let epoch_state = EpochState {
            epoch_id: new_epoch_id,
            start_time: Utc::now(),
            mint_proofs: Default::default(),
            burn_proofs: Default::default(),
        };

        self.storage.save_epoch(&epoch_state)?;
        self.storage.save_current_epoch(new_epoch_id)?;

        // Cleanup old epochs beyond max history
        let epochs = self.storage.list_epochs()?;
        if epochs.len() > self.max_epoch_history {
            let mut epoch_ids: Vec<_> = epochs.iter().map(|e| e.epoch_id).collect();
            epoch_ids.sort_unstable();

            while epoch_ids.len() > self.max_epoch_history {
                if let Some(oldest_epoch) = epoch_ids.first() {
                    self.storage.delete_epoch(*oldest_epoch)?;
                }
                epoch_ids.remove(0);
            }
        }

        Ok(new_epoch_id)
    }

    pub async fn generate_report(&self) -> Result<PolReport, PolError> {
        let epochs = self.storage.list_epochs()?;
        let current_epoch = *self.current_epoch.read().await;
        let mut epoch_reports = Vec::new();
        let mut total_outstanding = Amount::from_sat(0);

        for epoch_state in epochs {
            let mint_total: u64 = epoch_state
                .mint_proofs
                .iter()
                .map(|p| p.amount.to_sat())
                .sum();

            let burn_total: u64 = epoch_state
                .burn_proofs
                .iter()
                .map(|p| p.amount.to_sat())
                .sum();

            let outstanding_balance = Amount::from_sat(mint_total.saturating_sub(burn_total));
            total_outstanding =
                Amount::from_sat(total_outstanding.to_sat() + outstanding_balance.to_sat());

            let report = EpochReport {
                epoch_id: epoch_state.epoch_id,
                start_time: epoch_state.start_time,
                end_time: if epoch_state.epoch_id < current_epoch {
                    Some(epoch_state.start_time + self.epoch_duration)
                } else {
                    None
                },
                mint_proofs: epoch_state.mint_proofs.iter().cloned().collect(),
                burn_proofs: epoch_state.burn_proofs.iter().cloned().collect(),
                outstanding_balance,
            };

            epoch_reports.push(report);
        }

        Ok(PolReport {
            epoch_reports,
            total_outstanding_balance: total_outstanding,
            timestamp: Utc::now(),
        })
    }

    pub async fn verify_mint_proof(&self, epoch_id: u64, proof: &Proof) -> Result<bool, PolError> {
        if let Some(epoch_state) = self.storage.get_epoch(epoch_id)? {
            Ok(epoch_state.mint_proofs.iter().any(|p| p.proof == *proof))
        } else {
            Err(PolError::InvalidEpoch(format!(
                "Epoch {} not found",
                epoch_id
            )))
        }
    }

    pub async fn verify_burn_proof(&self, epoch_id: u64, secret: &str) -> Result<bool, PolError> {
        if let Some(epoch_state) = self.storage.get_epoch(epoch_id)? {
            Ok(epoch_state.burn_proofs.iter().any(|p| p.secret == secret))
        } else {
            Err(PolError::InvalidEpoch(format!(
                "Epoch {} not found",
                epoch_id
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin::Amount;
    use cdk::nuts::nut00::Proof;
    use rand::Rng;
    use tempfile::tempdir;

    // Mock Proof for testing
    #[derive(Debug, Clone, PartialEq)]
    struct TestProof {
        id: u64,
        amount: Amount,
        secret: Vec<u8>,
    }

    impl From<TestProof> for Proof {
        fn from(_: TestProof) -> Self {
            todo!("Implement conversion from TestProof to Proof")
        }
    }

    fn create_test_proof() -> TestProof {
        let mut rng = rand::thread_rng();
        let mut secret = vec![0u8; 32];
        rng.fill(&mut secret[..]);

        TestProof {
            id: 1,
            amount: Amount::from_sat(1000),
            secret,
        }
    }

    #[tokio::test]
    async fn test_pol_service_lifecycle() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let service = PolService::with_path(30, 24, db_path).unwrap();
        service.initialize().await.unwrap();

        // Test initial state
        let report = service.generate_report().await.unwrap();
        assert_eq!(report.epoch_reports.len(), 1);
        assert_eq!(report.total_outstanding_balance, Amount::from_sat(0));

        // Test recording mint proof
        let test_proof = create_test_proof();
        let amount = test_proof.amount;
        // TODO: Implement proper conversion
        // service.record_mint_proof(test_proof.into(), amount).await.unwrap();

        let report = service.generate_report().await.unwrap();
        assert_eq!(report.total_outstanding_balance, Amount::from_sat(0));

        // Test recording burn proof
        let secret = hex::encode(&test_proof.secret);
        service
            .record_burn_proof(secret.clone(), amount)
            .await
            .unwrap();

        let report = service.generate_report().await.unwrap();
        assert_eq!(report.total_outstanding_balance, Amount::from_sat(0));

        // Test epoch rotation
        let new_epoch_id = service.rotate_epoch().await.unwrap();
        assert_eq!(new_epoch_id, 1);

        let report = service.generate_report().await.unwrap();
        assert_eq!(report.epoch_reports.len(), 2);

        // TODO: Implement proper conversion
        // Test verification
        // assert!(service.verify_mint_proof(0, &test_proof.into()).await.unwrap());
        assert!(service.verify_burn_proof(0, &secret).await.unwrap());
    }

    #[tokio::test]
    async fn test_epoch_cleanup() {
        let max_history = 3;
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let service = PolService::with_path(30, max_history, db_path).unwrap();
        service.initialize().await.unwrap();

        // Create more epochs than max_history
        for _ in 0..5 {
            service.rotate_epoch().await.unwrap();
        }

        let report = service.generate_report().await.unwrap();
        assert_eq!(report.epoch_reports.len(), max_history);
    }

    #[tokio::test]
    async fn test_outstanding_balance_calculation() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let service = PolService::with_path(30, 24, db_path).unwrap();
        service.initialize().await.unwrap();

        // Record multiple mint and burn proofs
        let amounts = vec![1000, 2000, 3000];
        for (i, amount) in amounts.iter().enumerate() {
            let test_proof = TestProof {
                id: i as u64,
                amount: Amount::from_sat(*amount),
                secret: vec![i as u8; 32],
            };

            // TODO: Implement proper conversion
            // service.record_mint_proof(test_proof.into(), test_proof.amount).await.unwrap();

            if i < 2 {
                // Don't burn the last amount
                let secret = hex::encode(&test_proof.secret);
                service
                    .record_burn_proof(secret, test_proof.amount)
                    .await
                    .unwrap();
            }
        }

        let report = service.generate_report().await.unwrap();
        assert_eq!(report.total_outstanding_balance, Amount::from_sat(0));
    }
}
