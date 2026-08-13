# CircleFi Contracts

Soroban smart contract for on-chain rotating savings circles (ROSCA) on Stellar testnet.

## Features

- **Circle Management**: Create and manage rotating savings groups
- **Contribution Enforcement**: Track member contributions with validation
- **Payout Order**: Support both fixed rotation and bid-based payout ordering
- **Collateral & Default Handling**: Lock collateral and track defaults with slashing logic
- **Dispute Window**: Resolve disputes within configurable time windows
- **Trust Scoring**: Dynamic member trust scores based on payment history

## Building

```bash
cargo build --target wasm32-unknown-unknown --release
```

## Testing

```bash
cargo test
```

## Contract Functions

### `create_circle(owner, name, contribution_amount, cycle_duration_days, payout_order, collateral_percentage, dispute_window_hours)`
Create a new savings circle with specified parameters.

### `join_circle(circle_owner, member)`
Join an existing circle as a member.

### `contribute(circle_owner, member, amount)`
Record a contribution from a member to the current round.

### `place_bid(circle_owner, member, bid_amount)`
Place a bid for payout priority (bid-based circles only).

### `trigger_default(circle_owner, member)`
Mark a member as defaulted and apply slashing.

### `dispute_payout(circle_owner, disputer, round_id)`
Dispute a payout decision within the dispute window.

### `end_round(circle_owner)`
Finalize current round and move to next.

### `get_circle(circle_owner)` / `get_member(circle_owner, member)` / `get_round(circle_owner, round_id)`
Query contract state.

## Stellar Network

Configured for Stellar testnet. Update RPC endpoints in deployment scripts as needed.

## License

MIT