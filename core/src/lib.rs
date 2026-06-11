//! Kedge Core — Shared types for the agent and ZKP guest program.
//!
//! These types are serialized/deserialized across the zkVM boundary:
//! - The host (agent) serializes `ClaimInput` and sends it to the guest
//! - The guest evaluates conditions and commits `ClaimOutput` to the journal
//! - The host reads `ClaimOutput` from the journal as public outputs

#![no_std]

extern crate alloc;

use alloc::string::String;
use alloy_sol_types::sol;
use serde::{Deserialize, Serialize};

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

/// Input to the ZKP guest program (private witness).
///
/// This is the shipment telemetry data that the agent feeds
/// into the zkVM. It remains private — only the `ClaimOutput`
/// (journal) is revealed publicly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimInput {
    /// Unique tracking identifier
    pub tracking_id: String,
    /// Current shipment status
    pub status: ShipmentStatus,
    /// Total delay in hours from the estimated delivery date
    pub delay_hours: u64,
    /// The insured value in MockUSDT
    pub insured_value: u64,
    /// Unix timestamp of the data snapshot
    pub timestamp: u64,
    /// Address of the claimant who will receive the funds
    pub claimant: [u8; 20],
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
pub fn evaluate(input: &ClaimInput) -> ClaimOutput {
    let qualifies = matches!(
        input.status,
        ShipmentStatus::CriticalDelay | ShipmentStatus::Lost
    ) && input.delay_hours >= DELAY_THRESHOLD_HOURS;

    if !qualifies {
        return ClaimOutput {
            isTriggered: false,
            payoutAmount: 0,
            payoutPercentage: 0,
            trackingIdHash: simple_hash(input.tracking_id.as_bytes()).into(),
            timestamp: input.timestamp,
            claimant: input.claimant.into(),
        };
    }

    // Determine payout tier
    let payout_pct = PAYOUT_TIERS
        .iter()
        .find(|(min, max, _)| input.delay_hours >= *min && input.delay_hours < *max)
        .map(|(_, _, pct)| *pct)
        .unwrap_or(100);

    let payout_amount = (input.insured_value * payout_pct) / 100;

    ClaimOutput {
        isTriggered: true,
        payoutAmount: payout_amount,
        payoutPercentage: payout_pct,
        trackingIdHash: simple_hash(input.tracking_id.as_bytes()).into(),
        timestamp: input.timestamp,
        claimant: input.claimant.into(),
    }
}

/// Simple deterministic hash for the tracking ID.
///
/// Uses a basic FNV-1a-style hash expanded to 32 bytes.
/// This avoids pulling in a heavy crypto dependency in the guest.
fn simple_hash(data: &[u8]) -> [u8; 32] {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    let mut result = [0u8; 32];
    let bytes = hash.to_le_bytes();
    // Repeat the 8-byte hash across 32 bytes for a fixed-size output
    result[0..8].copy_from_slice(&bytes);
    result[8..16].copy_from_slice(&bytes);
    result[16..24].copy_from_slice(&bytes);
    result[24..32].copy_from_slice(&bytes);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    fn make_input(status: ShipmentStatus, delay_hours: u64, insured_value: u64) -> ClaimInput {
        ClaimInput {
            tracking_id: "TEST-001".to_string(),
            status,
            delay_hours,
            insured_value,
            timestamp: 1749225600,
            claimant: [0xAB; 20],
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
}
