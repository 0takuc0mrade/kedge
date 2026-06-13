# Kedge Frontend

Next.js operations console for the Kedge autonomous claim adjuster.

## Commands

```bash
pnpm install
pnpm dev
pnpm lint
pnpm build
pnpm start
```

The interface includes:

- a live-style shipment and claim decision view
- the RISC Zero proof and settlement pipeline
- links to the deployed Mantle Sepolia contracts
- an optional injected-wallet connection with Mantle Sepolia switching
- responsive layouts and reduced-motion support

The current claim and activity cards are presentation telemetry. They are not
yet sourced from a live indexer. The Rust agent remains the authority for
ingestion, proving, and settlement.
