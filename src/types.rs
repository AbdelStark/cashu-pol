use bitcoin::Amount;
use cdk::nuts::nut00::Proof;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
pub struct MockProof {
    pub id: String,
    pub amount: Amount,
    pub secret: Vec<u8>,
}

#[cfg(test)]
impl From<MockProof> for Proof {
    fn from(mock: MockProof) -> Self {
        Proof {
            keyset_id: mock.id,
            c: hex::encode(&mock.secret),
            witness: hex::encode(&mock.secret),
            dleq: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct MintProof {
    pub proof: Proof,
    pub amount: Amount,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub struct BurnProof {
    pub secret: String,
    pub amount: Amount,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochReport {
    pub epoch_id: u64,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub mint_proofs: Vec<MintProof>,
    pub burn_proofs: Vec<BurnProof>,
    pub outstanding_balance: Amount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolReport {
    pub epoch_reports: Vec<EpochReport>,
    pub total_outstanding_balance: Amount,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpochState {
    pub epoch_id: u64,
    pub start_time: DateTime<Utc>,
    pub mint_proofs: HashSet<MintProof>,
    pub burn_proofs: HashSet<BurnProof>,
}

#[derive(Debug, thiserror::Error)]
pub enum PolError {
    #[error("Invalid epoch: {0}")]
    InvalidEpoch(String),

    #[error("Proof verification failed: {0}")]
    ProofVerificationFailed(String),

    #[error("Report generation failed: {0}")]
    ReportGenerationFailed(String),

    #[error("Database error: {0}")]
    DatabaseError(String),
}
