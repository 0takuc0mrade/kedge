//! Claim evaluation engine for the Kedge agent.
//!
//! This module bridges the agent's ingestor data with the shared
//! core evaluation logic. It converts `ShipmentData` into `ClaimInput`,
//! runs the evaluation, and wraps the result for the agent's workflow.

use crate::ingestor::ShipmentData;
use eyre::{Result, WrapErr};
use kedge_core::{
    self, verify_oracle_signature, ClaimInput, ClaimOutput, LogisticsOraclePayload,
    ShipmentStatus as CoreStatus,
};

/// Result of a claim evaluation cycle (agent-side wrapper).
#[derive(Debug, Clone)]
pub struct ClaimEvaluation {
    /// Whether the parametric conditions have been met
    pub is_triggered: bool,

    /// The shipment tracking ID this evaluation pertains to
    pub tracking_id: String,

    /// The delay in hours that was observed
    pub observed_delay_hours: u64,

    /// Payout amount in MockUSDT (0 if not triggered)
    pub payout_amount: u64,

    /// Payout percentage applied (0-100)
    pub payout_percentage: u64,

    /// Human-readable reason for the evaluation outcome
    pub reason: String,

    /// Canonical signed payload commitment used as the on-chain claim ID
    pub claim_id: [u8; 32],

    /// The core input (needed for ZKP generation)
    pub claim_input: ClaimInput,
}

/// Convert ingestor ShipmentStatus to core ShipmentStatus.
fn to_core_status(status: &crate::ingestor::ShipmentStatus) -> CoreStatus {
    match status {
        crate::ingestor::ShipmentStatus::OnTime => CoreStatus::OnTime,
        crate::ingestor::ShipmentStatus::Delayed => CoreStatus::Delayed,
        crate::ingestor::ShipmentStatus::CriticalDelay => CoreStatus::CriticalDelay,
        crate::ingestor::ShipmentStatus::Lost => CoreStatus::Lost,
    }
}

/// Evaluate a shipment's data against the parametric insurance conditions.
///
/// Delegates to `kedge_core::evaluate()` — the single source of truth
/// for claim logic that is also used inside the zkVM guest program.
pub fn evaluate_claim(shipment: &ShipmentData) -> Result<ClaimEvaluation> {
    let claim_input = ClaimInput {
        payload: LogisticsOraclePayload {
            tracking_id: shipment.tracking_id.clone(),
            policy_id: decode_fixed_hex(&shipment.policy_id).wrap_err("Invalid policy_id")?,
            claimant: decode_fixed_hex(&shipment.claimant).wrap_err("Invalid claimant")?,
            status: to_core_status(&shipment.status),
            delay_hours: shipment.delay_hours,
            insured_value: shipment.insured_value,
            event_timestamp: shipment.timestamp,
            nonce: shipment.nonce,
            issued_at: shipment.issued_at,
            expires_at: shipment.expires_at,
            chain_id: shipment.chain_id,
        },
        oracle_public_key: decode_fixed_hex(&shipment.oracle_public_key)
            .wrap_err("Invalid oracle_public_key")?,
        oracle_signature: decode_hex(&shipment.oracle_signature)
            .wrap_err("Invalid oracle_signature")?,
    };

    if !verify_oracle_signature(&claim_input) {
        eyre::bail!("Shipment payload has an invalid logistics oracle signature");
    }

    let output: ClaimOutput = kedge_core::evaluate(&claim_input);

    let reason = if output.isTriggered {
        format!(
            "Delay of {}h → {}% of {} MockUSDT = {} MockUSDT",
            shipment.delay_hours,
            output.payoutPercentage,
            shipment.insured_value,
            output.payoutAmount
        )
    } else {
        format!(
            "Delay of {}h does not meet threshold or status {:?} is non-qualifying",
            shipment.delay_hours, shipment.status
        )
    };

    Ok(ClaimEvaluation {
        is_triggered: output.isTriggered,
        tracking_id: shipment.tracking_id.clone(),
        observed_delay_hours: shipment.delay_hours,
        payout_amount: output.payoutAmount,
        payout_percentage: output.payoutPercentage,
        reason,
        claim_id: output.payloadHash.0,
        claim_input,
    })
}

fn decode_hex(value: &str) -> Result<Vec<u8>> {
    hex::decode(value.strip_prefix("0x").unwrap_or(value)).wrap_err("Expected hex-encoded value")
}

fn decode_fixed_hex<const N: usize>(value: &str) -> Result<[u8; N]> {
    let decoded = decode_hex(value)?;
    decoded
        .try_into()
        .map_err(|_| eyre::eyre!("Expected {N} bytes"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestor::{ShipmentData, ShipmentStatus};
    use ed25519_dalek::{Signer, SigningKey};

    fn make_shipment(status: ShipmentStatus, delay_hours: u64, insured_value: u64) -> ShipmentData {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let policy_id = [0xCD; 32];
        let claimant = [0xAB; 20];
        let payload = LogisticsOraclePayload {
            tracking_id: "TEST-001".to_string(),
            policy_id,
            claimant,
            status: to_core_status(&status),
            delay_hours,
            insured_value,
            event_timestamp: 1749225600,
            nonce: 1,
            issued_at: 1749225600,
            expires_at: 1749225900,
            chain_id: 5003,
        };
        let signature = signing_key.sign(&payload.signing_bytes());

        ShipmentData {
            tracking_id: "TEST-001".to_string(),
            carrier: "TestCarrier".to_string(),
            origin: "Shanghai".to_string(),
            destination: "Rotterdam".to_string(),
            status,
            delay_hours,
            estimated_delivery: "2026-06-01T12:00:00Z".to_string(),
            actual_delivery: None,
            insured_value,
            timestamp: 1749225600,
            policy_id: format!("0x{}", hex::encode(policy_id)),
            claimant: format!("0x{}", hex::encode(claimant)),
            nonce: 1,
            issued_at: 1749225600,
            expires_at: 1749225900,
            chain_id: 5003,
            oracle_public_key: format!("0x{}", hex::encode(signing_key.verifying_key().to_bytes())),
            oracle_signature: format!("0x{}", hex::encode(signature.to_bytes())),
        }
    }

    #[test]
    fn test_no_trigger_on_time() {
        let shipment = make_shipment(ShipmentStatus::OnTime, 0, 50000);
        let eval = evaluate_claim(&shipment).unwrap();
        assert!(!eval.is_triggered);
        assert_eq!(eval.payout_amount, 0);
    }

    #[test]
    fn test_no_trigger_minor_delay() {
        let shipment = make_shipment(ShipmentStatus::Delayed, 24, 50000);
        let eval = evaluate_claim(&shipment).unwrap();
        assert!(!eval.is_triggered);
    }

    #[test]
    fn test_no_trigger_below_threshold() {
        let shipment = make_shipment(ShipmentStatus::CriticalDelay, 40, 50000);
        let eval = evaluate_claim(&shipment).unwrap();
        assert!(!eval.is_triggered);
    }

    #[test]
    fn test_tier_1_payout() {
        let shipment = make_shipment(ShipmentStatus::CriticalDelay, 48, 100000);
        let eval = evaluate_claim(&shipment).unwrap();
        assert!(eval.is_triggered);
        assert_eq!(eval.payout_percentage, 25);
        assert_eq!(eval.payout_amount, 25000);
    }

    #[test]
    fn test_tier_2_payout() {
        let shipment = make_shipment(ShipmentStatus::CriticalDelay, 96, 100000);
        let eval = evaluate_claim(&shipment).unwrap();
        assert!(eval.is_triggered);
        assert_eq!(eval.payout_percentage, 50);
        assert_eq!(eval.payout_amount, 50000);
    }

    #[test]
    fn test_tier_3_payout() {
        let shipment = make_shipment(ShipmentStatus::CriticalDelay, 150, 100000);
        let eval = evaluate_claim(&shipment).unwrap();
        assert!(eval.is_triggered);
        assert_eq!(eval.payout_percentage, 75);
        assert_eq!(eval.payout_amount, 75000);
    }

    #[test]
    fn test_tier_4_total_loss() {
        let shipment = make_shipment(ShipmentStatus::Lost, 300, 100000);
        let eval = evaluate_claim(&shipment).unwrap();
        assert!(eval.is_triggered);
        assert_eq!(eval.payout_percentage, 100);
        assert_eq!(eval.payout_amount, 100000);
    }

    #[test]
    fn test_exact_boundary_48h() {
        let shipment = make_shipment(ShipmentStatus::CriticalDelay, 48, 80000);
        let eval = evaluate_claim(&shipment).unwrap();
        assert!(eval.is_triggered);
        assert_eq!(eval.payout_percentage, 25);
        assert_eq!(eval.payout_amount, 20000);
    }

    #[test]
    fn test_exact_boundary_72h() {
        let shipment = make_shipment(ShipmentStatus::CriticalDelay, 72, 80000);
        let eval = evaluate_claim(&shipment).unwrap();
        assert!(eval.is_triggered);
        assert_eq!(eval.payout_percentage, 50);
        assert_eq!(eval.payout_amount, 40000);
    }

    #[test]
    fn test_claim_input_preserved() {
        let shipment = make_shipment(ShipmentStatus::CriticalDelay, 96, 100000);
        let eval = evaluate_claim(&shipment).unwrap();
        assert_eq!(eval.claim_input.payload.tracking_id, "TEST-001");
        assert_eq!(eval.claim_input.payload.delay_hours, 96);
    }

    #[test]
    fn test_rejects_tampered_oracle_payload() {
        let mut shipment = make_shipment(ShipmentStatus::CriticalDelay, 96, 100000);
        shipment.delay_hours = 120;

        assert!(evaluate_claim(&shipment).is_err());
    }

    #[test]
    fn test_accepts_python_oracle_fixture() {
        let json = r#"{
            "tracking_id": "KDG-CROSS-LANG-0001",
            "carrier": "CMA CGM",
            "origin": "Shanghai, CN",
            "destination": "Rotterdam, NL",
            "status": "critical_delay",
            "delay_hours": 96,
            "estimated_delivery": "2026-06-08T17:37:52Z",
            "actual_delivery": null,
            "insured_value": 50000,
            "timestamp": 1781285872,
            "policy_id": "0xa6ae293134a68767c1bfd9cf811d84e10d2322327dff3b5a390b8998608f71b0",
            "claimant": "0x4242424242424242424242424242424242424242",
            "nonce": 1,
            "issued_at": 1781285872,
            "expires_at": 1781286172,
            "chain_id": 5003,
            "oracle_public_key": "0xea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c",
            "oracle_signature": "0x8d5bb70240082f8a75eee3e6f7ce86d7b7d66fe839d50abd96a00607f41544def86fcd7a81dcc5d0645082eb3a66f8379a6cfc3e7aba21693daf60e8e8ebd800"
        }"#;
        let shipment: ShipmentData = serde_json::from_str(json).unwrap();
        let evaluation = evaluate_claim(&shipment).unwrap();

        assert!(evaluation.is_triggered);
        assert_eq!(evaluation.payout_amount, 25000);
    }
}
