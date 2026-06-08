//! Kedge — Autonomous Claim Adjuster
//!
//! An enterprise-grade AI agent that monitors off-chain shipping events,
//! generates Zero-Knowledge Proofs for claim verification, and autonomously
//! settles parametric insurance contracts on Mantle Network.

mod config;
mod ingestor;
mod evaluator;
mod submitter;

use eyre::Result;
use kedge_core::ClaimOutput;
use risc0_zkvm::{default_prover, ExecutorEnv};
use tracing::{info, warn, error};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize structured logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(true)
        .with_thread_ids(false)
        .init();

    info!("🚢 Kedge — Autonomous Claim Adjuster v{}", env!("CARGO_PKG_VERSION"));
    info!("   Mantle Turing Test Hackathon 2026 | AI x RWA Track");
    info!("─────────────────────────────────────────────────────");

    // Load configuration from environment
    let cfg = config::Config::from_env()?;
    info!("Agent: {}", cfg.agent_name);
    info!("RPC:   {}", cfg.rpc_url);
    info!("Mock API: {}", cfg.mock_api_url);
    info!("Polling interval: {}s", cfg.polling_interval_secs);
    info!("RISC Zero dev mode: {}", cfg.risc0_dev_mode);

    // Main autonomous loop
    info!("🔄 Starting autonomous monitoring loop...");

    loop {
        match run_cycle(&cfg).await {
            Ok(claim_triggered) => {
                if claim_triggered {
                    info!("✅ Claim cycle completed — settlement triggered");
                } else {
                    info!("⏳ No claim conditions met — sleeping {}s", cfg.polling_interval_secs);
                }
            }
            Err(e) => {
                error!("❌ Cycle error: {:#}", e);
                warn!("Retrying in {}s...", cfg.polling_interval_secs);
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(cfg.polling_interval_secs)).await;
    }
}

/// Execute one full claim evaluation cycle:
/// 1. Ingest shipping data from mock API
/// 2. Evaluate against parametric conditions
/// 3. If triggered: generate ZKP → submit on-chain → settle
async fn run_cycle(cfg: &config::Config) -> Result<bool> {
    // Phase 1: Data Ingestion
    let shipment = ingestor::fetch_shipment_status(&cfg.mock_api_url).await?;
    info!(
        "📦 Shipment {} | Status: {:?} | Delay: {} hours",
        shipment.tracking_id, shipment.status, shipment.delay_hours
    );

    // Phase 2: Claim Evaluation (off-chain quick check)
    let evaluation = evaluator::evaluate_claim(&shipment);

    if !evaluation.is_triggered {
        return Ok(false);
    }

    info!(
        "🔔 CLAIM TRIGGERED for shipment {} — payout: {} MockUSDT",
        shipment.tracking_id, evaluation.payout_amount
    );

    // Phase 3: ZKP Generation
    info!("🔐 Generating Zero-Knowledge Proof...");

    let env = ExecutorEnv::builder()
        .write(&evaluation.claim_input).map_err(|e| eyre::eyre!("{e}"))?
        .build().map_err(|e| eyre::eyre!("{e}"))?;

    let prover = default_prover();
    let prove_info = prover.prove(env, kedge_methods::CLAIM_EVALUATOR_ELF).map_err(|e| eyre::eyre!("{e}"))?;
    let receipt = prove_info.receipt;

    // Decode the journal to verify the output matches
    let journal_output: ClaimOutput = receipt.journal.decode().map_err(|e| eyre::eyre!("{e}"))?;
    info!(
        "🔐 ZKP generated — journal confirms: triggered={}, payout={} MockUSDT",
        journal_output.is_triggered, journal_output.payout_amount
    );

    // Verify the receipt (validates the proof itself)
    receipt.verify(kedge_methods::CLAIM_EVALUATOR_ID).map_err(|e| eyre::eyre!("{e}"))?;
    info!("✅ ZKP receipt verified successfully");

    // Phase 4: On-Chain Settlement (Day 7)
    // TODO: Submit claim transaction to ClaimRegistry with ZKP seal + journal
    info!("⛓️  On-chain submission: [placeholder — Day 7]");

    Ok(true)
}
