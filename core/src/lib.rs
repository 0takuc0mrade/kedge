//! Kedge Core — Shared types for the agent and ZKP guest program.
//!
//! These types are serialized/deserialized across the zkVM boundary:
//! - The host (agent) serializes `ClaimInput` and sends it to the guest
//! - The guest evaluates conditions and commits `ClaimOutput` to the journal
//! - The host reads `ClaimOutput` from the journal as public outputs

#![no_std]

extern crate alloc;

use alloc::{string::String, vec::Vec};
use alloy_sol_types::sol;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Shipment status categories matching the mock freight API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShipmentStatus {
    /// Shipment is moving on schedule
    OnTime,
    /// Shipment has a minor delay (< threshold)
    Delayed,
    /// Shipment has exceeded the parametric delay threshold
    CriticalDelay,
    /// Shipment has been lost or abandoned
    Lost,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogisticsOraclePayload {
    pub tracking_id: String,
    pub policy_id: [u8; 32],
    pub claimant: [u8; 20],
    pub status: ShipmentStatus,
    pub delay_hours: u64,
    pub insured_value: u64,
    pub event_timestamp: u64,
    pub nonce: u64,
    pub issued_at: u64,
    pub expires_at: u64,
    pub chain_id: u64,
}

/// Private witness passed to the zkVM guest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimInput {
    pub payload: LogisticsOraclePayload,
    pub oracle_public_key: [u8; 32],
    pub oracle_signature: Vec<u8>,
}

sol! {
    /// Output from the ZKP guest program (public journal).
    ///
    /// This is committed to the journal and becomes the public
    /// output that is verified on-chain. It proves that the claim
    /// evaluation was performed correctly without revealing the
    /// underlying shipment data.
    struct ClaimOutput {
        bool isTriggered;
        uint64 payoutAmount;
        uint64 payoutPercentage;
        bytes32 trackingIdHash;
        uint64 timestamp;
        address claimant;
        bytes32 policyId;
        bytes32 oracleKeyHash;
        bytes32 payloadHash;
        uint64 expiresAt;
        uint64 chainId;
    }
}

// ─── Evaluation Constants ──────────────────────────────────

/// Minimum delay (in hours) required to trigger any claim.
pub const DELAY_THRESHOLD_HOURS: u64 = 48;

/// Payout tiers: (min_delay_hours, max_delay_hours, payout_percentage).
pub const PAYOUT_TIERS: &[(u64, u64, u64)] = &[
    (48, 72, 25),         // 48-72h delay  → 25% payout
    (72, 120, 50),        // 72-120h delay → 50% payout
    (120, 240, 75),       // 120-240h delay → 75% payout
    (240, u64::MAX, 100), // 240h+ delay  → 100% payout (total loss)
];

/// Evaluate a claim input against parametric conditions.
///
/// This function is used by BOTH the agent (for quick local checks)
/// and the zkVM guest (for provable evaluation). Keeping the logic
/// in one place ensures consistency.
pub fn verify_oracle_signature(input: &ClaimInput) -> bool {
    let Ok(public_key) = VerifyingKey::from_bytes(&input.oracle_public_key) else {
        return false;
    };
    let Ok(signature_bytes) = <[u8; 64]>::try_from(input.oracle_signature.as_slice()) else {
        return false;
    };
    let signature = Signature::from_bytes(&signature_bytes);

    public_key
        .verify_strict(&input.payload.signing_bytes(), &signature)
        .is_ok()
}

pub fn evaluate(input: &ClaimInput) -> ClaimOutput {
    let payload = &input.payload;
    let qualifies = matches!(
        payload.status,
        ShipmentStatus::CriticalDelay | ShipmentStatus::Lost
    ) && payload.delay_hours >= DELAY_THRESHOLD_HOURS;

    let tracking_id_hash = sha256(payload.tracking_id.as_bytes());
    let oracle_key_hash = sha256(&input.oracle_public_key);
    let payload_hash = sha256(&payload.signing_bytes());

    if !qualifies {
        return ClaimOutput {
            isTriggered: false,
            payoutAmount: 0,
            payoutPercentage: 0,
            trackingIdHash: tracking_id_hash.into(),
            timestamp: payload.event_timestamp,
            claimant: payload.claimant.into(),
            policyId: payload.policy_id.into(),
            oracleKeyHash: oracle_key_hash.into(),
            payloadHash: payload_hash.into(),
            expiresAt: payload.expires_at,
            chainId: payload.chain_id,
        };
    }

    let payout_pct = PAYOUT_TIERS
        .iter()
        .find(|(min, max, _)| payload.delay_hours >= *min && payload.delay_hours < *max)
        .map(|(_, _, pct)| *pct)
        .unwrap_or(100);

    let payout_amount = (payload.insured_value * payout_pct) / 100;

    ClaimOutput {
        isTriggered: true,
        payoutAmount: payout_amount,
        payoutPercentage: payout_pct,
        trackingIdHash: tracking_id_hash.into(),
        timestamp: payload.event_timestamp,
        claimant: payload.claimant.into(),
        policyId: payload.policy_id.into(),
        oracleKeyHash: oracle_key_hash.into(),
        payloadHash: payload_hash.into(),
        expiresAt: payload.expires_at,
        chainId: payload.chain_id,
    }
}

impl LogisticsOraclePayload {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let tracking_id = self.tracking_id.as_bytes();
        let mut encoded = Vec::with_capacity(143 + tracking_id.len());

        encoded.extend_from_slice(b"KEDGE_ORACLE_V1");
        encoded.extend_from_slice(&(tracking_id.len() as u32).to_be_bytes());
        encoded.extend_from_slice(tracking_id);
        encoded.extend_from_slice(&self.policy_id);
        encoded.extend_from_slice(&self.claimant);
        encoded.push(status_code(&self.status));
        encoded.extend_from_slice(&self.delay_hours.to_be_bytes());
        encoded.extend_from_slice(&self.insured_value.to_be_bytes());
        encoded.extend_from_slice(&self.event_timestamp.to_be_bytes());
        encoded.extend_from_slice(&self.nonce.to_be_bytes());
        encoded.extend_from_slice(&self.issued_at.to_be_bytes());
        encoded.extend_from_slice(&self.expires_at.to_be_bytes());
        encoded.extend_from_slice(&self.chain_id.to_be_bytes());

        encoded
    }
}

fn status_code(status: &ShipmentStatus) -> u8 {
    match status {
        ShipmentStatus::OnTime => 0,
        ShipmentStatus::Delayed => 1,
        ShipmentStatus::CriticalDelay => 2,
        ShipmentStatus::Lost => 3,
    }
}

fn sha256(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use ed25519_dalek::{Signer, SigningKey};

    fn make_input(status: ShipmentStatus, delay_hours: u64, insured_value: u64) -> ClaimInput {
        ClaimInput {
            payload: LogisticsOraclePayload {
                tracking_id: "TEST-001".to_string(),
                policy_id: [0xCD; 32],
                claimant: [0xAB; 20],
                status,
                delay_hours,
                insured_value,
                event_timestamp: 1749225600,
                nonce: 1,
                issued_at: 1749225600,
                expires_at: 1749225900,
                chain_id: 5003,
            },
            oracle_public_key: [0; 32],
            oracle_signature: alloc::vec![0; 64],
        }
    }

    #[test]
    fn test_no_trigger_on_time() {
        let result = evaluate(&make_input(ShipmentStatus::OnTime, 0, 50000));
        assert!(!result.isTriggered);
        assert_eq!(result.payoutAmount, 0);
    }

    #[test]
    fn test_tier_1() {
        let result = evaluate(&make_input(ShipmentStatus::CriticalDelay, 48, 100000));
        assert!(result.isTriggered);
        assert_eq!(result.payoutPercentage, 25);
        assert_eq!(result.payoutAmount, 25000);
    }

    #[test]
    fn test_tier_2() {
        let result = evaluate(&make_input(ShipmentStatus::CriticalDelay, 96, 100000));
        assert!(result.isTriggered);
        assert_eq!(result.payoutPercentage, 50);
        assert_eq!(result.payoutAmount, 50000);
    }

    #[test]
    fn test_tier_3() {
        let result = evaluate(&make_input(ShipmentStatus::CriticalDelay, 150, 100000));
        assert!(result.isTriggered);
        assert_eq!(result.payoutPercentage, 75);
        assert_eq!(result.payoutAmount, 75000);
    }

    #[test]
    fn test_tier_4_total_loss() {
        let result = evaluate(&make_input(ShipmentStatus::Lost, 300, 100000));
        assert!(result.isTriggered);
        assert_eq!(result.payoutPercentage, 100);
        assert_eq!(result.payoutAmount, 100000);
    }

    #[test]
    fn test_tracking_id_hash_deterministic() {
        let r1 = evaluate(&make_input(ShipmentStatus::CriticalDelay, 48, 100000));
        let r2 = evaluate(&make_input(ShipmentStatus::CriticalDelay, 48, 100000));
        assert_eq!(r1.trackingIdHash, r2.trackingIdHash);
    }

    #[test]
    fn test_oracle_signature_verification() {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let mut input = make_input(ShipmentStatus::CriticalDelay, 48, 100000);
        input.oracle_public_key = signing_key.verifying_key().to_bytes();
        input.oracle_signature = signing_key
            .sign(&input.payload.signing_bytes())
            .to_bytes()
            .to_vec();

        assert!(verify_oracle_signature(&input));

        input.payload.delay_hours = 96;
        assert!(!verify_oracle_signature(&input));
    }
}
