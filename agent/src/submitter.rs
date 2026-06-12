//! On-chain transaction submitter for the Kedge agent.
//!
//! Handles signing and broadcasting claim settlement transactions
//! to the Mantle network via the agent's hot wallet using Alloy.

use alloy::{
    network::EthereumWallet,
    primitives::{Address, B256},
    providers::{Provider, ProviderBuilder},
    signers::local::PrivateKeySigner,
    sol,
};
use eyre::{eyre, Result, WrapErr};
use std::str::FromStr;
use url::Url;

// Compile-time safe interface declaration matching our Day 4 Solidity contract
sol! {
    #[sol(rpc)]
    interface IClaimRegistry {
        function submitClaim(bytes calldata seal, bytes calldata journal) external;
        function isClaimProcessed(bytes32 claimId) external view returns (bool);
    }
}

pub struct ChainSubmitter {
    rpc_urls: Vec<Url>,
    wallet: EthereumWallet,
    registry_address: Address,
    chain_id: u64,
    confirmation_depth: u64,
}

impl ChainSubmitter {
    /// Initializer creating an authenticated JSON-RPC pipeline with automatic fillers
    pub fn new(
        rpc_urls: &[String],
        private_key: &str,
        registry_address: &str,
        chain_id: u64,
        confirmation_depth: u64,
    ) -> Result<Self> {
        let signer =
            PrivateKeySigner::from_str(private_key).wrap_err("Invalid agent private key")?;
        let wallet = EthereumWallet::from(signer);

        if rpc_urls.is_empty() {
            return Err(eyre!("At least one RPC URL is required"));
        }
        let rpc_urls = rpc_urls
            .iter()
            .map(|url| Url::parse(url))
            .collect::<std::result::Result<Vec<_>, _>>()
            .wrap_err("Invalid RPC URL")?;
        let registry_addr =
            Address::from_str(registry_address).wrap_err("Invalid ClaimRegistry address")?;

        Ok(Self {
            rpc_urls,
            wallet,
            registry_address: registry_addr,
            chain_id,
            confirmation_depth,
        })
    }

    pub async fn verify_chain(&self) -> Result<()> {
        let mut failures = Vec::new();
        for rpc_url in &self.rpc_urls {
            let provider = ProviderBuilder::new().connect_http(rpc_url.clone());
            match provider.get_chain_id().await {
                Ok(actual) if actual == self.chain_id => return Ok(()),
                Ok(actual) => failures.push(format!(
                    "{rpc_url} returned chain {actual}, expected {}",
                    self.chain_id
                )),
                Err(error) => failures.push(format!("{rpc_url}: {error}")),
            }
        }
        Err(eyre!(
            "No healthy RPC for chain {}: {}",
            self.chain_id,
            failures.join("; ")
        ))
    }

    pub async fn is_claim_processed(&self, claim_id: [u8; 32]) -> Result<bool> {
        let mut failures = Vec::new();
        for rpc_url in &self.rpc_urls {
            let provider = ProviderBuilder::new().connect_http(rpc_url.clone());
            match provider.get_chain_id().await {
                Ok(actual) if actual == self.chain_id => {}
                Ok(actual) => {
                    failures.push(format!("{rpc_url} returned unexpected chain {actual}"));
                    continue;
                }
                Err(error) => {
                    failures.push(format!("{rpc_url}: {error}"));
                    continue;
                }
            }

            let contract = IClaimRegistry::new(self.registry_address, provider);
            match contract.isClaimProcessed(B256::from(claim_id)).call().await {
                Ok(result) => return Ok(result),
                Err(error) => failures.push(format!("{rpc_url}: {error}")),
            }
        }
        Err(eyre!(
            "Claim status lookup failed on every RPC: {}",
            failures.join("; ")
        ))
    }

    /// Autonomous transaction broadcaster for packaging the ZKP and submitting to Mantle/Anvil
    pub async fn submit_claim(&self, seal: Vec<u8>, journal: Vec<u8>) -> Result<String> {
        let mut failures = Vec::new();
        for rpc_url in &self.rpc_urls {
            let provider = ProviderBuilder::new()
                .wallet(self.wallet.clone())
                .connect_http(rpc_url.clone());

            match provider.get_chain_id().await {
                Ok(actual) if actual == self.chain_id => {}
                Ok(actual) => {
                    failures.push(format!("{rpc_url} returned unexpected chain {actual}"));
                    continue;
                }
                Err(error) => {
                    failures.push(format!("{rpc_url}: {error}"));
                    continue;
                }
            }

            let contract = IClaimRegistry::new(self.registry_address, provider);
            tracing::info!(
                "🔗 Constructing on-chain transaction payload via {}",
                rpc_url
            );
            let tx_call = contract.submitClaim(seal.clone().into(), journal.clone().into());

            tracing::info!("🚀 Broadcasting transaction to ClaimRegistry...");
            let pending_tx = match tx_call.send().await {
                Ok(pending_tx) => pending_tx,
                Err(error) => {
                    failures.push(format!("{rpc_url}: broadcast failed: {error}"));
                    continue;
                }
            };

            tracing::info!("⏳ Awaiting {} confirmation(s)...", self.confirmation_depth);
            let receipt = match pending_tx
                .with_required_confirmations(self.confirmation_depth)
                .get_receipt()
                .await
            {
                Ok(receipt) => receipt,
                Err(error) => {
                    failures.push(format!("{rpc_url}: confirmation failed: {error}"));
                    continue;
                }
            };

            if receipt.status() {
                return Ok(format!("{:?}", receipt.transaction_hash));
            }
            failures.push(format!("{rpc_url}: transaction reverted"));
        }

        Err(eyre!(
            "Transaction failed on every RPC: {}",
            failures.join("; ")
        ))
    }
}
