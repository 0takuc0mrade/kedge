# Mantle Sepolia Deployment

Kedge was deployed to Mantle Sepolia on June 13, 2026.

- Chain ID: `5003`
- Agent ID: `1`
- Agent wallet: `0x9E4997610B7C7bB122E1Dc7C10cC5a800987AD7c`
- Claim evaluator image ID:
  `0xc57ec85a2b1d9dd2f93b60b05055e40c763ef7fc2cf4adf9a4ba71916483d346`
- Authorized oracle key hash:
  `0xdacdad000a5fa7625e2e150e0927eb8124e94af0db916aab6aa6546f67d56391`

## Contracts

| Contract | Address |
| --- | --- |
| RISC Zero Groth16 verifier | [`0xB950...C16f`](https://explorer.sepolia.mantle.xyz/address/0xB950c3799a349Ba6A9264382BC60799F2a35C16f) |
| Agent identity registry | [`0x1A47...fb03`](https://explorer.sepolia.mantle.xyz/address/0x1A47e658ddD31C18ec801BE2128F7cF873d2fb03) |
| MockUSDT | [`0x3816...0c8a`](https://explorer.sepolia.mantle.xyz/address/0x38165e85d4220580f6515bEEB93cF4F0EEd50c8a) |
| Claim registry | [`0xC5e3...314b`](https://explorer.sepolia.mantle.xyz/address/0xC5e3Ffa61D3B3e2c8d8F132917e595A19562314b) |
| Settlement vault | [`0x1FC3...5dDA`](https://explorer.sepolia.mantle.xyz/address/0x1FC353C48b00F5DaB51c736828fb08EcA7F25dDA) |

The vault was initialized with `100,000 MockUSDT`.

## Transactions

| Action | Transaction |
| --- | --- |
| Deploy Groth16 verifier | [`0x8db647...6531d`](https://explorer.sepolia.mantle.xyz/tx/0x8db6475e7df77e1a922820deebd0dabf3d527a9c2c41ee7df8fa7623c4e6531d) |
| Deploy identity registry | [`0xcd03ab...48027`](https://explorer.sepolia.mantle.xyz/tx/0xcd03ab5bd349bb0457ac34bb88fcfd7c9a719ce7d1d7f0f7bd723b1fa6248027) |
| Register Kedge identity | [`0xd4a8bf...e581f`](https://explorer.sepolia.mantle.xyz/tx/0xd4a8bf6cc273d329cc4d07d7dd5971b34665748d8254cb7b0967672e014e581f) |
| Deploy MockUSDT | [`0x2603d9...b919`](https://explorer.sepolia.mantle.xyz/tx/0x2603d9d6e48f5b0729d236301d974074b8057e7f9a9dd424e38107902354b919) |
| Deploy claim registry | [`0x9b6aca...e7b7`](https://explorer.sepolia.mantle.xyz/tx/0x9b6aca31a5ae23de04b8d026ae66add68c24f332c3c564a5b7076976271fe7b7) |
| Deploy settlement vault | [`0x763327...95ea`](https://explorer.sepolia.mantle.xyz/tx/0x76332729749fac00031fd3efa7a50d147dc2d0987fef35a4384b32c3b02695ea) |
| Configure vault | [`0xaab966...e3fe`](https://explorer.sepolia.mantle.xyz/tx/0xaab9663509983cd7b758a28da0b7cf695872c426de16f4bcce6022f4541ae3fe) |
| Fund vault | [`0xdf060f...67f1`](https://explorer.sepolia.mantle.xyz/tx/0xdf060f2693aa44e30b36ffb2d6300d4c080b05e963ad536e82ea0549e72067f1) |

All transactions succeeded and the deployed configuration was read back from
the chain after broadcast.
