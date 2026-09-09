#!/usr/bin/env bash
# Build, deploy and open a circle on testnet.
#
#   ./scripts/deploy.sh                       # 3 members, 100 units, 1-minute rounds
#   ./scripts/deploy.sh 5 1000000 604800      # capacity, contribution, round seconds
#
# Prints the circle's contract id on the last line, which is what the frontend
# and the SDK take.
set -euo pipefail

CAPACITY="${1:-3}"
CONTRIBUTION="${2:-100}"
ROUND_SECONDS="${3:-60}"
IDENTITY="${IDENTITY:-circlefi}"
NETWORK="${NETWORK:-testnet}"

command -v stellar >/dev/null || {
  echo "the stellar CLI is required: cargo install --locked stellar-cli" >&2
  exit 1
}

echo "==> building"
cargo build --target wasm32-unknown-unknown --release
WASM=target/wasm32-unknown-unknown/release/circlefi_circle.wasm
ls -l "$WASM"

echo "==> identity"
stellar keys generate --global "$IDENTITY" --network "$NETWORK" --fund 2>/dev/null || true
ADMIN=$(stellar keys address "$IDENTITY")
echo "admin: $ADMIN"

echo "==> a token for the circle to be denominated in"
# On testnet the native asset's own contract is the simplest real SEP-41 token.
TOKEN=$(stellar contract id asset --asset native --network "$NETWORK")
echo "token: $TOKEN"

echo "==> deploying"
CIRCLE=$(stellar contract deploy \
  --wasm "$WASM" \
  --source "$IDENTITY" \
  --network "$NETWORK" \
  -- \
  --admin "$ADMIN" \
  --token "$TOKEN" \
  --contribution "$CONTRIBUTION" \
  --round_seconds "$ROUND_SECONDS" \
  --capacity "$CAPACITY")

echo
echo "circle deployed"
echo "  capacity     $CAPACITY"
echo "  contribution $CONTRIBUTION"
echo "  round        ${ROUND_SECONDS}s"
echo
echo "$CIRCLE"
