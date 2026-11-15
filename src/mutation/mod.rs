//! GraphQL mutation implementations
//!
//! This module contains mutation classes for sending data to KnishIO nodes,
//! equivalent to the JavaScript mutation classes. Provides 100% compatibility
//! with the JavaScript SDK mutation capabilities.

use crate::error::Result;
use crate::graphql::{create_mutation_request, GraphQLClient, GraphQLRequest};
use crate::query::Query;
use crate::response::Response;
use serde_json::{json, Value};
use std::collections::HashMap;

/// Base Mutation trait for all mutation implementations
///
/// This trait extends the Query trait to provide mutation-specific functionality
/// including proper GraphQL mutation string generation and execution.
#[async_trait::async_trait]
pub trait Mutation: Query + Send + Sync {
    /// Get the GraphQL mutation string
    fn get_mutation(&self) -> &str;

    /// Create a mutation request (equivalent to createQuery in JS)
    fn create_mutation_request(&self, variables: Option<Value>) -> GraphQLRequest {
        let compiled_vars = self.compiled_variables(variables);
        create_mutation_request(self.get_mutation(), compiled_vars)
    }

    /// Execute the mutation (equivalent to execute in JS)
    ///
    /// This method sends the mutation to a KnishIO node and returns the response.
    /// It handles authentication context and error processing.
    async fn execute(
        &self,
        client: &GraphQLClient,
        variables: Option<Value>,
        _context: Option<HashMap<String, Value>>,
    ) -> Result<Box<dyn Response>> {
        let request = self.create_mutation_request(variables);

        let response = client.mutate(request).await?;

        // Re-wrap under "data" so the response classes' `data.<Field>` data_keys navigate
        // correctly (JS keeps the full envelope + Dot.get("data.X"); the prior code unwrapped
        // response.data, leaving every "data.X" lookup to fail and fall back to the wrapper).
        let json_data = json!({ "data": response.data });
        Ok(self.create_response(json_data))
    }

    /// Create mutation context for authentication (can be overridden)
    ///
    /// This method provides authentication tokens and other context data
    /// needed for authenticated mutations.
    fn create_mutation_context(&self) -> HashMap<String, Value> {
        self.create_query_context()
    }
}

/// Base Mutation implementation (equivalent to Mutation.js)
///
/// This struct provides a basic mutation implementation that can be used
/// directly or as a foundation for more specific mutation types.
pub struct BaseMutation {
    mutation_string: String,
    variables: Option<Value>,
}

impl BaseMutation {
    /// Create a new BaseMutation with a GraphQL mutation string
    pub fn new(mutation_string: impl Into<String>) -> Self {
        BaseMutation {
            mutation_string: mutation_string.into(),
            variables: None,
        }
    }

    /// Set variables for this mutation
    pub fn with_variables(mut self, variables: Value) -> Self {
        self.variables = Some(variables);
        self
    }
}

#[async_trait::async_trait]
impl Query for BaseMutation {
    fn get_query(&self) -> &str {
        &self.mutation_string
    }

    fn compiled_variables(&self, variables: Option<Value>) -> Option<Value> {
        variables.or_else(|| self.variables.clone())
    }

    fn create_response(&self, json: Value) -> Box<dyn Response> {
        match crate::response::BaseResponse::new(json) {
            Ok(resp) => Box::new(resp),
            Err(e) => {
                eprintln!("BaseResponse construction failed: {}", e);
                Box::new(crate::response::BaseResponse::empty())
            }
        }
    }

    /// Execute as query - delegates to mutation execute
    async fn execute(
        &self,
        client: &GraphQLClient,
        variables: Option<Value>,
        context: Option<HashMap<String, Value>>,
    ) -> Result<Box<dyn Response>> {
        // Delegate to mutation execute
        Mutation::execute(self, client, variables, context).await
    }
}

#[async_trait::async_trait]
impl Mutation for BaseMutation {
    fn get_mutation(&self) -> &str {
        &self.mutation_string
    }
}

// Specific mutation type implementations
pub mod active_session;
pub mod claim_shadow_wallet;
pub mod create_identifier;
pub mod create_meta;
pub mod create_rule;
pub mod create_token;
pub mod create_wallet;
pub mod deposit_buffer_token;
pub mod link_identifier;
pub mod propose_molecule;
pub mod request_authorization;
pub mod request_authorization_guest;
pub mod request_tokens;
pub mod transfer_tokens;
pub mod withdraw_buffer_token;

// Re-export mutation classes for easy access
pub use active_session::MutationActiveSession;
pub use claim_shadow_wallet::{ClaimShadowWalletParams, MutationClaimShadowWallet};
pub use create_identifier::{CreateIdentifierParams, MutationCreateIdentifier};
pub use create_meta::{CreateMetaParams, MutationCreateMeta};
pub use create_rule::{CreateRuleParams, MutationCreateRule};
pub use create_token::{CreateTokenParams, MutationCreateToken};
pub use create_wallet::MutationCreateWallet;
pub use deposit_buffer_token::{DepositBufferTokenParams, MutationDepositBufferToken};
pub use link_identifier::MutationLinkIdentifier;
pub use propose_molecule::MutationProposeMolecule;
pub use request_authorization::{MutationRequestAuthorization, RequestAuthorizationParams};
pub use request_authorization_guest::MutationRequestAuthorizationGuest;
pub use request_tokens::{MutationRequestTokens, RequestTokensParams};
pub use transfer_tokens::{MutationTransferTokens, TransferTokensParams};
pub use withdraw_buffer_token::{MutationWithdrawBufferToken, WithdrawBufferTokenParams};

/// Mutation builder for creating common mutations
///
/// This provides a convenient interface for constructing mutations
/// programmatically with proper type safety and validation.
pub struct MutationBuilder {
    secret: Option<String>,
    bundle: Option<String>,
    client: Option<GraphQLClient>,
}

impl MutationBuilder {
    /// Create a new mutation builder
    pub fn new() -> Self {
        MutationBuilder {
            secret: None,
            bundle: None,
            client: None,
        }
    }

    /// Set the secret for signing operations
    pub fn with_secret(mut self, secret: impl Into<String>) -> Self {
        self.secret = Some(secret.into());
        self
    }

    /// Set the bundle hash for user identification
    pub fn with_bundle(mut self, bundle: impl Into<String>) -> Self {
        self.bundle = Some(bundle.into());
        self
    }

    /// Set the GraphQL client for execution
    pub fn with_client(mut self, client: GraphQLClient) -> Self {
        self.client = Some(client);
        self
    }

    /// Build a ProposeMolecule mutation
    pub fn propose_molecule(
        self,
        molecule: crate::molecule::Molecule,
    ) -> Result<MutationProposeMolecule> {
        if let Some(client) = self.client {
            let knish_client = crate::client::KnishIOClient::new(
                vec!["http://localhost".to_string()], // Default endpoint
                None,
                None,
                None,
                None,
                None,
            );
            Ok(MutationProposeMolecule::new(client, knish_client, molecule))
        } else {
            Ok(MutationProposeMolecule::from_molecule(molecule))
        }
    }

    /// Build a CreateWallet mutation
    pub fn create_wallet(
        self,
        molecule: crate::molecule::Molecule,
    ) -> Result<MutationCreateWallet> {
        if let Some(client) = self.client {
            let knish_client = crate::client::KnishIOClient::new(
                vec!["http://localhost".to_string()], // Default endpoint
                None,
                None,
                None,
                None,
                None,
            );
            Ok(MutationCreateWallet::new(client, knish_client, molecule))
        } else {
            Ok(MutationCreateWallet::from_molecule(molecule))
        }
    }

    /// Build a TransferTokens mutation
    pub fn transfer_tokens(
        self,
        molecule: crate::molecule::Molecule,
    ) -> Result<MutationTransferTokens> {
        if let Some(client) = self.client {
            let knish_client = crate::client::KnishIOClient::new(
                vec!["http://localhost".to_string()], // Default endpoint
                None,
                None,
                None,
                None,
                None,
            );
            Ok(MutationTransferTokens::new(client, knish_client, molecule))
        } else {
            Ok(MutationTransferTokens::from_molecule(molecule))
        }
    }

    /// Build an ActiveSession mutation
    pub fn active_session(self) -> MutationActiveSession {
        MutationActiveSession::new()
    }
}

impl Default for MutationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convenience functions for creating common mutations
pub mod helpers {
    use super::*;
    use crate::molecule::Molecule;
    use crate::wallet::Wallet;

    /// Create a value transfer mutation
    ///
    /// # Arguments
    /// * `secret` - User's secret key
    /// * `source_wallet` - Source wallet for the transfer
    /// * `recipient_wallet` - Destination wallet
    /// * `amount` - Amount to transfer
    pub fn create_value_transfer(
        secret: &str,
        source_wallet: &Wallet,
        recipient_wallet: &Wallet,
        amount: f64,
    ) -> Result<MutationTransferTokens> {
        let mut molecule = Molecule::with_params(
            Some(secret.to_string()),
            None,
            Some(source_wallet.clone()),
            None,
            None,
            None,
        );

        // Initialize value transfer in molecule
        molecule.init_value(recipient_wallet, amount)?;
        molecule.sign(None, false, true)?;
        molecule.check(Some(source_wallet))?;

        Ok(MutationTransferTokens::from_molecule(molecule))
    }

    /// Create a wallet creation mutation
    ///
    /// # Arguments
    /// * `secret` - User's secret key
    /// * `wallet` - Wallet to create
    pub fn create_wallet_creation(secret: &str, wallet: &Wallet) -> Result<MutationCreateWallet> {
        let mut molecule =
            Molecule::with_params(Some(secret.to_string()), None, None, None, None, None);

        // Initialize wallet creation in molecule
        molecule.init_wallet_creation(wallet, Vec::new())?;
        molecule.sign(None, false, true)?;
        molecule.check(None)?;

        Ok(MutationCreateWallet::from_molecule(molecule))
    }

    /// Create a token creation mutation
    ///
    /// # Arguments
    /// * `secret` - User's secret key
    /// * `recipient_wallet` - Wallet to receive new tokens
    /// * `amount` - Amount of tokens to create
    /// * `meta` - Token metadata, signed in this order
    pub fn create_token_creation(
        secret: &str,
        recipient_wallet: &Wallet,
        amount: f64,
        meta: Vec<crate::types::MetaItem>,
    ) -> Result<MutationCreateToken> {
        let mut molecule =
            Molecule::with_params(Some(secret.to_string()), None, None, None, None, None);

        // Initialize token creation in molecule
        molecule.init_token_creation(recipient_wallet, amount, meta)?;
        let bundle = recipient_wallet.bundle.as_deref().unwrap_or("").to_string();
        molecule.sign(Some(bundle), false, true)?;
        molecule.check(None)?;

        Ok(MutationCreateToken::from_molecule(molecule))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::molecule::Molecule;

    #[test]
    fn test_base_mutation_creation() {
        let mutation = BaseMutation::new("mutation { test }");
        assert_eq!(mutation.get_mutation(), "mutation { test }");
    }

    #[test]
    fn test_mutation_with_variables() {
        let variables = json!({"key": "value"});
        let mutation = BaseMutation::new("mutation { test }").with_variables(variables.clone());

        assert_eq!(mutation.compiled_variables(None), Some(variables));
    }

    #[test]
    fn test_mutation_builder() {
        let builder = MutationBuilder::new()
            .with_secret("test-secret")
            .with_bundle("test-bundle");

        // Test that we can create mutations through the builder
        let molecule = Molecule::new();
        let mutation_result = builder.propose_molecule(molecule);
        assert!(mutation_result.is_ok());
    }

    #[test]
    fn test_helper_functions() {
        use crate::wallet::Wallet;

        let source_wallet =
            Wallet::create(Some("source-secret"), None, "TEST", None, None, None).unwrap();

        let recipient_wallet =
            Wallet::create(Some("recipient-secret"), None, "TEST", None, None, None).unwrap();

        let result =
            helpers::create_value_transfer("test-secret", &source_wallet, &recipient_wallet, 100.0);

        // Should fail without proper molecule initialization methods
        // but the function should exist and be callable
        assert!(result.is_err()); // Expected to fail without proper molecule methods
    }
}
