//! Kedge ZKP Guest Program — Claim Evaluator
//!
//! This program runs inside the RISC Zero zkVM. It:
//! 1. Reads shipment data (`ClaimInput`) as private input from the host
//! 2. Evaluates the parametric insurance conditions
//! 3. Commits the evaluation result (`ClaimOutput`) to the journal
//!
//! The journal becomes the public output — verifiable on-chain without
//! revealing the underlying shipment telemetry data.

#![no_std]
#![no_main]

use alloy_sol_types::SolValue;
use kedge_core::{evaluate, verify_oracle_signature, ClaimInput};
use risc0_zkvm::guest::env;

risc0_zkvm::guest::entry!(main);

fn main() {
    // 1. Read the private witness (shipment data) from the host
    // The input is passed via env::read() using standard serialization
    let claim_input: ClaimInput = env::read();

    // 2. Authenticate the logistics oracle payload and validate its time window.
    assert!(
        verify_oracle_signature(&claim_input),
        "invalid logistics oracle signature"
    );
    assert!(
        claim_input.payload.issued_at <= claim_input.payload.event_timestamp,
        "event predates oracle issuance"
    );
    assert!(
        claim_input.payload.event_timestamp <= claim_input.payload.expires_at,
        "event is outside oracle validity window"
    );
    assert!(claim_input.payload.chain_id != 0, "invalid target chain");

    // 3. Evaluate the claim against the parametric rules
    let claim_output = evaluate(&claim_input);

    // 4. ABI encode the authenticated result into the public journal.
    // We use env::commit_slice because we are writing standard Solidity ABI-encoded bytes,
    // not a serialized Rust struct.
    let encoded = claim_output.abi_encode();
    env::commit_slice(&encoded);
}
