//! Kedge — Autonomous Claim Adjuster
//!
//! An enterprise-grade AI agent that monitors off-chain shipping events,
//! generates Zero-Knowledge Proofs for claim verification, and autonomously
//! settles parametric insurance contracts on Mantle Network.

mod config;
mod evaluator;
mod ingestor;
mod state;
mod submitter;

use eyre::Result;
use kedge_core::ClaimOutput;
use risc0_ethereum_contracts::encode_seal;
use risc0_zkvm::sha::Digestible;
use risc0_zkvm::{default_prover, ExecutorEnv, ProverOpts, Receipt};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .with_thread_ids(false)
        .init();

    info!(
        "🚢 Kedge — Autonomous Claim Adjuster v{}",
        env!("CARGO_PKG_VERSION")
    );
    info!("   Mantle Turing Test Hackathon 2026 | AI x RWA Track");
    info!("─────────────────────────────────────────────────────");

    // Print the image ID for the deploy script
    info!(
        "CLAIM_EVALUATOR_ID: {:?}",
        kedge_methods::CLAIM_EVALUATOR_ID
    );

    // Load configuration from environment
    let cfg = config::Config::from_env()?;
    info!("Agent: Kedge");
    info!("RPC endpoints: {}", cfg.rpc_urls.len());
    info!("Identity Registry: {}", cfg.erc8004_identity_registry);
    info!("Mock API: {}", cfg.mock_api_url);
    info!("Polling interval: {}s", cfg.polling_interval_secs);
    info!("RISC Zero dev mode: {}", cfg.risc0_dev_mode);

    // Initialize the on-chain submitter
    let submitter = submitter::ChainSubmitter::new(
        &cfg.rpc_urls,
        &cfg.agent_private_key,
        &cfg.claim_registry_address,
        cfg.chain_id,
        cfg.confirmation_depth,
    )
    .map_err(|e| eyre::eyre!("Failed to initialize ChainSubmitter: {}", e))?;
    submitter
        .verify_chain()
        .await
        .map_err(|e| eyre::eyre!("RPC chain verification failed: {e}"))?;
    let mut state = state::StateStore::load(&cfg.state_file)?;

    // Main autonomous loop
    info!("🔄 Starting autonomous monitoring loop...");

    let mut consecutive_failures = 0u32;
    loop {
        let sleep_secs = match run_cycle(&cfg, &submitter, &mut state).await {
            Ok(claim_triggered) => {
                consecutive_failures = 0;
                if claim_triggered {
                    info!("✅ Claim cycle completed — settlement triggered");
                } else {
                    info!(
                        "⏳ No claim conditions met — sleeping {}s",
                        cfg.polling_interval_secs
                    );
                }
                cfg.polling_interval_secs
            }
            Err(e) => {
                consecutive_failures = consecutive_failures.saturating_add(1);
                let multiplier = 1u64 << consecutive_failures.min(10);
                let delay = cfg
                    .polling_interval_secs
                    .saturating_mul(multiplier)
                    .min(cfg.max_backoff_secs);
                error!("❌ Cycle error: {:#}", e);
                warn!("Retrying in {}s...", delay);
                delay
            }
        };

        tokio::time::sleep(std::time::Duration::from_secs(sleep_secs)).await;
    }
}

/// Execute one full claim evaluation cycle:
/// 1. Ingest shipping data from mock API
/// 2. Evaluate against parametric conditions
/// 3. If triggered: generate ZKP → submit on-chain → settle
async fn run_cycle(
    cfg: &config::Config,
    submitter: &submitter::ChainSubmitter,
    state: &mut state::StateStore,
) -> Result<bool> {
    // Phase 1: Data Ingestion
    let shipment = ingestor::fetch_shipment_status(&cfg.mock_api_url).await?;
    info!(
        "📦 Shipment {} | Status: {:?} | Delay: {} hours",
        shipment.tracking_id, shipment.status, shipment.delay_hours
    );

    // Phase 2: Claim Evaluation (off-chain quick check)
    let evaluation = evaluator::evaluate_claim(&shipment)?;

    if !evaluation.is_triggered {
        return Ok(false);
    }

    if state.contains(&evaluation.claim_id) {
        info!("Claim already recorded locally; skipping proof generation");
        return Ok(false);
    }
    if submitter.is_claim_processed(evaluation.claim_id).await? {
        info!("Claim already settled on-chain; updating local state");
        state.mark_processed(&evaluation.claim_id)?;
        return Ok(false);
    }

    info!(
        "🔔 CLAIM TRIGGERED for shipment {} — payout: {} MockUSDT",
        shipment.tracking_id, evaluation.payout_amount
    );

    // Phase 3: ZKP Generation
    info!("🔐 Generating Zero-Knowledge Proof...");

    let env = ExecutorEnv::builder()
        .write(&evaluation.claim_input)
        .map_err(|e| eyre::eyre!("{e}"))?
        .build()
        .map_err(|e| eyre::eyre!("{e}"))?;

    let receipt = prove_claim(env, cfg.risc0_dev_mode)?;

    // Decode the journal to verify the output matches using ABI decoding
    use alloy::sol_types::SolValue;
    let journal_output =
        ClaimOutput::abi_decode(&receipt.journal.bytes).map_err(|e| eyre::eyre!("{e}"))?;
    info!(
        "🔐 ZKP generated — journal confirms: triggered={}, payout={} MockUSDT",
        journal_output.isTriggered, journal_output.payoutAmount
    );

    // Verify the receipt (validates the proof itself)
    receipt
        .verify(kedge_methods::CLAIM_EVALUATOR_ID)
        .map_err(|e| eyre::eyre!("{e}"))?;
    info!("✅ ZKP receipt verified successfully off-chain.");

    let seal = evm_seal(&receipt, cfg.risc0_dev_mode)?;

    let journal_bytes = receipt.journal.bytes.clone();

    // Phase 4: On-Chain Settlement
    match submitter.submit_claim(seal, journal_bytes).await {
        Ok(tx_hash) => {
            state.mark_processed(&evaluation.claim_id)?;
            info!("🏆 SUCCESSFUL SETTLEMENT | Tx Hash: {}", tx_hash);
        }
        Err(e) => {
            error!("❌ TRANSACTION FAILURE: {}", e);
            return Err(eyre::eyre!("{e}"));
        }
    }

    Ok(true)
}

fn prove_claim(env: ExecutorEnv<'_>, dev_mode: bool) -> Result<Receipt> {
    let prover = default_prover();
    let prove_info = if dev_mode {
        prover.prove(env, kedge_methods::CLAIM_EVALUATOR_ELF)
    } else {
        prover.prove_with_opts(
            env,
            kedge_methods::CLAIM_EVALUATOR_ELF,
            &ProverOpts::groth16(),
        )
    }
    .map_err(|e| eyre::eyre!("RISC Zero proving failed: {e}"))?;

    Ok(prove_info.receipt)
}

fn evm_seal(receipt: &Receipt, dev_mode: bool) -> Result<Vec<u8>> {
    if !dev_mode {
        return encode_seal(receipt)
            .map_err(|e| eyre::eyre!("Failed to encode Groth16 seal for EVM: {e}"));
    }

    // The local deployment uses RiscZeroMockVerifier with this test-only selector.
    let claim_digest = receipt
        .claim()
        .map_err(|e| eyre::eyre!("Failed to get claim: {e}"))?
        .digest();
    let mut seal = vec![0x12, 0x34, 0x56, 0x78];
    seal.extend_from_slice(claim_digest.as_bytes());
    Ok(seal)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::sol_types::SolValue;
    use ed25519_dalek::{Signer, SigningKey};
    use kedge_core::{ClaimInput, LogisticsOraclePayload, ShipmentStatus};

    fn signed_claim_input() -> ClaimInput {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let payload = LogisticsOraclePayload {
            tracking_id: "KDG-ZKVM-0001".to_string(),
            policy_id: [0xCD; 32],
            claimant: [0xAB; 20],
            status: ShipmentStatus::CriticalDelay,
            delay_hours: 96,
            insured_value: 100_000,
            event_timestamp: 1_781_285_872,
            nonce: 1,
            issued_at: 1_781_285_872,
            expires_at: 1_781_286_172,
            chain_id: 5003,
        };
        let signature = signing_key.sign(&payload.signing_bytes());
        ClaimInput {
            payload,
            oracle_public_key: signing_key.verifying_key().to_bytes(),
            oracle_signature: signature.to_bytes().to_vec(),
        }
    }

    fn executor_env(input: &ClaimInput) -> ExecutorEnv<'_> {
        ExecutorEnv::builder()
            .write(input)
            .unwrap()
            .build()
            .unwrap()
    }

    #[test]
    fn signed_oracle_payload_executes_inside_zkvm() {
        std::env::set_var("RISC0_DEV_MODE", "1");

        let input = signed_claim_input();
        let env = ExecutorEnv::builder()
            .write(&input)
            .unwrap()
            .build()
            .unwrap();

        let receipt = default_prover()
            .prove(env, kedge_methods::CLAIM_EVALUATOR_ELF)
            .unwrap()
            .receipt;
        let output = ClaimOutput::abi_decode(&receipt.journal.bytes).unwrap();

        assert!(output.isTriggered);
        assert_eq!(output.payoutAmount, 50_000);
        assert_eq!(output.policyId.0, [0xCD; 32]);
        assert_eq!(output.chainId, 5003);

        let seal = evm_seal(&receipt, true).unwrap();
        assert_eq!(&seal[..4], &[0x12, 0x34, 0x56, 0x78]);
        assert_eq!(seal.len(), 36);
    }

    #[test]
    #[ignore = "requires Docker and generates a production Groth16 proof"]
    fn signed_oracle_payload_generates_evm_groth16_seal() {
        std::env::set_var("RISC0_DEV_MODE", "0");

        let input = signed_claim_input();
        let receipt = prove_claim(executor_env(&input), false).unwrap();
        receipt.verify(kedge_methods::CLAIM_EVALUATOR_ID).unwrap();

        let seal = evm_seal(&receipt, false).unwrap();
        let groth16 = receipt.inner.groth16().unwrap();

        assert_eq!(&seal[..4], &groth16.verifier_parameters.as_bytes()[..4]);
        assert_eq!(&seal[4..], groth16.seal.as_slice());
    }
}
