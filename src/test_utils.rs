use bitcoin::Amount as BitcoinAmount;
use cdk::{
    nuts::nut00::Proof, nuts::nut01::PublicKey, nuts::nut02::Id, secret::Secret,
    Amount as CashuAmount,
};
use chrono::Utc;

use crate::types::{BurnProof, EpochState, MintProof};

pub fn create_sample_proof(keyset_id: Id, amount: CashuAmount) -> Proof {
    let secret = Secret::generate();
    let c = PublicKey::from_slice(&[2; 33]).unwrap();

    Proof::new(amount, keyset_id, secret, c)
}

pub fn create_sample_mint_proof(keyset_id: Id, amount: CashuAmount) -> MintProof {
    let proof = create_sample_proof(keyset_id, amount);
    let amount_u64: u64 = amount.into();
    MintProof {
        proof,
        amount: BitcoinAmount::from_sat(amount_u64),
        timestamp: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_sample_proof() {
        let keyset_id = Id::from_bytes(&[0; 8]).unwrap();
        let amount = CashuAmount::from(1000u64);
        let proof = create_sample_proof(keyset_id, amount.clone());

        assert_eq!(proof.keyset_id, keyset_id);
    }

    #[test]
    fn test_create_sample_mint_proof() {
        let keyset_id = Id::from_bytes(&[0; 8]).unwrap();
        let amount = CashuAmount::from(1000u64);
        let amount_u64: u64 = amount.into();
        let mint_proof = create_sample_mint_proof(keyset_id, amount.clone());
        assert_eq!(mint_proof.amount, BitcoinAmount::from_sat(amount_u64));
    }
}
