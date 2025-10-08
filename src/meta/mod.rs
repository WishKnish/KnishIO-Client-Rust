//! Metadata management for the KnishIO SDK
//!
//! This module provides structures for handling various types of metadata
//! in the KnishIO distributed ledger system.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use crate::types::MetaItem;
use crate::error::Result;

// Re-export PolicyMeta from the dedicated policy_meta module
pub use crate::policy_meta::PolicyMeta;

/// General metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub meta_type: String,
    pub meta_id: String,
    pub meta: Vec<MetaItem>,
}

impl Meta {
    /// Create a new Meta instance
    pub fn new(meta_type: impl Into<String>, meta_id: impl Into<String>) -> Self {
        Meta {
            meta_type: meta_type.into(),
            meta_id: meta_id.into(),
            meta: Vec::new(),
        }
    }
    
    /// Add a metadata item
    pub fn add_item(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.meta.push(MetaItem::new(key, value));
    }
    
    /// Convert to HashMap
    pub fn to_map(&self) -> HashMap<String, String> {
        self.meta
            .iter()
            .map(|item| (item.key.clone(), item.value.clone()))
            .collect()
    }
    
    /// Aggregate metadata from a vector of MetaItems into a HashMap
    ///
    /// Equivalent to Meta.aggregateMeta() in JavaScript SDK
    pub fn aggregate_meta(meta_items: &[MetaItem]) -> HashMap<String, String> {
        meta_items
            .iter()
            .map(|item| (item.key.clone(), item.value.clone()))
            .collect()
    }
}

/// Atom-specific metadata manager
///
/// Equivalent to AtomMeta.js class, this manages metadata for atomic operations
/// with methods for merging, context handling, wallet metadata, and policy management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtomMeta {
    pub meta: Vec<MetaItem>,
}

impl AtomMeta {
    /// Create new AtomMeta instance
    ///
    /// # Arguments
    ///
    /// * `meta` - Initial metadata as either a single MetaItem or vector
    pub fn new(meta: Option<Vec<MetaItem>>) -> Self {
        AtomMeta {
            meta: meta.unwrap_or_default(),
        }
    }

    /// Create AtomMeta from object-like data (HashMap)
    ///
    /// # Arguments
    ///
    /// * `data` - HashMap containing key-value metadata pairs
    pub fn from_map(data: HashMap<String, String>) -> Self {
        let meta_items: Vec<MetaItem> = data
            .into_iter()
            .map(|(key, value)| MetaItem::new(key, value))
            .collect();
        
        AtomMeta { meta: meta_items }
    }

    /// Merge additional metadata
    ///
    /// Equivalent to AtomMeta.merge() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `new_meta` - Additional metadata to merge
    pub fn merge(&mut self, new_meta: Vec<MetaItem>) -> &mut Self {
        // Use HashSet to deduplicate based on key
        let mut existing_keys: HashSet<String> = 
            self.meta.iter().map(|item| item.key.clone()).collect();
        
        for item in new_meta {
            if !existing_keys.contains(&item.key) {
                existing_keys.insert(item.key.clone());
                self.meta.push(item);
            }
        }
        
        self
    }

    /// Merge metadata from HashMap
    ///
    /// # Arguments
    ///
    /// * `data` - HashMap containing key-value metadata pairs
    pub fn merge_map(&mut self, data: HashMap<String, String>) -> &mut Self {
        let new_meta: Vec<MetaItem> = data
            .into_iter()
            .map(|(key, value)| MetaItem::new(key, value))
            .collect();
        
        self.merge(new_meta)
    }

    /// Add context metadata if enabled
    ///
    /// Equivalent to AtomMeta.addContext() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `context` - Optional context string (defaults to schema.org)
    pub fn add_context(&mut self, context: Option<String>) -> &mut Self {
        const USE_META_CONTEXT: bool = false;
        const DEFAULT_META_CONTEXT: &str = "https://www.schema.org";

        if USE_META_CONTEXT {
            let context_value = context.unwrap_or_else(|| DEFAULT_META_CONTEXT.to_string());
            let mut context_map = HashMap::new();
            context_map.insert("context".to_string(), context_value);
            self.merge_map(context_map);
        }

        self
    }

    /// Set atom wallet metadata
    ///
    /// Equivalent to AtomMeta.setAtomWallet() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `wallet` - Wallet instance to extract metadata from
    pub fn set_atom_wallet(&mut self, wallet: &crate::wallet::Wallet) -> &mut Self {
        let mut wallet_meta = HashMap::new();
        
        if let Some(ref pubkey) = wallet.pubkey {
            wallet_meta.insert("pubkey".to_string(), pubkey.clone());
        }
        
        if let Some(ref characters) = wallet.characters {
            wallet_meta.insert("characters".to_string(), characters.clone());
        }

        // Add token units meta key
        if !wallet.token_units.is_empty() {
            let units_data = wallet.get_token_units_data();
            if let Ok(units_json) = serde_json::to_string(&units_data) {
                wallet_meta.insert("tokenUnits".to_string(), units_json);
            }
        }

        // Add trade rates meta key
        if !wallet.trade_rates.is_empty() {
            if let Ok(rates_json) = serde_json::to_string(&wallet.trade_rates) {
                wallet_meta.insert("tradeRates".to_string(), rates_json);
            }
        }

        self.merge_map(wallet_meta)
    }

    /// Set full wallet metadata for new wallet creation
    ///
    /// Equivalent to AtomMeta.setMetaWallet() in JavaScript
    /// Used for shadow wallet claim, wallet creation, and token creation
    ///
    /// # Arguments
    ///
    /// * `wallet` - Wallet instance to extract full metadata from
    pub fn set_meta_wallet(&mut self, wallet: &crate::wallet::Wallet) -> &mut Self {
        let mut wallet_meta = HashMap::new();
        
        wallet_meta.insert("walletTokenSlug".to_string(), wallet.token.clone());
        
        if let Some(ref bundle) = wallet.bundle {
            wallet_meta.insert("walletBundleHash".to_string(), bundle.clone());
        }
        
        if let Some(ref address) = wallet.address {
            wallet_meta.insert("walletAddress".to_string(), address.clone());
        }
        
        if let Some(ref position) = wallet.position {
            wallet_meta.insert("walletPosition".to_string(), position.clone());
        }
        
        if let Some(ref batch_id) = wallet.batch_id {
            wallet_meta.insert("walletBatchId".to_string(), batch_id.clone());
        }
        
        if let Some(ref pubkey) = wallet.pubkey {
            wallet_meta.insert("walletPubkey".to_string(), pubkey.clone());
        }
        
        if let Some(ref characters) = wallet.characters {
            wallet_meta.insert("walletCharacters".to_string(), characters.clone());
        }

        self.merge_map(wallet_meta)
    }

    /// Set shadow wallet claim metadata
    ///
    /// Equivalent to AtomMeta.setShadowWalletClaim() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `shadow_wallet_claim` - Shadow wallet claim amount
    pub fn set_shadow_wallet_claim(&mut self, shadow_wallet_claim: f64) -> &mut Self {
        let mut claim_meta = HashMap::new();
        claim_meta.insert("shadowWalletClaim".to_string(), shadow_wallet_claim.to_string());
        self.merge_map(claim_meta)
    }

    /// Set signing wallet metadata
    ///
    /// Equivalent to AtomMeta.setSigningWallet() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `signing_wallet` - Wallet used for signing the operation
    pub fn set_signing_wallet(&mut self, signing_wallet: &crate::wallet::Wallet) -> &mut Self {
        let mut signing_data = HashMap::new();
        
        signing_data.insert("tokenSlug".to_string(), signing_wallet.token.clone());
        
        if let Some(ref bundle) = signing_wallet.bundle {
            signing_data.insert("bundleHash".to_string(), bundle.clone());
        }
        
        if let Some(ref address) = signing_wallet.address {
            signing_data.insert("address".to_string(), address.clone());
        }
        
        if let Some(ref position) = signing_wallet.position {
            signing_data.insert("position".to_string(), position.clone());
        }
        
        if let Some(ref pubkey) = signing_wallet.pubkey {
            signing_data.insert("pubkey".to_string(), pubkey.clone());
        }
        
        if let Some(ref characters) = signing_wallet.characters {
            signing_data.insert("characters".to_string(), characters.clone());
        }

        if let Ok(signing_json) = serde_json::to_string(&signing_data) {
            let mut signing_meta = HashMap::new();
            signing_meta.insert("signingWallet".to_string(), signing_json);
            self.merge_map(signing_meta);
        }

        self
    }

    /// Add policy metadata
    ///
    /// Equivalent to AtomMeta.addPolicy() in JavaScript
    ///
    /// # Arguments
    ///
    /// * `policy` - Policy data to add
    pub fn add_policy(&mut self, policy: serde_json::Value) -> Result<&mut Self> {
        // Get current meta keys for policy validation
        let meta_keys: Vec<String> = self.meta.iter().map(|item| item.key.clone()).collect();
        
        // Create PolicyMeta instance
        let policy_meta = PolicyMeta::new(policy, meta_keys);
        
        let policy_json = policy_meta.to_json()?;
        let mut policy_map = HashMap::new();
        policy_map.insert("policy".to_string(), policy_json);
        
        self.merge_map(policy_map);
        
        Ok(self)
    }

    /// Get the metadata as a vector
    ///
    /// Equivalent to AtomMeta.get() in JavaScript
    ///
    /// # Returns
    ///
    /// Vector of MetaItem instances
    pub fn get(&self) -> &[MetaItem] {
        &self.meta
    }

    /// Convert to HashMap for easier access
    ///
    /// # Returns
    ///
    /// HashMap representation of the metadata
    pub fn to_map(&self) -> HashMap<String, String> {
        self.meta
            .iter()
            .map(|item| (item.key.clone(), item.value.clone()))
            .collect()
    }
}

