#!/bin/bash

# CircleFi Contract Deployment Script
# Targets Stellar Testnet

set -e

NETWORK="testnet"
RPC_URL="https://soroban-testnet.stellar.org"

echo "🚀 CircleFi Contract Deployment"
echo "================================"
echo "Network: $NETWORK"
echo "RPC URL: $RPC_URL"
echo ""

# Build contract
echo "📦 Building contract..."
cargo build --target wasm32-unknown-unknown --release

# Get contract binary
CONTRACT_BINARY="target/wasm32-unknown-unknown/release/circlefi_contract.wasm"

if [ ! -f "$CONTRACT_BINARY" ]; then
    echo "❌ Contract build failed"
    exit 1
fi

echo "✅ Contract built successfully"
echo ""

# Deploy instructions
echo "📝 Deployment Instructions:"
echo "1. Install Soroban CLI: npm install -g stellar-cli"
echo "2. Configure network: soroban config network add testnet --rpc-url $RPC_URL"
echo "3. Deploy contract: soroban contract deploy --network testnet --source <source-account> $CONTRACT_BINARY"
echo ""
echo "4. Save the contract ID for your app configuration"
echo ""

# Generate WASM hash for reference
echo "📊 Contract Hash:"
shasum -a 256 "$CONTRACT_BINARY"
