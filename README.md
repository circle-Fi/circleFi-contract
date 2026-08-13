# CircleFi Contracts

Production-ready Soroban smart contract for on-chain rotating savings circles (ROSCA) on Stellar testnet.

## Features

- **Circle Management**: Create and manage rotating savings groups with configurable parameters
- **Contribution Enforcement**: Track and validate member contributions with error handling
- **Payout Order**: Support both fixed rotation and bid-based payout ordering
- **Collateral & Default Handling**: Lock collateral and track defaults with progressive slashing logic
- **Dispute Window**: Resolve disputes within configurable time windows
- **Trust Scoring**: Dynamic member trust scores (0-100) based on payment history
- **Member Status Tracking**: Active, Defaulted, Suspended, or Exited states
- **Comprehensive Validation**: Input validation for all parameters
- **Production Error Handling**: Detailed error codes for debugging

## Building

```bash
cargo build --target wasm32-unknown-unknown --release
```

## Testing

```bash
cargo test
```

## Deployment

See [DEPLOYMENT.md](DEPLOYMENT.md) for detailed deployment instructions.

Quick start:
```bash
soroban contract deploy --network testnet --source circlefi-deployer \
  target/wasm32-unknown-unknown/release/circlefi_contract.wasm
```

## Contract Functions

### Circle Management

#### `create_circle(owner, name, contribution_amount, cycle_duration_days, payout_order, collateral_percentage, dispute_window_hours, max_members, min_trust_score)`
Create a new savings circle with specified parameters.

**Parameters:**
- `owner: Address` - Circle creator/admin
- `name: String` - Circle name
- `contribution_amount: i128` - Required contribution per member
- `cycle_duration_days: u32` - Duration of each round (1-365)
- `payout_order: PayoutOrder` - Fixed (0) or Bid (1)
- `collateral_percentage: u32` - Collateral requirement (0-100)
- `dispute_window_hours: u32` - Time to dispute payouts (1-720)
- `max_members: u32` - Maximum members (1-100)
- `min_trust_score: u32` - Minimum trust score to join (0-100)

**Returns:** Owner address on success, ErrorCode on failure

#### `join_circle(circle_owner, member)`
Join an existing circle as a member.

**Validations:**
- Member not already joined
- Circle is active
- Circle has available slots
- Member meets minimum trust score

#### `deactivate_circle(circle_owner)`
Deactivate a circle (admin only).

### Member Management

#### `contribute(circle_owner, member, amount)`
Record a contribution from a member to the current round.

**Validations:**
- Amount matches circle requirement
- Member is active
- Circle is active

#### `get_member(circle_owner, member)`
Retrieve member information and status.

**Returns:** Member struct with:
- Contribution and payout counts
- Collateral locked
- Default count and trust score
- Status and join date

#### `trigger_default(circle_owner, member)`
Mark a member as defaulted and apply progressive slashing.

**Effects:**
- Sets member status to Defaulted
- Locks collateral
- Reduces trust score (15 * default_count)
- Suspends member if trust score reaches 0

### Payout Management

#### `place_bid(circle_owner, member, bid_amount)`
Place a bid for payout priority (bid-based circles only).

**Validations:**
- Circle uses bid payout order
- Bid amount is positive

#### `end_round(circle_owner)`
End current round and move to next (admin only).

**Validations:**
- Round has expired
- Updates total volume

#### `finalize_round(circle_owner, round_id, payout_recipient)`
Finalize a round and award payout (admin only).

**Effects:**
- Marks round as finalized
- Records payout recipient
- Increases recipient's trust score (+5)

### Dispute Resolution

#### `dispute_payout(circle_owner, disputer, round_id)`
Dispute a payout decision within the window.

**Validations:**
- Dispute window not expired
- Round not already disputed

### Query Functions

#### `get_circle(circle_owner)`
Get circle configuration and stats.

**Returns:**
- Circle metadata and parameters
- Current round and member count
- Total volume

#### `get_round(circle_owner, round_id)`
Get round details and status.

**Returns:**
- Round timing and pot
- Payout recipient and bids
- Dispute status

## Error Codes

| Code | Meaning |
|------|---------|
| 1 | CircleNotFound |
| 2 | MemberNotFound |
| 3 | RoundNotFound |
| 4 | InvalidAmount |
| 5 | UnauthorizedAccess |
| 6 | InsufficientCollateral |
| 7 | InvalidPayoutOrder |
| 8 | RoundNotEnded |
| 9 | InvalidParameters |
| 10 | MemberAlreadyJoined |
| 11 | CircleNotActive |
| 12 | MaxMembersReached |
| 13 | InsufficientTrustScore |

## Data Structures

### Circle
```rust
{
  owner: Address,
  name: String,
  contribution_amount: i128,
  cycle_duration_days: u32,
  payout_order: PayoutOrder (Fixed=0, Bid=1),
  collateral_percentage: u32,
  dispute_window_hours: u32,
  current_round: u32,
  is_active: bool,
  created_at: u64,
  member_count: u32,
  max_members: u32,
  total_volume: i128,
  min_trust_score: u32
}
```

### Member
```rust
{
  address: Address,
  contributions_made: u32,
  payouts_received: u32,
  collateral_locked: i128,
  default_count: u32,
  trust_score: u32,
  joined_at: u64,
  last_contribution: u64,
  status: MemberStatus (Active=0, Defaulted=1, Suspended=2, Exited=3),
  total_contributed: i128
}
```

### Round
```rust
{
  circle: Address,
  round_id: u32,
  start_time: u64,
  end_time: u64,
  total_pot: i128,
  payout_recipient: Address,
  highest_bid: i128,
  contributions_received: u32,
  member_count: u32,
  disputed: bool,
  dispute_resolver: Address,
  is_finalized: bool
}
```

## Testing Examples

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_create_circle

# Run with output
cargo test -- --nocapture
```

## Security

- ✅ All function calls require authorization
- ✅ Input validation on all parameters
- ✅ Overflow/underflow protection via Rust types
- ✅ No reentrancy vulnerabilities (state-based)
- ✅ Proper error propagation
- ✅ Trust score prevents sybil attacks
- ✅ Collateral enforcement

## Performance

- Efficient storage access via keyed data structures
- O(1) lookups for circles, members, and rounds
- No expensive iterations in production code

## License

MIT
