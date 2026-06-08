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

use kedge_core::{evaluate, ClaimInput};
use risc0_zkvm::guest::env;

risc0_zkvm::guest::entry!(main);

fn main() {
    // Read the private input from the host
    let input: ClaimInput = env::read();

    // Evaluate the claim using the shared parametric logic
    let output = evaluate(&input);

    // Commit the evaluation result to the journal (public output)
    env::commit(&output);
}
