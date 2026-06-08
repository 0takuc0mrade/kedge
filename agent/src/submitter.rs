//! On-chain transaction submitter for the Kedge agent.
//!
//! Handles signing and broadcasting claim settlement transactions
//! to the Mantle Sepolia network via the agent's hot wallet.
//!
//! This module will be fully implemented on Day 7 when the smart
//! contracts are deployed and ready for integration.

use eyre::Result;
use tracing::info;

use crate::config::Config;
use crate::evaluator::ClaimEvaluation;

/// Submit a verified claim to the on-chain ClaimRegistry.
///
/// # Flow
/// 1. Encode the ZKP seal + journal into calldata
/// 2. Build the transaction targeting ClaimRegistry.submitClaim()
/// 3. Sign with the agent's hot wallet private key
/// 4. Broadcast to Mantle Sepolia and await confirmation
///
/// # Returns
/// The transaction hash as a hex string on success.
pub async fn submit_claim(
    _cfg: &Config,
    _evaluation: &ClaimEvaluation,
    _zkp_seal: &[u8],
    _journal: &[u8],
) -> Result<String> {
    // TODO: Day 7 — Full implementation
    //
    // let signer = PrivateKeySigner::from_str(&cfg.agent_private_key)?;
    // let provider = ProviderBuilder::new()
    //     .with_recommended_fillers()
    //     .signer(EthereumSigner::from(signer))
    //     .on_http(cfg.rpc_url.parse()?);
    //
    // let claim_registry = ClaimRegistry::new(
    //     cfg.claim_registry_address.parse()?,
    //     &provider,
    // );
    //
    // let tx = claim_registry
    //     .submitClaim(zkp_seal.into(), image_id, journal.into())
    //     .send()
    //     .await?
    //     .get_receipt()
    //     .await?;
    //
    // Ok(format!("{:?}", tx.transaction_hash))

    info!("📡 [STUB] Would submit claim for {} to ClaimRegistry", _evaluation.tracking_id);
    Ok("0x_placeholder_tx_hash".to_string())
}
