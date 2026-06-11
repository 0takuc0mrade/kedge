# Kedge Contracts

Solidity contracts for Kedge's identity-gated parametric claim settlement.

## Contracts

- `AgentIdentityRegistry.sol`: mints the Kedge agent identity and exposes agent
  authorization checks.
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

For local deployment, start Anvil and run:

```bash
forge script script/Deploy.s.sol:DeployScript \
  --rpc-url http://127.0.0.1:8545 \
  --broadcast
```

The deployment script currently uses `RiscZeroMockVerifier` and is for local
development only.
