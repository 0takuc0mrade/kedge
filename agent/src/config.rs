//! Configuration module for the Kedge agent.
//!
//! Loads all runtime parameters from environment variables,
//! supporting both `.env` files and direct environment injection.

use eyre::{Result, WrapErr};

/// Runtime configuration for the Kedge autonomous agent.
#[derive(Debug, Clone)]
pub struct Config {
    /// Agent display name
    pub agent_name: String,

    /// Mantle Sepolia RPC endpoint
    pub rpc_url: String,

    /// Chain ID (5003 for Mantle Sepolia)
    pub chain_id: u64,

    /// Mock freight API base URL
    pub mock_api_url: String,

    /// Seconds between each polling cycle
    pub polling_interval_secs: u64,

    /// Agent hot wallet private key (hex, no 0x prefix)
    pub agent_private_key: String,

    /// ClaimRegistry contract address
    pub claim_registry_address: String,

    /// SettlementVault contract address
    pub settlement_vault_address: String,

    /// Whether RISC Zero is running in dev mode (no real proofs)
    pub risc0_dev_mode: bool,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Automatically reads `.env` file if present.
    pub fn from_env() -> Result<Self> {
        // Load .env file if it exists (non-fatal if missing)
        let _ = dotenvy::dotenv();

        Ok(Self {
            agent_name: std::env::var("AGENT_NAME")
                .unwrap_or_else(|_| "Kedge".to_string()),

            rpc_url: std::env::var("MANTLE_SEPOLIA_RPC")
                .unwrap_or_else(|_| "https://rpc.sepolia.mantle.xyz".to_string()),

            chain_id: std::env::var("CHAIN_ID")
                .unwrap_or_else(|_| "5003".to_string())
                .parse()
                .wrap_err("Invalid CHAIN_ID")?,

            mock_api_url: std::env::var("MOCK_API_URL")
                .unwrap_or_else(|_| "http://localhost:8089".to_string()),

            polling_interval_secs: std::env::var("POLLING_INTERVAL_SECS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .wrap_err("Invalid POLLING_INTERVAL_SECS")?,

            agent_private_key: std::env::var("AGENT_HOT_WALLET_PRIVATE_KEY")
                .unwrap_or_default(),

            claim_registry_address: std::env::var("CLAIM_REGISTRY_ADDRESS")
                .unwrap_or_default(),

            settlement_vault_address: std::env::var("SETTLEMENT_VAULT_ADDRESS")
                .unwrap_or_default(),

            risc0_dev_mode: std::env::var("RISC0_DEV_MODE")
                .unwrap_or_else(|_| "1".to_string())
                == "1",
        })
    }
}
