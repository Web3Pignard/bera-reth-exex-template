# Bera-Reth ExEx Template

Template for building indexers, trading bots, real-time analytics and more on Berachain using [Execution Extensions (ExEx)](https://reth.rs/exex/overview/).


## Quick Start

```bash
# 1. Clone template and beacon-kit dependency
git clone https://github.com/your-org/bera-reth-exex-template.git
cd bera-reth-exex-template
git clone https://github.com/berachain/beacon-kit.git ../beacon-kit

# 2. Run with BeaconKit integration
make start-local  # Runs indefinitely, Ctrl+C to stop
```

This starts both BeaconKit and Bera-Reth with the ExEx, logging PoL transactions in real-time.

## Production Deployment

For mainnet and testnet deployments, build the production Docker image:

```bash
make docker-build
```

The resulting image can be used as a **drop-in replacement** for the official bera-reth Docker image, as long as the bera-reth version in `Cargo.toml` matches the required version for your network. Simply replace `berachain/bera-reth` with your custom ExEx image in your deployment configuration.

## Development

Edit `src/exex.rs` to customize the exex. Find more information on the [official docs](https://reth.rs/exex/overview/#how-do-i-build-an-execution-extension)


## Available Commands

```bash
make help           # Show all available commands
make start-local    # Start ExEx with BeaconKit (runs indefinitely)
make docker-build   # Build production Docker image (maxperf profile)
make pr             # Run all CI checks (formatting, linting, tests, docs)
make pr-fix         # Auto-fix formatting issues

# Local development
cargo build         # Build debug binary
cargo fmt           # Format code
cargo clippy        # Run linting  
cargo test          # Run tests

# Debug mode
RUST_LOG=debug make start-local
```

## License

Apache-2.0