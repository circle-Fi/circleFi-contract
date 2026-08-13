# Production Deployment Guide

## Prerequisites

- Rust 1.70+
- Soroban CLI (`stellar-cli`)
- Stellar testnet account with XLM

## Building

```bash
cargo build --target wasm32-unknown-unknown --release
```

## Deployment

### 1. Configure Stellar Network

```bash
soroban config network add testnet --rpc-url https://soroban-testnet.stellar.org --network-passphrase "Test SDF Network ; September 2015"
```

### 2. Create/Fund Account

```bash
soroban config identity generate circlefi-deployer
soroban config set-default-source-account circlefi-deployer

# Fund account at https://friendbot.stellar.org/
# Or use existing funded account:
soroban config identity show circlefi-deployer
```

### 3. Deploy Contract

```bash
soroban contract deploy \
  --network testnet \
  --source circlefi-deployer \
  --wasm target/wasm32-unknown-unknown/release/circlefi_contract.wasm
```

Save the returned contract ID.

## Testing

```bash
cargo test
```

## Contract Functions

See [README.md](../README.md) for full API documentation.

## Security Considerations

- All amounts validated against contribution requirements
- Trust scores prevent low-quality members from defaulting repeatedly
- Collateral locked and accessible via default trigger
- Dispute window prevents stale payout challenges
- All state transitions require proper authorization

## Cost Optimization

- Uses Soroban instance storage for efficient data access
- Batch operations supported for gas efficiency
- Consider optimizing round finalization for large circles

## Monitoring

- Track default events per member
- Monitor trust score distributions
- Alert on circle deactivation
