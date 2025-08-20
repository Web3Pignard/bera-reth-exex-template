#!/bin/bash
# Start BeaconKit and ExEx for local development
set -e

BEACON_KIT="${BEACON_KIT:-../beacon-kit}"
STARTUP_TIMEOUT="${STARTUP_TIMEOUT:-20}"

echo "=== Bera-Reth ExEx Template - Local Development ==="
echo "BeaconKit: $BEACON_KIT"

# Check BeaconKit dependency
if [ ! -d "$BEACON_KIT" ]; then
    echo "ERROR: BeaconKit not found at $BEACON_KIT"
    echo "Run: git clone https://github.com/berachain/beacon-kit.git $BEACON_KIT"
    exit 1
fi

# Build ExEx in debug mode for development
echo "Building ExEx (debug mode)..."
cargo build

# Check if ports are available (only listening processes)
echo "Checking port availability..."
for port in 8545 8551 3500; do
    if lsof -i :$port | grep LISTEN >/dev/null 2>&1; then
        echo "ERROR: Port $port is in use by a listening process"
        echo "Please stop any processes using these ports and try again"
        lsof -i :$port | grep LISTEN
        exit 1
    fi
done

# Cleanup function
cleanup() {
    echo ""
    echo "Cleaning up processes..."
    [ -n "$BEACON_PID" ] && kill $BEACON_PID 2>/dev/null || true
    [ -n "$EXEX_PID" ] && kill $EXEX_PID 2>/dev/null || true
    pkill -f "beacond\|bera-reth-exex" 2>/dev/null || true
    jobs -p | xargs -r kill 2>/dev/null || true
    echo "Cleanup completed"
}
trap cleanup EXIT INT TERM

# Function to get current block number
get_block_number() {
    result=$(curl -s -X POST -H "Content-Type: application/json" \
         --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
         http://localhost:8545 2>/dev/null | \
    grep -o '"result":"[^"]*"' | cut -d'"' -f4 2>/dev/null)
    if [ -n "$result" ]; then
        printf "%d\n" "$result" 2>/dev/null || echo "0"
    else
        echo "0"
    fi
}

# Clean previous state before starting
echo "Cleaning previous state..."
rm -rf ~/.bera-reth "$BEACON_KIT/.tmp" 2>/dev/null || true

# Start BeaconKit
echo "Starting BeaconKit..."
cd "$BEACON_KIT"
bash -c 'echo "y" | make start' 2>&1 | sed 's/^/[BEACONKIT] /' &
BEACON_PID=$!

# Wait for BeaconKit to initialize and create genesis file
echo "Waiting for BeaconKit to create genesis file..."
genesis_file="$BEACON_KIT/.tmp/beacond/eth-genesis.json"
start_time=$(date +%s)

while [ ! -f "$genesis_file" ]; do
    if [ $(($(date +%s) - start_time)) -ge $STARTUP_TIMEOUT ]; then
        echo "ERROR: Genesis file not created after ${STARTUP_TIMEOUT}s"
        echo "BeaconKit may not have started properly"
        exit 1
    fi
    echo "Waiting for genesis file... ($(($(date +%s) - start_time))/${STARTUP_TIMEOUT}s)"
    sleep 2
done

echo "✅ Genesis file created at: $genesis_file"

# Start ExEx node directly
echo "Starting ExEx node..."
cd - >/dev/null # Return to ExEx directory quietly
./target/debug/bera-reth node \
    --chain "$BEACON_KIT/.tmp/beacond/eth-genesis.json" \
    --http \
    --http.addr "0.0.0.0" \
    --http.port 8545 \
    --http.api eth,net \
    --authrpc.addr "0.0.0.0" \
    --authrpc.jwtsecret "$BEACON_KIT/testing/files/jwt.hex" \
    --datadir "$BEACON_KIT/.tmp/beacond/eth-home" \
    --ipcpath "$BEACON_KIT/.tmp/beacond/eth-home/eth-engine.ipc" \
    --engine.persistence-threshold 0 \
    --engine.memory-block-buffer-target 0 \
    2>&1 | sed 's/^/[EXEX] /' &
EXEX_PID=$!

echo ""
echo "✅ Both BeaconKit and ExEx are running!"
echo "Monitor the logs above for PoL transaction processing."
echo "Press Ctrl+C to stop both processes."
echo ""

# Wait for both processes
wait