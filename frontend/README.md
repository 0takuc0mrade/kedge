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
- a one-time signed coverage mandate before autonomous monitoring begins
- responsive layouts and reduced-motion support

The current claim and activity cards are presentation telemetry. They are not
yet sourced from a live indexer. The Rust agent remains the authority for
ingestion, proving, and settlement.

Coverage activation is a hackathon demo bridge: the claimant signs a canonical
mandate and the browser stores the resulting policy reference locally. No
premium is charged and the mandate is not yet written on-chain. A production
version requires a `PolicyRegistry`, premium collection, and agent ingestion of
registered policy events.

Open `http://localhost:3000/?activate=1` to begin a demo recording directly at
the activation step.
