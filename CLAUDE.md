# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust application that demonstrates Ethereum blockchain interaction using the Alloy framework. The application connects to Ethereum via WebSocket, queries a custom oracle contract using Multicall for efficient batched RPC calls, and integrates with SigNoz for distributed tracing.

## Key Architecture Components

### Core Dependencies
- **Alloy Framework**: Modern Ethereum library for Rust with full feature set
  - `alloy-provider`: RPC provider functionality
  - `alloy-contract`: Smart contract interaction
  - `alloy-transport-ws`: WebSocket transport layer
  - `alloy-sol-types`: Solidity type generation via `sol!` macro
- **Tokio**: Async runtime with full feature set
- **OpenTelemetry**: Distributed tracing with SigNoz backend integration
- **Tonic**: gRPC client for telemetry export

### Application Structure
- **Single-file application**: All logic contained in `src/main.rs`
- **Contract Interface**: Uses `sol!` macro to generate Rust bindings from Solidity interface
- **Multicall Pattern**: Batches multiple contract calls into single RPC request
- **Telemetry Integration**: OpenTelemetry spans with SigNoz exporter

## Development Commands

### Build and Run
```bash
# Build the project
cargo build

# Run the application
cargo run

# Build in release mode
cargo build --release

# Run with release optimizations
cargo run --release
```

### Development Tools
```bash
# Check code without building
cargo check

# Run linter
cargo clippy

# Format code
cargo fmt

# Run tests (if any exist)
cargo test
```

## Environment Configuration

The application requires these environment variables for SigNoz:
- `SIGNOZ_ENDPOINT`: SigNoz collector endpoint URL (e.g., https://otelcollector.b100pro.com)
- `SIGNOZ_API_KEY`: API key for secured SigNoz instances (optional, only needed for protected instances)
- `APP_NAME`: Application name for tracing service identification (optional, defaults to "chainlink_multicall_signoz")

Create a `.env` file in the root directory with these variables. The code automatically detects if authentication is needed based on the presence of `SIGNOZ_API_KEY`.

## Contract Interface

The application interacts with a CustomOracle contract at address `0x6CAFE228eC0B0bC2D076577d56D35Fe704318f6d`. The contract interface includes:
- Price feeds: `BASE_FEED_1()`, `BASE_FEED_2()`, `QUOTE_FEED_1()`, `QUOTE_FEED_2()`
- Configuration: `SCALE_FACTOR()`, `VAULT()`, `VAULT_CONVERSION_SAMPLE()`
- Main function: `price()` - returns current price

## Blockchain Configuration

- **RPC Endpoint**: `wss://ethereum-rpc.publicnode.com` (WebSocket)
- **Network**: Ethereum Mainnet
- **Transport**: WebSocket for real-time connection

## Telemetry Setup

The application uses OpenTelemetry with SigNoz backend:
- Tracer initialization in `init_tracer()` function
- OTLP exporter with gRPC transport
- Optional metadata-based authentication for secured instances
- Tokio runtime integration for async spans
- Automatic authentication detection based on environment variables

## Key Patterns

### Multicall Usage
The application demonstrates efficient blockchain querying by:
1. Creating individual contract call builders
2. Adding them to a multicall builder
3. Executing all calls in a single RPC request
4. Destructuring results in call order

### Error Handling
- Uses `eyre` crate for error handling
- Propagates errors with `?` operator
- Wraps main function return in `eyre::Result`

## Language Notes

- Comments and println! statements are in Russian
- Variable names follow Rust conventions in English
- Code structure follows standard Rust patterns despite comment language
