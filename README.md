# Kedge

Kedge is an autonomous claim adjuster for parametric shipping insurance, built
for the Mantle Turing Test Hackathon 2026 AI x RWA track.

The agent polls oracle-signed shipment telemetry, verifies the oracle signature
and evaluates a deterministic insurance policy inside the RISC Zero zkVM, then
submits the resulting Groth16 receipt and ABI journal to smart contracts on
Mantle. An ERC-8004-compatible identity controls who may submit claims, and a
settlement vault pays qualifying claims in ERC-20 tokens.

## Architecture

```text
Freight API
    |
    +-- Ed25519-signed policy + shipment event
    |
    v
Kedge agent (Rust/Tokio)
    |
    +-- signature and chain preflight
    +-- persistent/on-chain replay check
    |
    +-- RISC Zero guest execution
            |
            v
      Groth16 proof + ABI journal
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
| `docs/` | Deployment and ERC-8004 registration artifacts |

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

Fill in the wallet, oracle, policy, and deployed contract values in `.env`.
Never commit this file or a funded private key. Set `KEDGE_ENV=testnet` to make
the mock API fail closed unless stable oracle, claimant, and policy values are
provided.

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

The production Groth16 integration test is ignored by default because it runs a
large Docker prover:

```bash
cargo test -p kedge-agent \
  signed_oracle_payload_generates_evm_groth16_seal \
  -- --ignored --nocapture
```

Run it on a machine with enough memory; it is intentionally not required for
routine development.

### Run the agent

Start Anvil or configure Mantle Sepolia contract addresses, then run:

```bash
cargo run -p kedge-agent
```

`RISC0_DEV_MODE=1` is intended only for local development. Production or
testnet settlement must use a real verifier and cryptographically secure
receipts.

The runtime verifies the connected chain before polling, supports comma-
separated `RPC_FALLBACK_URLS`, checks on-chain replay state before proving,
persists completed claim IDs under `.kedge/`, waits for configurable
confirmations, and applies bounded exponential backoff after failures.

## Smart Contracts

- `AgentIdentityRegistry`: ERC-8004 registration, URI, metadata, verified agent
  wallet, and authorized-agent lookup
- `ClaimRegistry`: proof verification, replay protection, and settlement trigger
- `SettlementVault`: restricted ERC-20 treasury disbursement
- `MockUSDT`: local and testnet demonstration token

## Mantle Sepolia Deployment

Generate the current guest image ID:

```bash
cargo run -q -p kedge-methods --example image_id
```

Configure every deployment variable in `.env`, including a distinct funded
`DEPLOYER_PRIVATE_KEY`, `AGENT_WALLET_ADDRESS`, `ORACLE_KEY_HASH`,
`CLAIM_EVALUATOR_IMAGE_ID`, `AGENT_METADATA_URI`, and
`VAULT_FUNDING_AMOUNT`. Then simulate before broadcasting:

```bash
cd contracts
forge script script/Deploy.s.sol:DeployMantleSepolia \
  --rpc-url "$MANTLE_SEPOLIA_RPC"
```

Broadcast only after the simulation succeeds:

```bash
forge script script/Deploy.s.sol:DeployMantleSepolia \
  --rpc-url "$MANTLE_SEPOLIA_RPC" \
  --broadcast
```

The script deploys the RISC Zero v3 Groth16 verifier directly because the
upstream deployment registry does not list a Mantle Sepolia verifier.

## Status

- Oracle-signed logistics payload verification implemented inside the zkVM
- ABI journal binds policy, claimant, oracle, payload, expiry, and chain
- RISC Zero v3 EVM seal encoding and production Groth16 path implemented
- Hardened identity-gated contracts with 19 Foundry tests
- ERC-8004 registration, metadata, URI, and verified-wallet support implemented
- RPC failover, chain preflight, durable deduplication, confirmations, and
  exponential backoff implemented
- Mantle Sepolia deployment prepared; broadcast requires funded deployer and
  agent wallets
- Full local Groth16 generation is deferred to a higher-memory runner

## Security Notice

Kedge is hackathon software and has not been audited. Do not use it with real
funds or production insurance contracts.
