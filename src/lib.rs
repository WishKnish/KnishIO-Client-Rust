//! KnishIO Rust SDK
//!
//! A comprehensive Rust implementation of the KnishIO SDK for post-blockchain
//! distributed ledger technology. This SDK provides complete compatibility with
//! the JavaScript reference implementation while leveraging Rust's performance
//! and memory safety.
//!
//! # Features
//!
//! - **Post-Quantum Cryptography**: ML-KEM key encapsulation and WOTS+ one-time signatures
//! - **SHAKE256 Hashing**: Quantum-resistant cryptographic hashing
//! - **Molecular Transactions**: Atomic operations grouped into molecules
//! - **GraphQL Integration**: Seamless communication with KnishIO nodes
//! - **Wallet Management**: Deterministic wallet generation and management
//! - **Cross-Platform Compatibility**: 100% compatible with other SDK implementations
//!
//! # Quick Start
//!
//! ```rust
//! use knishio_client::{Wallet, Molecule};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a wallet
//!     let wallet = Wallet::create(
//!         Some("your-secret-here"),
//!         None,
//!         "KNISH",
//!         None,
//!         None,
//!     )?;
//!
//!     // Create and sign a molecule
//!     let mut molecule = Molecule::new(
//!         Some("your-secret-here".to_string()),
//!         None,
//!         Some(wallet),
//!         None,
//!         None,
//!         None,
//!     );
//!
//!     // ... build your transaction ...
//!
//!     Ok(())
//! }
//! ```

/// SDK Version constant
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// Core modules
pub mod atom;
pub mod crypto;
pub mod error;
pub mod molecule;
pub mod types;
pub mod wallet;

// GraphQL communication modules
pub mod graphql;
pub mod query;
pub mod mutation;
pub mod response;

// Client module
pub mod client;

// Additional modules
pub mod auth;
pub mod subscribe;
pub mod meta;
pub mod rules;
pub mod versions;
pub mod token_unit;
pub mod policy_meta;

// Utility modules
pub mod utils;

// Validation modules
pub mod check_molecule;

// Re-exports for convenience
pub use atom::Atom;
pub use error::{KnishIOError, Result};
pub use molecule::{Molecule, TypeSafeMoleculeBuilder, ValueAtomParams, MetaAtomParams, IdentityAtomParams, TokenRequestAtomParams, BufferDepositAtomParams, BufferWithdrawAtomParams, FusionAtomParams, StackableTransferParams};
pub use types::{Isotope, MetaItem};
pub use wallet::Wallet;
pub use client::{KnishIOClient, builder::ClientBuilder};
pub use check_molecule::CheckMolecule;
pub use token_unit::TokenUnit;
pub use policy_meta::PolicyMeta;

// Rules system re-exports
pub use rules::{Rule, Callback, Condition};

// Version utilities re-exports
pub use versions::{HashAtom, Version4, AtomVersion, StructureUtils};

// GraphQL re-exports - Production-Ready Client
pub use graphql::{
    GraphQLClient, GraphQLRequest, GraphQLResponse, GraphQLError, ErrorLocation,
    SocketConfig, GraphQLConnectionStats, RetryPolicy, RetryStrategy, RetryCondition,
    RetryExecutor, ClientConfig, ConnectionPoolConfig, PoolStats, WebSocketManager, ConnectionState,
    WebSocketReconnectConfig, global_pool, execute_with_retry,
    create_query_request, create_mutation_request, create_subscription_request
};
pub use query::{Query, BaseQuery};
pub use mutation::{Mutation, BaseMutation};
pub use response::{Response, BaseResponse};

/// Cryptographic operations module
///
/// Provides all cryptographic primitives used by the KnishIO SDK including
/// SHAKE256 hashing, secret generation, and bundle hash computation.
pub use crypto::{generate_bundle_hash, generate_secret, generate_batch_id, shake256};

/// Molecule transaction builder utilities
///
/// Helper functions for constructing common molecular transaction patterns.
pub mod builders {
    pub use crate::molecule::Molecule;
    pub use crate::atom::{Atom, AtomCreateParams};
    pub use crate::types::{Isotope, MetaItem};
}

/// Utilities for cross-SDK compatibility testing
///
/// Functions and types used for validation and compatibility testing
/// across different SDK implementations.
pub mod validation {
    pub use crate::atom::Atom;
    pub use crate::molecule::Molecule;
    pub use crate::crypto::{shake256, generate_secret};
    pub use crate::error::{KnishIOError, Result};
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_sdk_version() {
        assert!(!VERSION.is_empty());
        // Version should follow semantic versioning
        assert!(VERSION.contains('.'));
    }

    #[test]
    fn test_basic_workflow() {
        // Test that we can create basic SDK objects
        let wallet = Wallet::create(
            Some("test-secret-12345"),
            None,
            "TEST",
            None,
            None,
        );
        
        assert!(wallet.is_ok());
        
        let wallet = wallet.unwrap();
        assert_eq!(wallet.token, "TEST");
        assert!(wallet.bundle.is_some());
        assert!(wallet.address.is_some());
    }

    #[test]
    fn test_molecular_transaction() {
        // Test that we can create a basic molecule
        let wallet = Wallet::create(
            Some("test-secret-12345"),
            None,
            "TEST",
            None,
            None,
        ).unwrap();

        let molecule = Molecule::with_params(
            Some("test-secret-12345".to_string()),
            None,
            Some(wallet),
            None,
            None,
            None,
        );

        assert_eq!(molecule.atoms.len(), 0);
        assert!(molecule.molecular_hash.is_none());
        assert!(molecule.secret.is_some());
    }

    #[test]
    fn test_cryptographic_operations() {
        // Test SHAKE256 hashing
        let hash = shake256("test", 256);
        assert_eq!(hash.len(), 64); // 256 bits = 64 hex characters

        // Test secret generation
        let secret = generate_secret("test-seed");
        assert_eq!(secret.len(), 2048); // Should be 2048 characters
        
        // Test batch ID generation
        let batch_id = generate_batch_id();
        assert_eq!(batch_id.len(), 64); // Should be 64 hex characters
    }

    #[test]
    fn test_atom_creation() {
        let atom = Atom::new("W1", "test-address", Isotope::V, "TEST");
        
        assert_eq!(atom.position, "W1");
        assert_eq!(atom.wallet_address, "test-address");
        assert_eq!(atom.isotope, Isotope::V);
        assert_eq!(atom.token, "TEST");
        assert!(atom.value.is_none());
        assert!(atom.ots_fragment.is_none());
    }
    
    #[test]
    fn test_client_creation() {
        let client = KnishIOClient::new(
            "http://localhost:8080",
            None,
            None,
            None,
            Some(3),
            Some(false),
        );
        
        assert_eq!(client.get_server_sdk_version(), 3);
        assert!(!client.has_secret());
        assert!(!client.has_bundle());
    }
}