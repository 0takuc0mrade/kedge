# Kedge Contracts

Solidity contracts for Kedge's identity-gated parametric claim settlement.

## Contracts

- `AgentIdentityRegistry.sol`: ERC-8004-compatible registration, metadata,
  URI, verified agent-wallet, transfer, and authorization behavior.
- `ClaimRegistry.sol`: verifies RISC Zero receipts, decodes claim journals,
  blocks replayed claims, and triggers payouts.
- `SettlementVault.sol`: holds ERC-20 liquidity and permits disbursement only
  through the claim registry.
- `MockUSDT.sol`: demonstration settlement asset.

## Commands

```bash
git submodule update --init --recursive
forge build
forge test
```

For local deployment with the mock verifier, start Anvil and run:

```bash
forge script script/Deploy.s.sol:DeployLocal \
  --rpc-url http://127.0.0.1:8545 \
  --broadcast
```

For Mantle Sepolia, configure the variables documented in the root
`.env.example`, simulate, then broadcast:

```bash
forge script script/Deploy.s.sol:DeployMantleSepolia \
  --rpc-url "$MANTLE_SEPOLIA_RPC"

forge script script/Deploy.s.sol:DeployMantleSepolia \
  --rpc-url "$MANTLE_SEPOLIA_RPC" \
  --broadcast
```

The testnet script deploys `RiscZeroGroth16Verifier` v3.0.1 with the matching
control IDs; it does not use `RiscZeroMockVerifier`.
