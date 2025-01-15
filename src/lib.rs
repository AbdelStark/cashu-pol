mod service;
mod storage;
mod test_utils;
mod types;

pub use service::PolService;
pub use storage::Storage;
pub use test_utils::*;
pub use types::{BurnProof, EpochReport, MintProof, PolError, PolReport};

#[cfg(test)]
mod tests {
    use super::*;
    use cdk::{nuts::nut02::Id, Amount as CashuAmount};
    use chrono::Utc;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_service_with_sample_data() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Create service with 7-day epochs and 4 epochs history
        let service = PolService::with_path(7, 4, db_path).unwrap();
        service.initialize().await.unwrap();

        // Create sample data across multiple epochs
        let amounts = [
            (vec![5000, 3000], vec![2000]), // Epoch 0: +8000, -2000 = +6000
            (vec![2000], vec![1000, 500]),  // Epoch 1: +2000, -1500 = +500
            (vec![1000], vec![4000]),       // Epoch 2: +1000, -4000 = -3000
        ];

        let keyset_id = Id::from_bytes(&[0; 8]).unwrap();

        // Record proofs for each epoch
        for (epoch, (mint_amounts, burn_amounts)) in amounts.iter().enumerate() {
            for &amount in mint_amounts {
                let mint_proof = create_sample_mint_proof(keyset_id, CashuAmount::from(amount));
                service
                    .record_mint_proof(mint_proof.proof.clone(), mint_proof.amount)
                    .await
                    .unwrap();
            }

            for &amount in burn_amounts {
                let secret = format!("burn_{}", amount);
                service
                    .record_burn_proof(secret, bitcoin::Amount::from_sat(amount))
                    .await
                    .unwrap();
            }

            if epoch < amounts.len() - 1 {
                service.rotate_epoch().await.unwrap();
            }
        }

        // Generate and verify report
        let report = service.generate_report().await.unwrap();

        assert_eq!(report.epoch_reports.len(), 3);

        // Verify balances for each epoch
        let expected_balances = [6000, 500, -3000];
        for (i, balance) in expected_balances.iter().enumerate() {
            assert_eq!(
                report.epoch_reports[i].outstanding_balance.to_sat() as i64,
                *balance,
                "Balance mismatch in epoch {}",
                i
            );
        }

        // Verify total outstanding balance
        assert_eq!(report.total_outstanding_balance.to_sat(), 3500);
    }

    #[tokio::test]
    async fn test_epoch_rotation_with_sample_data() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Create service with 7-day epochs and 2 epochs history
        let service = PolService::with_path(7, 2, db_path).unwrap();
        service.initialize().await.unwrap();

        let keyset_id = Id::from_bytes(&[0; 8]).unwrap();

        // Add data to first epoch
        let mint_proof = create_sample_mint_proof(keyset_id, CashuAmount::from(5000u64));
        service
            .record_mint_proof(mint_proof.proof.clone(), mint_proof.amount)
            .await
            .unwrap();
        service
            .record_burn_proof("test_burn".to_string(), bitcoin::Amount::from_sat(2000))
            .await
            .unwrap();

        // Rotate epoch
        service.rotate_epoch().await.unwrap();

        // Add data to second epoch
        let mint_proof = create_sample_mint_proof(keyset_id, CashuAmount::from(3000u64));
        service
            .record_mint_proof(mint_proof.proof.clone(), mint_proof.amount)
            .await
            .unwrap();

        // Rotate epoch again (should remove first epoch due to max_history=2)
        service.rotate_epoch().await.unwrap();

        // Verify report only contains last 2 epochs
        let report = service.generate_report().await.unwrap();
        assert_eq!(report.epoch_reports.len(), 2);

        // First epoch in report should have 3000 sat outstanding
        assert_eq!(report.epoch_reports[0].outstanding_balance.to_sat(), 3000);
    }
}
