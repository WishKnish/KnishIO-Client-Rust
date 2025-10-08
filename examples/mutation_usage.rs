//! KnishIO Rust SDK - Mutation Usage Examples
//!
//! This file demonstrates how to use all core mutations in the KnishIO Rust SDK.
//! Each example shows the complete workflow from setup to execution.

use knishio_client::{
    KnishIOClient, GraphQLClient, Wallet, Molecule,
    mutation::{
        MutationProposeMolecule, MutationCreateWallet, MutationCreateToken,
        MutationTransferTokens, MutationRequestTokens, MutationClaimShadowWallet,
        MutationRequestAuthorization, MutationActiveSession,
        CreateTokenParams, TransferTokensParams, RequestTokensParams,
        ClaimShadowWalletParams, RequestAuthorizationParams
    }
};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Example: Basic setup for all mutations
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup clients
    let graphql_client = GraphQLClient::new("https://api.knish.io/graphql");
    let knish_client = KnishIOClient::new(
        vec!["https://api.knish.io".to_string()],
        None, None, None, None, None
    );

    // Run all examples
    example_propose_molecule(&graphql_client, &knish_client).await?;
    example_create_wallet(&graphql_client, &knish_client).await?;
    example_create_token(&graphql_client, &knish_client).await?;
    example_transfer_tokens(&graphql_client, &knish_client).await?;
    example_request_tokens(&graphql_client, &knish_client).await?;
    example_claim_shadow_wallet(&graphql_client, &knish_client).await?;
    example_request_authorization(&graphql_client, &knish_client).await?;
    example_active_session(&graphql_client, &knish_client).await?;

    Ok(())
}

/// Example 1: ProposeMolecule - The core transaction submission
async fn example_propose_molecule(
    graphql_client: &GraphQLClient,
    knish_client: &KnishIOClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== ProposeMolecule Example ===");

    // Create a molecule with some transaction data
    let mut molecule = Molecule::new();
    // Note: In practice, you would populate the molecule with atoms
    // molecule.add_atom(...);
    // molecule.sign(...);
    // molecule.check(...);

    // Create the mutation
    let mutation = MutationProposeMolecule::new(
        graphql_client.clone(),
        knish_client.clone(),
        molecule
    );

    // Execute the mutation
    let response = mutation.execute(graphql_client, None, None).await?;
    println!("ProposeMolecule response: {:?}", response);

    Ok(())
}

/// Example 2: CreateWallet - Creating new wallets
async fn example_create_wallet(
    graphql_client: &GraphQLClient,
    knish_client: &KnishIOClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== CreateWallet Example ===");

    // Create a wallet to be created on the ledger
    let wallet = Wallet::create(
        Some("test-secret-12345"),
        None,
        "TEST",
        None,
        None
    )?;

    // Create a molecule for the wallet creation
    let molecule = Molecule::with_params(
        Some("test-secret-12345".to_string()),
        None,
        None,
        None,
        None,
        None
    );

    // Create the mutation
    let mut mutation = MutationCreateWallet::new(
        graphql_client.clone(),
        knish_client.clone(),
        molecule
    );

    // Fill the molecule with wallet creation data
    // Note: This will fail until molecule methods are implemented
    match mutation.fill_molecule(&wallet) {
        Ok(_) => {
            let response = mutation.execute(graphql_client, None, None).await?;
            println!("CreateWallet response: {:?}", response);
        }
        Err(e) => {
            println!("CreateWallet failed (expected): {:?}", e);
            println!("This is expected until molecule initialization methods are implemented");
        }
    }

    Ok(())
}

/// Example 3: CreateToken - Creating new tokens
async fn example_create_token(
    graphql_client: &GraphQLClient,
    knish_client: &KnishIOClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== CreateToken Example ===");

    // Create recipient wallet
    let recipient_wallet = Wallet::create(
        Some("recipient-secret-12345"),
        None,
        "TEST",
        None,
        None
    )?;

    // Prepare metadata
    let mut meta = HashMap::new();
    meta.insert("name".to_string(), json!("Test Token"));
    meta.insert("symbol".to_string(), json!("TST"));
    meta.insert("decimals".to_string(), json!(8));
    meta.insert("totalSupply".to_string(), json!(1000000));

    // Create parameters
    let params = CreateTokenParams {
        recipient_wallet: recipient_wallet.clone(),
        amount: 1000.0,
        meta: Some(meta),
    };

    // Create a molecule for token creation
    let molecule = Molecule::with_params(
        Some("creator-secret-12345".to_string()),
        None,
        None,
        None,
        None,
        None
    );

    // Create the mutation
    let mut mutation = MutationCreateToken::new(
        graphql_client.clone(),
        knish_client.clone(),
        molecule
    );

    // Fill the molecule with token creation data
    match mutation.fill_molecule(params) {
        Ok(_) => {
            let response = mutation.execute(graphql_client, None, None).await?;
            println!("CreateToken response: {:?}", response);
        }
        Err(e) => {
            println!("CreateToken failed (expected): {:?}", e);
            println!("This is expected until molecule initialization methods are implemented");
        }
    }

    Ok(())
}

/// Example 4: TransferTokens - Moving tokens between wallets
async fn example_transfer_tokens(
    graphql_client: &GraphQLClient,
    knish_client: &KnishIOClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== TransferTokens Example ===");

    // Create source wallet
    let source_wallet = Wallet::create(
        Some("source-secret-12345"),
        None,
        "TEST",
        None,
        None
    )?;

    // Create recipient wallet
    let recipient_wallet = Wallet::create(
        Some("recipient-secret-12345"),
        None,
        "TEST",
        None,
        None
    )?;

    // Create parameters
    let params = TransferTokensParams {
        recipient_wallet: recipient_wallet.clone(),
        amount: 100.0,
    };

    // Create a molecule for the transfer
    let molecule = Molecule::with_params(
        Some("source-secret-12345".to_string()),
        None,
        Some(source_wallet.clone()),
        None,
        None,
        None
    );

    // Create the mutation
    let mut mutation = MutationTransferTokens::new(
        graphql_client.clone(),
        knish_client.clone(),
        molecule
    );

    // Fill the molecule with transfer data
    match mutation.fill_molecule(params) {
        Ok(_) => {
            let response = mutation.execute(graphql_client, None, None).await?;
            println!("TransferTokens response: {:?}", response);
        }
        Err(e) => {
            println!("TransferTokens failed (expected): {:?}", e);
            println!("This is expected until molecule initialization methods are implemented");
        }
    }

    Ok(())
}

/// Example 5: RequestTokens - Requesting tokens from the network
async fn example_request_tokens(
    graphql_client: &GraphQLClient,
    knish_client: &KnishIOClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== RequestTokens Example ===");

    // Prepare metadata for the request
    let mut meta = HashMap::new();
    meta.insert("reason".to_string(), json!("Testing token request"));
    meta.insert("requestedBy".to_string(), json!("test-user"));

    // Create parameters
    let params = RequestTokensParams {
        token: "TEST".to_string(),
        amount: 500.0,
        meta_type: "tokenRequest".to_string(),
        meta_id: "req-123".to_string(),
        meta: Some(meta),
        batch_id: Some("batch-456".to_string()),
    };

    // Create a molecule for the request
    let molecule = Molecule::with_params(
        Some("requester-secret-12345".to_string()),
        None,
        None,
        None,
        None,
        None
    );

    // Create the mutation
    let mut mutation = MutationRequestTokens::new(
        graphql_client.clone(),
        knish_client.clone(),
        molecule
    );

    // Fill the molecule with request data
    match mutation.fill_molecule(params) {
        Ok(_) => {
            let response = mutation.execute(graphql_client, None, None).await?;
            println!("RequestTokens response: {:?}", response);
        }
        Err(e) => {
            println!("RequestTokens failed (expected): {:?}", e);
            println!("This is expected until molecule initialization methods are implemented");
        }
    }

    Ok(())
}

/// Example 6: ClaimShadowWallet - Claiming shadow wallets
async fn example_claim_shadow_wallet(
    graphql_client: &GraphQLClient,
    knish_client: &KnishIOClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== ClaimShadowWallet Example ===");

    // Create parameters
    let params = ClaimShadowWalletParams {
        token: "TEST".to_string(),
        batch_id: Some("shadow-batch-789".to_string()),
    };

    // Create a molecule for the claim
    let molecule = Molecule::with_params(
        Some("claimer-secret-12345".to_string()),
        None,
        None,
        None,
        None,
        None
    );

    // Create the mutation
    let mut mutation = MutationClaimShadowWallet::new(
        graphql_client.clone(),
        knish_client.clone(),
        molecule
    );

    // Fill the molecule with claim data
    match mutation.fill_molecule(params) {
        Ok(_) => {
            let response = mutation.execute(graphql_client, None, None).await?;
            println!("ClaimShadowWallet response: {:?}", response);
        }
        Err(e) => {
            println!("ClaimShadowWallet failed (expected): {:?}", e);
            println!("This is expected until molecule initialization methods are implemented");
        }
    }

    Ok(())
}

/// Example 7: RequestAuthorization - Requesting authorization tokens
async fn example_request_authorization(
    graphql_client: &GraphQLClient,
    knish_client: &KnishIOClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== RequestAuthorization Example ===");

    // Prepare metadata for the authorization request
    let mut meta = HashMap::new();
    meta.insert("purpose".to_string(), json!("API access"));
    meta.insert("permissions".to_string(), json!(["read", "write"]));
    meta.insert("duration".to_string(), json!("1h"));

    // Create parameters
    let params = RequestAuthorizationParams {
        meta,
    };

    // Create a molecule for the authorization request
    let molecule = Molecule::with_params(
        Some("user-secret-12345".to_string()),
        None,
        None,
        None,
        None,
        None
    );

    // Create the mutation
    let mut mutation = MutationRequestAuthorization::new(
        graphql_client.clone(),
        knish_client.clone(),
        molecule
    );

    // Fill the molecule with authorization data
    match mutation.fill_molecule(params) {
        Ok(_) => {
            let response = mutation.execute(graphql_client, None, None).await?;
            println!("RequestAuthorization response: {:?}", response);
        }
        Err(e) => {
            println!("RequestAuthorization failed (expected): {:?}", e);
            println!("This is expected until molecule initialization methods are implemented");
        }
    }

    Ok(())
}

/// Example 8: ActiveSession - Managing active user sessions
async fn example_active_session(
    graphql_client: &GraphQLClient,
    _knish_client: &KnishIOClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== ActiveSession Example ===");

    // Create the mutation (doesn't need molecule)
    let mutation = MutationActiveSession::new();

    // Prepare variables for the session
    let variables = json!({
        "bundleHash": "user-bundle-hash-12345",
        "metaType": "userSession",
        "metaId": "session-abc-123",
        "ipAddress": "192.168.1.100",
        "browser": "Mozilla/5.0 (compatible; Rust-Client/1.0)",
        "osCpu": "Linux x86_64",
        "resolution": "1920x1080",
        "timeZone": "UTC",
        "json": json!({
            "sessionData": "additional session information",
            "userAgent": "KnishIO-Rust-SDK/1.0"
        }).to_string()
    });

    // Execute the mutation
    let response = mutation.execute(graphql_client, Some(variables), None).await?;
    println!("ActiveSession response: {:?}", response);

    Ok(())
}

/// Example: Using the mutation builder pattern
#[allow(dead_code)]
async fn example_builder_pattern(
    graphql_client: &GraphQLClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Mutation Builder Pattern Example ===");

    use knishio_client::mutation::MutationBuilder;

    // Create a builder
    let builder = MutationBuilder::new()
        .with_secret("user-secret-12345")
        .with_bundle("user-bundle-hash-12345")
        .with_client(graphql_client.clone());

    // Create a molecule
    let molecule = Molecule::new();

    // Use builder to create mutation
    let mutation = builder.propose_molecule(molecule)?;
    println!("Created ProposeMolecule using builder pattern");

    // You can also use the builder for other mutations
    let builder = MutationBuilder::new()
        .with_secret("user-secret-12345")
        .with_client(graphql_client.clone());

    let active_session = builder.active_session();
    println!("Created ActiveSession using builder pattern: {:?}", active_session);

    Ok(())
}

/// Example: Using helper functions for common operations
#[allow(dead_code)]
async fn example_helper_functions() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Helper Functions Example ===");

    use knishio_client::mutation::helpers;

    // Create wallets
    let source_wallet = Wallet::create(
        Some("source-secret"),
        None,
        "TEST",
        None,
        None
    )?;

    let recipient_wallet = Wallet::create(
        Some("recipient-secret"),
        None,
        "TEST",
        None,
        None
    )?;

    // Use helper to create value transfer
    match helpers::create_value_transfer(
        "source-secret",
        &source_wallet,
        &recipient_wallet,
        100.0,
    ) {
        Ok(mutation) => {
            println!("Created value transfer mutation using helper");
            println!("Mutation type: TransferTokens, Amount: 100.0");
        }
        Err(e) => {
            println!("Helper function failed (expected): {:?}", e);
            println!("This is expected until molecule methods are implemented");
        }
    }

    // Use helper to create wallet creation
    match helpers::create_wallet_creation("user-secret", &source_wallet) {
        Ok(mutation) => {
            println!("Created wallet creation mutation using helper");
        }
        Err(e) => {
            println!("Helper function failed (expected): {:?}", e);
        }
    }

    // Use helper to create token creation
    let mut metadata = HashMap::new();
    metadata.insert("name".to_string(), json!("Helper Token"));

    match helpers::create_token_creation(
        "creator-secret",
        &recipient_wallet,
        1000.0,
        Some(metadata),
    ) {
        Ok(mutation) => {
            println!("Created token creation mutation using helper");
        }
        Err(e) => {
            println!("Helper function failed (expected): {:?}", e);
        }
    }

    Ok(())
}

/// Example: Error handling patterns
#[allow(dead_code)]
async fn example_error_handling(
    graphql_client: &GraphQLClient,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Error Handling Example ===");

    // Create a mutation that might fail
    let mutation = MutationActiveSession::new();

    // Example with invalid variables
    let invalid_variables = json!({
        // Missing required bundleHash
        "metaType": "session",
        "metaId": "test"
    });

    match mutation.execute(graphql_client, Some(invalid_variables), None).await {
        Ok(response) => {
            println!("Unexpected success: {:?}", response);
        }
        Err(e) => {
            println!("Expected error: {:?}", e);
            
            // Handle different error types
            match e {
                knishio_client::KnishIOError::GraphQL(graphql_errors) => {
                    println!("GraphQL errors: {:?}", graphql_errors);
                }
                knishio_client::KnishIOError::Network(network_error) => {
                    println!("Network error: {:?}", network_error);
                }
                knishio_client::KnishIOError::Validation(validation_error) => {
                    println!("Validation error: {:?}", validation_error);
                }
                _ => {
                    println!("Other error type");
                }
            }
        }
    }

    Ok(())
}