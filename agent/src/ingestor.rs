//! Data ingestion module for the Kedge agent.
//!
//! Fetches real-time shipment status data from the mock freight API,
//! returning structured data for downstream claim evaluation.

use eyre::{Result, WrapErr};
use serde::{Deserialize, Serialize};

/// Possible states of a tracked shipment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// Shipment telemetry data returned by the mock freight API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipmentData {
    /// Unique tracking identifier
    pub tracking_id: String,

    /// Name of the shipping carrier
    pub carrier: String,

    /// Port/location of origin
    pub origin: String,

    /// Port/location of destination
    pub destination: String,

    /// Current shipment status
    pub status: ShipmentStatus,

    /// Total delay in hours from the estimated delivery date
    pub delay_hours: u64,

    /// ISO 8601 timestamp of estimated delivery
    pub estimated_delivery: String,

    /// ISO 8601 timestamp of actual/current projected delivery
    pub actual_delivery: Option<String>,

    /// The insured value in MockUSDT (for claim calculation)
    pub insured_value: u64,

    /// Unix timestamp of the data snapshot
    pub timestamp: u64,
}

/// Fetch the latest shipment status from the mock freight API.
///
/// Calls `GET {base_url}/api/v1/shipment/latest` and deserializes
/// the response into a `ShipmentData` struct.
pub async fn fetch_shipment_status(base_url: &str) -> Result<ShipmentData> {
    let url = format!("{}/api/v1/shipment/latest", base_url);

    let response = reqwest::get(&url)
        .await
        .wrap_err_with(|| format!("Failed to reach mock API at {}", url))?;

    let status_code = response.status();
    if !status_code.is_success() {
        eyre::bail!(
            "Mock API returned HTTP {}: {}",
            status_code,
            response.text().await.unwrap_or_default()
        );
    }

    let shipment: ShipmentData = response
        .json()
        .await
        .wrap_err("Failed to deserialize shipment data")?;

    Ok(shipment)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shipment_status_deserialization() {
        let json = r#"{
            "tracking_id": "KDG-2026-0001",
            "carrier": "Maersk Global",
            "origin": "Shanghai, CN",
            "destination": "Rotterdam, NL",
            "status": "critical_delay",
            "delay_hours": 72,
            "estimated_delivery": "2026-06-01T12:00:00Z",
            "actual_delivery": null,
            "insured_value": 50000,
            "timestamp": 1749225600
        }"#;

        let shipment: ShipmentData = serde_json::from_str(json).unwrap();
        assert_eq!(shipment.tracking_id, "KDG-2026-0001");
        assert_eq!(shipment.status, ShipmentStatus::CriticalDelay);
        assert_eq!(shipment.delay_hours, 72);
        assert_eq!(shipment.insured_value, 50000);
    }
}
