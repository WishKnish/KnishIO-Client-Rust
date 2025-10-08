# KnishIO Client SDK for Rust

A comprehensive Rust implementation of the Knish.IO SDK for post-blockchain distributed ledger technology. This SDK provides complete compatibility with JavaScript, Kotlin, PHP, and Python implementations while leveraging Rust's performance and safety features.

## Features

- **Post-Quantum Cryptography**: ML-KEM768 and XMSS signatures
- **DAG Architecture**: Directed acyclic graph transaction processing
- **Molecular Transactions**: Atom/Molecule composition patterns
- **Cross-SDK Compatibility**: 100% compatible with other SDK implementations
- **One-Time Signatures**: WOTS+ signature algorithm with exact JavaScript compatibility
- **Performance**: High-performance cryptographic operations with SHAKE256

## Core Components

### Molecules
Molecules are the fundamental transaction units containing one or more atomic operations:

```rust
use knishio_client::{Molecule, Wallet, types::MetaItem};

// Create wallets
let mut sender = Wallet::create(Some("secret"), None, "TEST", None, None)?;
sender.balance = 1000.0;
let recipient = Wallet::create(Some("other-secret"), None, "TEST", None, None)?;

// Create and initialize molecule
let mut molecule = Molecule::new(
    Some("secret".to_string()),
    None,
    Some(sender),
    None,
    None,
    None,
);

// Transfer 100 tokens
molecule.init_value(&recipient, 100.0)?;

// Sign with one-time signature
molecule.sign(None, false, true)?;
```

### Atoms
Atoms represent individual operations within molecules:

```rust
use knishio_client::{Atom, Isotope};

let atom = Atom::new("position123", "address456", Isotope::V, "TEST");
molecule.add_atom(atom);
```

### Cryptographic Functions
SHAKE256 hashing compatible with JavaScript implementation:

```rust
use knishio_client::crypto::shake256;

let hash = shake256("test input", 256);
println!("Hash: {}", hash); // Identical to JavaScript output
```

## Molecule Types

The SDK supports all standard molecule types:

- **Value Transfer**: Token transfers between wallets
- **Token Creation**: Create new token types
- **Wallet Creation**: Initialize new wallets
- **Metadata**: Store arbitrary data
- **Authorization**: Permission management
- **Identity**: ContinuID management

```rust
// Metadata molecule
let metadata = vec![
    MetaItem::new("name", "Document"),
    MetaItem::new("type", "pdf"),
];
molecule.init_meta(metadata, "document", "doc123", None)?;

// Token creation
molecule.init_token_creation(&recipient, 1000.0, metadata)?;

// Authorization
let auth_meta = vec![MetaItem::new("permission", "read")];
molecule.init_authorization(auth_meta)?;
```

## One-Time Signatures (WOTS+)

The implementation includes the exact one-time signature algorithm from the JavaScript SDK:

1. **Hash Enumeration**: Convert molecular hash to base-17 representation
2. **Normalization**: Ensure sum equals zero for 50% key leakage
3. **Key Chunking**: Divide 4096-character key into 16 segments
4. **Iterative Hashing**: Apply SHAKE256 based on normalized values
5. **Fragment Distribution**: Distribute signature across atoms

```rust
// Molecular hash enumeration (matches JavaScript exactly)
let hash = "329f873f147f8e50d50e92508236a09e95cc0d154605173f6e5f8e47c11192c5";
let enumerated = Molecule::enumerate(hash);
let normalized = Molecule::normalize(enumerated);
// normalized sum will always be 0
```

## Cross-SDK Compatibility

This implementation ensures 100% compatibility with other SDKs:

- **Identical SHAKE256 output** across all platforms
- **Same molecular hash generation** as JavaScript/Kotlin/PHP
- **Compatible one-time signatures** for cross-platform verification
- **Consistent atom ordering** and indexing
- **Matching JSON serialization** format

## Testing

Run the comprehensive test suite:

```bash
cargo test
```

Run the interactive demo:

```bash
cargo run --example molecule_demo
```

## Performance

The Rust implementation provides significant performance benefits:

- **Memory Safety**: Zero-cost abstractions with compile-time guarantees
- **Cryptographic Speed**: Native performance for SHAKE256 operations
- **Concurrent Processing**: Async/await support for scalable operations
- **Low Overhead**: Minimal runtime cost for signature operations

## Dependencies

- `serde`: JSON serialization/deserialization
- `sha3`: SHAKE256 cryptographic hashing
- `hex`: Hexadecimal encoding/decoding
- `base64`: Base64 encoding for signatures
- `chrono`: Timestamp generation
- `rand`: Random number generation

## Documentation

Full API documentation is available:

```bash
cargo doc --open
```

## License

This project is licensed under the same terms as other KnishIO SDK implementations.

## Contributing

Contributions are welcome! Please ensure:

1. **Compatibility**: All changes must maintain 100% compatibility with JavaScript implementation
2. **Tests**: Add comprehensive tests for new functionality
3. **Performance**: Leverage Rust's performance advantages
4. **Documentation**: Update documentation for API changes

For the complete KnishIO ecosystem documentation, visit the main repository.