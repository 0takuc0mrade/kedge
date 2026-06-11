//! On-chain transaction submitter for the Kedge agent.
//!
//! Handles signing and broadcasting claim settlement transactions
//! to the Mantle network via the agent's hot wallet using Alloy.

use alloy::{
    network::EthereumWallet, primitives::Address, providers::ProviderBuilder,
    signers::local::PrivateKeySigner, sol,
};
use std::str::FromStr;
use url::Url;

// Compile-time safe interface declaration matching our Day 4 Solidity contract
sol! {
    #[sol(rpc)]
    interface IClaimRegistry {
        function submitClaim(bytes calldata seal, bytes calldata journal) external;
    }
}

pub struct ChainSubmitter {
    rpc_url: Url,
    wallet: EthereumWallet,
    registry_address: Address,
}

impl ChainSubmitter {
    /// Initializer creating an authenticated JSON-RPC pipeline with automatic fillers
    pub fn new(
        rpc_url: &str,
        private_key: &str,
        registry_address: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let signer = PrivateKeySigner::from_str(private_key)?;
        let wallet = EthereumWallet::from(signer);

        let url = Url::parse(rpc_url)?;
        let registry_addr = Address::from_str(registry_address)?;

        Ok(Self {
            rpc_url: url,
            wallet,
            registry_address: registry_addr,
        })
    }

    /// Autonomous transaction broadcaster for packaging the ZKP and submitting to Mantle/Anvil
    pub async fn submit_claim(
        &self,
        seal: Vec<u8>,
        journal: Vec<u8>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let provider = ProviderBuilder::new()
            .wallet(self.wallet.clone())
            .connect_http(self.rpc_url.clone());

        let contract = IClaimRegistry::new(self.registry_address, provider);

        tracing::info!("🔗 Constructing on-chain transaction payload...");
        let tx_call = contract.submitClaim(seal.into(), journal.into());

        tracing::info!("🚀 Broadcasting transaction to ClaimRegistry via hot wallet...");
        let pending_tx = tx_call.send().await?;

        tracing::info!("⏳ Awaiting block confirmation receipt...");
        let receipt = pending_tx.get_receipt().await?;

        if !receipt.status() {
            return Err("Transaction execution reverted on-chain".into());
        }

        Ok(format!("{:?}", receipt.transaction_hash))
    }
}
