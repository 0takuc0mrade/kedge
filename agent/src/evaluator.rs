//! Claim evaluation engine for the Kedge agent.
//!
//! Implements the parametric insurance logic: given a shipment's
//! current telemetry, determines whether the delay threshold has
//! been breached and calculates the appropriate payout tier.

use crate::ingestor::{ShipmentData, ShipmentStatus};

/// Minimum delay (in hours) required to trigger any claim.
const DELAY_THRESHOLD_HOURS: u64 = 48;

/// Payout tiers as a percentage of insured value.
/// Each tier is (min_delay_hours, max_delay_hours, payout_percentage).
const PAYOUT_TIERS: &[(u64, u64, u64)] = &[
    (48, 72, 25),   // 48-72h delay → 25% payout
    (72, 120, 50),  // 72-120h delay → 50% payout
    (120, 240, 75), // 120-240h delay → 75% payout
    (240, u64::MAX, 100), // 240h+ delay → 100% payout (total loss)
];

/// Result of a claim evaluation cycle.
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
}

/// Evaluate a shipment's data against the parametric insurance conditions.
///
/// The logic is intentionally straightforward for the hackathon demo:
/// - If `delay_hours >= DELAY_THRESHOLD_HOURS`, the claim is triggered
/// - Payout amount scales with delay severity across defined tiers
/// - The evaluation result contains all data needed for ZKP generation
pub fn evaluate_claim(shipment: &ShipmentData) -> ClaimEvaluation {
    // Check if shipment status indicates a qualifying event
    let qualifies = matches!(
        shipment.status,
        ShipmentStatus::CriticalDelay | ShipmentStatus::Lost
    ) && shipment.delay_hours >= DELAY_THRESHOLD_HOURS;

    if !qualifies {
        return ClaimEvaluation {
            is_triggered: false,
            tracking_id: shipment.tracking_id.clone(),
            observed_delay_hours: shipment.delay_hours,
            payout_amount: 0,
            payout_percentage: 0,
            reason: format!(
                "Delay of {}h does not meet {}h threshold or status {:?} is non-qualifying",
                shipment.delay_hours, DELAY_THRESHOLD_HOURS, shipment.status
            ),
        };
    }

    // Determine payout tier
    let (payout_pct, tier_label) = PAYOUT_TIERS
        .iter()
        .find(|(min, max, _)| shipment.delay_hours >= *min && shipment.delay_hours < *max)
        .map(|(min, max, pct)| {
            let label = if *max == u64::MAX {
                format!("{}h+ (total loss)", min)
            } else {
                format!("{}h-{}h", min, max)
            };
            (*pct, label)
        })
        .unwrap_or((100, "overflow".to_string()));

    let payout_amount = (shipment.insured_value * payout_pct) / 100;

    ClaimEvaluation {
        is_triggered: true,
        tracking_id: shipment.tracking_id.clone(),
        observed_delay_hours: shipment.delay_hours,
        payout_amount,
        payout_percentage: payout_pct,
        reason: format!(
            "Delay of {}h in tier {} → {}% of {} MockUSDT = {} MockUSDT",
            shipment.delay_hours, tier_label, payout_pct, shipment.insured_value, payout_amount
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestor::ShipmentData;

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
        // Critical delay status but hours below threshold
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
}
