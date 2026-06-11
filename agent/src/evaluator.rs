//! Claim evaluation engine for the Kedge agent.
//!
//! This module bridges the agent's ingestor data with the shared
//! core evaluation logic. It converts `ShipmentData` into `ClaimInput`,
//! runs the evaluation, and wraps the result for the agent's workflow.

use crate::ingestor::ShipmentData;
use kedge_core::{self, ClaimInput, ClaimOutput, ShipmentStatus as CoreStatus};

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
pub fn evaluate_claim(shipment: &ShipmentData) -> ClaimEvaluation {
    let claim_input = ClaimInput {
        tracking_id: shipment.tracking_id.clone(),
        status: to_core_status(&shipment.status),
        delay_hours: shipment.delay_hours,
        insured_value: shipment.insured_value,
        timestamp: shipment.timestamp,
        claimant: [0x42; 20], // Hardcoded dummy claimant
    };

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

    ClaimEvaluation {
        is_triggered: output.isTriggered,
        tracking_id: shipment.tracking_id.clone(),
        observed_delay_hours: shipment.delay_hours,
        payout_amount: output.payoutAmount,
        payout_percentage: output.payoutPercentage,
        reason,
        claim_input,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestor::{ShipmentData, ShipmentStatus};

    fn make_shipment(status: ShipmentStatus, delay_hours: u64, insured_value: u64) -> ShipmentData {
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
        }
    }

    #[test]
    fn test_no_trigger_on_time() {
        let shipment = make_shipment(ShipmentStatus::OnTime, 0, 50000);
        let eval = evaluate_claim(&shipment);
        assert!(!eval.is_triggered);
        assert_eq!(eval.payout_amount, 0);
    }

    #[test]
    fn test_no_trigger_minor_delay() {
        let shipment = make_shipment(ShipmentStatus::Delayed, 24, 50000);
        let eval = evaluate_claim(&shipment);
        assert!(!eval.is_triggered);
    }

    #[test]
    fn test_no_trigger_below_threshold() {
        let shipment = make_shipment(ShipmentStatus::CriticalDelay, 40, 50000);
        let eval = evaluate_claim(&shipment);
        assert!(!eval.is_triggered);
    }

    #[test]
    fn test_tier_1_payout() {
        let shipment = make_shipment(ShipmentStatus::CriticalDelay, 48, 100000);
        let eval = evaluate_claim(&shipment);
        assert!(eval.is_triggered);
        assert_eq!(eval.payout_percentage, 25);
        assert_eq!(eval.payout_amount, 25000);
    }

    #[test]
    fn test_tier_2_payout() {
        let shipment = make_shipment(ShipmentStatus::CriticalDelay, 96, 100000);
        let eval = evaluate_claim(&shipment);
        assert!(eval.is_triggered);
        assert_eq!(eval.payout_percentage, 50);
        assert_eq!(eval.payout_amount, 50000);
    }

    #[test]
    fn test_tier_3_payout() {
        let shipment = make_shipment(ShipmentStatus::CriticalDelay, 150, 100000);
        let eval = evaluate_claim(&shipment);
        assert!(eval.is_triggered);
        assert_eq!(eval.payout_percentage, 75);
        assert_eq!(eval.payout_amount, 75000);
    }

    #[test]
    fn test_tier_4_total_loss() {
        let shipment = make_shipment(ShipmentStatus::Lost, 300, 100000);
        let eval = evaluate_claim(&shipment);
        assert!(eval.is_triggered);
        assert_eq!(eval.payout_percentage, 100);
        assert_eq!(eval.payout_amount, 100000);
    }

    #[test]
    fn test_exact_boundary_48h() {
        let shipment = make_shipment(ShipmentStatus::CriticalDelay, 48, 80000);
        let eval = evaluate_claim(&shipment);
        assert!(eval.is_triggered);
        assert_eq!(eval.payout_percentage, 25);
        assert_eq!(eval.payout_amount, 20000);
    }

    #[test]
    fn test_exact_boundary_72h() {
        let shipment = make_shipment(ShipmentStatus::CriticalDelay, 72, 80000);
        let eval = evaluate_claim(&shipment);
        assert!(eval.is_triggered);
        assert_eq!(eval.payout_percentage, 50);
        assert_eq!(eval.payout_amount, 40000);
    }

    #[test]
    fn test_claim_input_preserved() {
        let shipment = make_shipment(ShipmentStatus::CriticalDelay, 96, 100000);
        let eval = evaluate_claim(&shipment);
        assert_eq!(eval.claim_input.tracking_id, "TEST-001");
        assert_eq!(eval.claim_input.delay_hours, 96);
    }
}
