# Reed-Solomon Codec

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A high-performance Reed-Solomon erasure coding implementation in Rust, providing efficient data encoding and recovery capabilities for distributed systems and storage applications.

## Features

- Configurable data and parity shard counts
- Efficient encoding and decoding operations
- Built-in data validation and error handling
- Support for large data blocks (up to 4GB)
- Zero-copy operations where possible
- Comprehensive test coverage

## Building

1. Clone the repository:
```bash
git clone https://github.com/hambosto/reed-solomon-codec
cd reed-solomon-codec
```

2. Build the project:
```bash
cargo build --release
```

3. Run tests:
```bash
cargo test
```

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
reed-solomon-codec = { git = "https://github.com/hambosto/reed-solomon-codec" }
```

Basic usage example:

```rust
use reed_solomon_codec::ReedSolomonCodec;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new codec with 10 data shards and 4 parity shards
    let codec = ReedSolomonCodec::new(10, 4)?;
    
    // Your original data
    let data = b"Hello, World!".to_vec();
    
    // Encode the data
    let encoded = codec.encode(&data)?;
    
    // Decode the data
    let decoded = codec.decode(&encoded)?;
    
    assert_eq!(data, decoded);
    Ok(())
}
```

## Usage

### Creating a Codec

```rust
let codec = ReedSolomonCodec::new(data_shards, parity_shards)?;
```

The codec requires two parameters:
- `data_shards`: Number of data shards (1-256)
- `parity_shards`: Number of parity shards (1-256)

Total shards (data + parity) must not exceed 256.

### Encoding Data

```rust
let encoded = codec.encode(&original_data)?;
```

### Decoding Data

```rust
let decoded = codec.decode(&encoded_data)?;
```

## Configuration Limits

- Shard count: 1-256 shards
- Data size: 1 byte to 4GB
- Total shards: Maximum 256 (data + parity)

## Error Handling

The library provides detailed error types for various failure scenarios:

- `InvalidShardCount`: When shard configuration is invalid
- `InvalidDataSize`: When input data size is out of bounds
- `CodecError`: For general codec initialization errors
- `EncodingError`: For encoding operation failures
- `DecodingError`: For decoding operation failures

## Performance Considerations

- The codec uses efficient algorithms for encoding and decoding
- Memory allocation is minimized through careful buffer management
- Large data blocks are processed in chunks for better memory usage

## Development

### Prerequisites

- Rust 1.67 or higher
- Cargo (included with Rust)

### Setting Up Development Environment

1. Install Rust if you haven't already:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Clone and build:
```bash
git clone https://github.com/yourusername/reed-solomon-codec
cd reed-solomon-codec
cargo build
```

### Running Tests

Run the test suite:
```bash
cargo test
```

Run tests with detailed output:
```bash
cargo test -- --nocapture
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

Please make sure to update tests as appropriate.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Based on the [reed-solomon-erasure](https://github.com/rust-rse/reed-solomon-erasure) library
- Inspired by best practices in erasure coding implementations