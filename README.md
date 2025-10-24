# Pulstream Token Plugin

A high-performance Solana blockchain data processing tool built on Jetstreamer for tracking and analyzing PumpFun token trades in real-time.

## Overview

Pulstream Token Plugin provides a plugin-based framework for processing Solana blockchain data with a focus on tracking PumpFun token trades. It leverages Jetstreamer's efficient data streaming capabilities to process historical blockchain data by epoch or slot range.

## Features

- 🚀 **High-Performance Processing**: Multi-threaded blockchain data processing with Jetstreamer
- 🎯 **Token-Specific Tracking**: Filter and track trades for specific token mint addresses
- 🔌 **Plugin Architecture**: Extensible plugin system for custom data processing logic
- 📊 **Trade Event Detection**: Automatically decode and process PumpFun trade events
- ⚡ **Flexible Range Queries**: Process data by epoch or custom slot ranges
- 🛠️ **Custom Event Handlers**: Implement custom processors for trade events

## Prerequisites

- Rust 1.70 or higher
- Solana blockchain data accessible through Jetstreamer

## Installation

Clone the repository and build the project:

```bash
git clone <repository-url>
cd pulstream-token-plugin
cargo build --release
```

## Usage

### Basic Usage

Process an entire epoch:

```bash
cargo run -- <epoch_number>
```

Process a specific slot range:

```bash
cargo run -- <start_slot>:<end_slot>
```

### Track a Specific Token Mint

Track trades for a specific token mint address:

```bash
cargo run -- --mint <MINT_ADDRESS> <epoch_number>
```

Or using the short flag:

```bash
cargo run -- -m <MINT_ADDRESS> <epoch_number>
```

### Configuration

#### Environment Variables

- `PULSTREAM_MINT`: Set the token mint address to track
- `JETSTREAMER_THREADS`: Number of processing threads (default: 1)

Example:

```bash
export JETSTREAMER_THREADS=4
export PULSTREAM_MINT=<MINT_ADDRESS>
cargo run -- 500
```

## Architecture

The project is organized as a Cargo workspace with two main components:

### 1. Main Binary (`src/main.rs`)

The main application that:

- Parses command-line arguments
- Configures the Jetstreamer runner
- Initializes and registers plugins
- Processes blockchain data

### 2. Plugin Library (`pulstream-plugin/`)

A support library containing:

#### Plugins (`pulstream-plugin/src/plugins/`)

- **PumpfunTrackingPlugin**: Tracks and processes PumpFun token trades
  - Filters transactions by mint address
  - Decodes PumpFun instructions
  - Emits structured trade events

#### Utilities (`pulstream-plugin/src/utils/`)

- **instruction.rs**: Transaction and instruction metadata extraction
- **transformers.rs**: Data transformation utilities

## Trade Event Structure

When a trade is detected, the following information is captured:

```rust
TradeEvent {
    slot: u64,              // Slot number
    signature: String,      // Transaction signature
    timestamp: i64,         // Trade timestamp
    program_id: String,     // Program ID
    mint: String,           // Token mint address
    payer: String,          // Payer/user address
    amount_in: u64,         // Input amount
    amount_out: u64,        // Output amount
    is_buy: bool,           // Whether it's a buy or sell
}
```

## Custom Plugin Development

Create custom plugins by implementing the `Plugin` trait from Jetstreamer:

```rust
use jetstreamer::plugin::{Plugin, PluginFuture};
use futures_util::future::FutureExt;

#[derive(Clone)]
pub struct MyCustomPlugin {
    // Your plugin fields
}

impl Plugin for MyCustomPlugin {
    fn name(&self) -> &'static str {
        "My Custom Plugin"
    }

    fn on_transaction<'a>(
        &'a self,
        _thread_id: usize,
        _db: Option<Arc<Client>>,
        transaction: &'a TransactionData,
    ) -> PluginFuture<'a> {
        async move {
            // Your processing logic here
            Ok(())
        }
        .boxed()
    }

    // Implement other lifecycle methods...
}
```

## Dependencies

### Core Dependencies

- **jetstreamer**: Blockchain data streaming framework
- **carbon-core**: Solana instruction decoding framework
- **carbon-pumpfun-decoder**: PumpFun-specific instruction decoder
- **solana-\***: Solana SDK libraries for transaction and account handling

### Supporting Libraries

- **tokio**: Async runtime
- **serde/serde_json**: Serialization
- **log**: Logging infrastructure
- **clickhouse**: ClickHouse database client (optional)

## Examples

### Example 1: Track a Token for Epoch 500

```bash
cargo run -- --mint 9BB6NFEcjBCtnNLFko2FqVQBq8HHM13kCyYcdQbgpump 500
```

### Example 2: Process a Slot Range with Multiple Threads

```bash
export JETSTREAMER_THREADS=8
cargo run -- 250000000:250001000
```

### Example 3: Custom Event Processing

```rust
let mint_pubkey = mint.parse::<Pubkey>()?;
let plugin = PumpfunTrackingPlugin::with_processor(
    mint_pubkey,
    Arc::new(|trade_event: &TradeEvent| {
        // Custom processing logic
        println!("Trade detected: {} tokens at slot {}",
                 trade_event.amount_out,
                 trade_event.slot);
    }),
);
```

## Project Structure

```
pulstream-token-plugin/
├── src/
│   └── main.rs              # Main application entry point
├── pulstream-plugin/
│   ├── src/
│   │   ├── lib.rs          # Library root
│   │   ├── plugins.rs      # Plugin module exports
│   │   ├── plugins/
│   │   │   └── pumpfun_tracking.rs  # PumpFun tracking plugin
│   │   └── utils/
│   │       ├── instruction.rs       # Instruction utilities
│   │       ├── transformers.rs      # Data transformers
│   │       └── mod.rs
│   └── Cargo.toml
├── Cargo.toml              # Workspace configuration
└── README.md
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

[Add your license information here]

## Support

For issues and questions, please open an issue on the GitHub repository.
