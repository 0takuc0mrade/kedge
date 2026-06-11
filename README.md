# Kedge

Kedge is an autonomous claim adjuster for parametric shipping insurance, built
for the Mantle Turing Test Hackathon 2026 AI x RWA track.

The agent polls shipment telemetry, evaluates a deterministic insurance policy
inside the RISC Zero zkVM, and submits the resulting proof journal to smart
contracts on Mantle. An on-chain agent identity controls who may submit claims,
and a settlement vault pays qualifying claims in ERC-20 tokens.

## Architecture

```text
Freight API
    |
    v
Kedge agent (Rust/Tokio)
    |
    +-- local policy evaluation
    |
    +-- RISC Zero guest execution
            |
            v
      proof + ABI journal
            |
            v
ClaimRegistry (Mantle)
    |
    +-- verifies the RISC Zero receipt
    +-- checks the Kedge agent identity
    +-- prevents duplicate claims
    |
    v
SettlementVault --> ERC-20 payout
```

## Repository Layout

| Path | Purpose |
| --- | --- |
| `agent/` | Autonomous polling, proving, and Alloy transaction broadcaster |
| `core/` | Shared `no_std` policy logic and ABI-compatible claim types |
| `methods/` | RISC Zero host bindings and zkVM guest program |
| `contracts/` | Solidity identity, claim registry, and settlement contracts |
| `mock-api/` | FastAPI freight telemetry simulator |

## Parametric Policy

Claims require a critical delay or lost shipment status and at least 48 hours
of delay.

| Delay | Payout |
| --- | ---: |
| 48-71 hours | 25% |
| 72-119 hours | 50% |
| 120-239 hours | 75% |
| 240+ hours | 100% |

## Local Development

### Prerequisites

- Rust and Cargo
- RISC Zero toolchain
- Foundry (`forge`, `anvil`, and `cast`)
- Python 3.11+

### Configure

```bash
git submodule update --init --recursive
cp .env.example .env
```

Fill in the wallet and deployed contract values in `.env`. Never commit this
file or a funded private key.

### Run the freight simulator

```bash
python -m venv mock-api/.venv
source mock-api/.venv/bin/activate
pip install -r mock-api/requirements.txt
python mock-api/server.py
```

### Test

```bash
cargo test --workspace
cd contracts && forge test
```

### Run the agent

Start Anvil or configure Mantle Sepolia contract addresses, then run:

```bash
cargo run -p kedge-agent
```

`RISC0_DEV_MODE=1` is intended only for local development. Production or
testnet settlement must use a real verifier and cryptographically secure
receipts.

## Smart Contracts

- `AgentIdentityRegistry`: identity NFT and authorized-agent lookup
- `ClaimRegistry`: proof verification, replay protection, and settlement trigger
- `SettlementVault`: restricted ERC-20 treasury disbursement
- `MockUSDT`: local and testnet demonstration token

The current deployment script uses RISC Zero's mock verifier for local Anvil
development. Replace it with the network verifier before a production-style
deployment.

## Status

- Shared parametric policy and zkVM guest implemented
- ABI-compatible RISC Zero journal implemented
- Alloy transaction broadcasting implemented
- Identity-gated settlement contracts implemented
- Local autonomous claim-to-payout loop verified
- Mantle Sepolia deployment and real Groth16 proving in progress

## Security Notice

Kedge is hackathon software and has not been audited. Do not use it with real
funds or production insurance contracts.
