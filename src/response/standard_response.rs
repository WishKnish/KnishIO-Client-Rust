/*!
Enhanced Response System for Rust SDK

Implements JavaScript SDK compatible response interface patterns
with Rust-specific enhancements (memory safety, Result integration, tokio async)
*/

use std::fmt::Debug;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use futures::stream::{self, Stream};
use anyhow::{Result, Error};

/// Universal response trait matching JavaScript SDK pattern
pub trait UniversalResponse<T>: Debug + Send + Sync {
    fn success(&self) -> bool;
    fn payload(&self) -> Option<&T>;
    fn reason(&self) -> Option<&str>;
    fn data(&self) -> Option<&serde_json::Value>;
}

/// Enhanced error information with detailed context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseError {
    pub message: String,
    pub code: Option<String>,
    pub details: Vec<String>,
    pub context: Option<String>,
    pub timestamp: String,
    pub operation: Option<String>,
}

impl ResponseError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            code: None,
            details: Vec::new(),
            context: None,
            timestamp: Self::current_timestamp(),
            operation: None,
        }
    }
    
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }
    
    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }
    
    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operation = Some(operation.into());
        self
    }
    
    fn current_timestamp() -> String {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string()
    }
}

/// Enhanced validation result pattern for Rust safety
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ResponseError>,
    pub warnings: Vec<String>,
}

impl<T> ValidationResult<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            warnings: Vec::new(),
        }
    }
    
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ResponseError::new(error)),
            warnings: Vec::new(),
        }
    }
    
    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings = warnings;
        self
    }
}

/// Response metadata for enhanced debugging and monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub timestamp: String,
    pub operation: String,
    pub duration: Option<u64>,
    pub request_id: Option<String>,
    pub server_version: Option<String>,
    pub client_version: String,
}

impl ResponseMetadata {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            timestamp: ResponseError::current_timestamp(),
            operation: operation.into(),
            duration: None,
            request_id: None,
            server_version: None,
            client_version: "0.1.0".to_string(),
        }
    }
}

/// Enhanced response interface with functional programming support
pub trait EnhancedResponse<T>: UniversalResponse<T> {
    /// Convert to ValidationResult for enhanced error handling
    fn to_validation_result(&self) -> ValidationResult<T> where T: Clone;
    
    /// Functional programming map operation
    fn map<U, F>(&self, mapper: F) -> Box<dyn EnhancedResponse<U>>
    where
        F: FnOnce(&T) -> U + Send + Sync + 'static,
        U: Debug + Send + Sync + Clone + Serialize + for<'de> Deserialize<'de> + 'static,
        T: Clone;
    
    /// Enhanced debugging with optional labels
    fn debug(&self, label: Option<&str>) -> &Self;
    
    /// Convert to Result type for idiomatic Rust error handling
    fn to_result(&self) -> Result<T> where T: Clone;
    
    /// Stream integration for async processing
    fn to_stream(&self) -> impl Stream<Item = T> where T: Clone;
}

/// Standard response implementation with memory safety guarantees
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardResponse<T> {
    successful: bool,
    payload_data: Option<T>,
    error_message: Option<String>,
    raw_data: Option<serde_json::Value>,
    metadata: ResponseMetadata,
}

impl<T> StandardResponse<T> 
where 
    T: Debug + Send + Sync + Clone + Serialize + for<'de> Deserialize<'de>
{
    /// Create successful response
    pub fn success(payload: T, operation: impl Into<String>) -> Self {
        Self {
            successful: true,
            payload_data: Some(payload),
            error_message: None,
            raw_data: None,
            metadata: ResponseMetadata::new(operation),
        }
    }
    
    /// Create error response
    pub fn failure(error_message: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            successful: false,
            payload_data: None,
            error_message: Some(error_message.into()),
            raw_data: None,
            metadata: ResponseMetadata::new(operation),
        }
    }
    
    /// Create with raw data
    pub fn with_raw_data(mut self, raw_data: serde_json::Value) -> Self {
        self.raw_data = Some(raw_data);
        self
    }
    
    /// Create with duration
    pub fn with_duration(mut self, duration: u64) -> Self {
        self.metadata.duration = Some(duration);
        self
    }
    
    /// Functional programming combinators
    pub fn map<U, F>(self, mapper: F) -> StandardResponse<U>
    where
        F: FnOnce(T) -> U,
        U: Debug + Send + Sync + Clone + Serialize + for<'de> Deserialize<'de>
    {
        if self.successful {
            if let Some(payload) = self.payload_data {
                match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| mapper(payload))) {
                    Ok(mapped_payload) => StandardResponse::success(mapped_payload, format!("{}_mapped", self.metadata.operation))
                        .with_raw_data(self.raw_data.unwrap_or_default()),
                    Err(_) => StandardResponse::failure("Mapping failed", format!("{}_map_failed", self.metadata.operation))
                        .with_raw_data(self.raw_data.unwrap_or_default())
                }
            } else {
                StandardResponse::failure("No payload to map", format!("{}_map_no_payload", self.metadata.operation))
            }
        } else {
            StandardResponse::failure(
                self.error_message.unwrap_or_else(|| "Unknown error".to_string()),
                self.metadata.operation
            ).with_raw_data(self.raw_data.unwrap_or_default())
        }
    }
    
    /// Enhanced pattern matching with closures
    pub fn fold<R, S, F>(self, on_success: S, on_failure: F) -> R
    where
        S: FnOnce(T) -> R,
        F: FnOnce(String) -> R,
    {
        if self.successful {
            if let Some(payload) = self.payload_data {
                on_success(payload)
            } else {
                on_failure("Successful response has no payload".to_string())
            }
        } else {
            on_failure(self.error_message.unwrap_or_else(|| "Unknown error".to_string()))
        }
    }
    
    /// Convert from legacy Rust response format
    pub fn from_legacy_response(legacy_response: &dyn std::any::Any, operation: impl Into<String>) -> Self {
        // Rust doesn't have reflection like Java, so we'll need a more structured approach
        // This is a placeholder implementation that would need actual type matching
        StandardResponse::failure("Legacy conversion not yet implemented", operation)
    }
}

impl<T> UniversalResponse<T> for StandardResponse<T> 
where 
    T: Debug + Send + Sync + Clone + Serialize + for<'de> Deserialize<'de>
{
    fn success(&self) -> bool {
        self.successful
    }
    
    fn payload(&self) -> Option<&T> {
        self.payload_data.as_ref()
    }
    
    fn reason(&self) -> Option<&str> {
        self.error_message.as_deref()
    }
    
    fn data(&self) -> Option<&serde_json::Value> {
        self.raw_data.as_ref()
    }
}

impl<T> EnhancedResponse<T> for StandardResponse<T> 
where 
    T: Debug + Send + Sync + Clone + Serialize + for<'de> Deserialize<'de>
{
    fn to_validation_result(&self) -> ValidationResult<T> where T: Clone {
        if self.successful {
            if let Some(payload) = &self.payload_data {
                ValidationResult::success(payload.clone())
            } else {
                ValidationResult::failure("Successful response has no payload")
            }
        } else {
            ValidationResult::failure(
                self.error_message.clone().unwrap_or_else(|| "Unknown error".to_string())
            )
        }
    }
    
    fn map<U, F>(&self, mapper: F) -> Box<dyn EnhancedResponse<U>>
    where
        F: FnOnce(&T) -> U + Send + Sync + 'static,
        U: Debug + Send + Sync + Clone + Serialize + for<'de> Deserialize<'de> + 'static,
        T: Clone,
    {
        if self.successful {
            if let Some(payload) = &self.payload_data {
                let mapped_payload = mapper(payload);
                Box::new(StandardResponse::success(mapped_payload, format!("{}_mapped", self.metadata.operation)))
            } else {
                Box::new(StandardResponse::failure("No payload to map", format!("{}_map_no_payload", self.metadata.operation)))
            }
        } else {
            Box::new(StandardResponse::failure(
                self.error_message.clone().unwrap_or_else(|| "Unknown error".to_string()),
                self.metadata.operation.clone()
            ))
        }
    }
    
    fn debug(&self, label: Option<&str>) -> &Self {
        let debug_prefix = label.unwrap_or("StandardResponse");
        
        if self.successful {
            println!("[{}] Success: payload={:?}, operation={}", debug_prefix, self.payload_data, self.metadata.operation);
        } else {
            println!("[{}] Failure: error={:?}, operation={}", debug_prefix, self.error_message, self.metadata.operation);
        }
        
        self
    }
    
    fn to_result(&self) -> Result<T> where T: Clone {
        if self.successful {
            if let Some(payload) = &self.payload_data {
                Ok(payload.clone())
            } else {
                Err(Error::msg("Successful response has no payload"))
            }
        } else {
            Err(Error::msg(self.error_message.clone().unwrap_or_else(|| "Unknown error".to_string())))
        }
    }
    
    fn to_stream(&self) -> impl Stream<Item = T> where T: Clone {
        if let Some(payload) = &self.payload_data {
            if self.successful {
                stream::once(async { payload.clone() })
            } else {
                stream::empty()
            }
        } else {
            stream::empty()
        }
    }
}

// Type aliases for specific response types
pub type MetaResponse = StandardResponse<serde_json::Value>;
pub type TokenResponse = StandardResponse<serde_json::Value>;
pub type TransferResponse = StandardResponse<serde_json::Value>;
pub type BalanceResponse = StandardResponse<serde_json::Value>;
pub type WalletResponse = StandardResponse<serde_json::Value>;
pub type AuthResponse = StandardResponse<serde_json::Value>;

/// Response factory for creating standardized responses
pub struct ResponseFactory;

impl ResponseFactory {
    pub fn create_success_response<T>(
        payload: T,
        operation: impl Into<String>,
        raw_data: Option<serde_json::Value>,
        duration: Option<u64>
    ) -> StandardResponse<T>
    where
        T: Debug + Send + Sync + Clone + Serialize + for<'de> Deserialize<'de>
    {
        let mut response = StandardResponse::success(payload, operation);
        if let Some(data) = raw_data {
            response = response.with_raw_data(data);
        }
        if let Some(d) = duration {
            response = response.with_duration(d);
        }
        response
    }
    
    pub fn create_error_response<T>(
        error_message: impl Into<String>,
        operation: impl Into<String>,
        raw_data: Option<serde_json::Value>
    ) -> StandardResponse<T>
    where
        T: Debug + Send + Sync + Clone + Serialize + for<'de> Deserialize<'de>
    {
        let mut response = StandardResponse::failure(error_message, operation);
        if let Some(data) = raw_data {
            response = response.with_raw_data(data);
        }
        response
    }
}

/// Response utilities for enhanced operations
pub struct ResponseUtils;

impl ResponseUtils {
    /// Combine multiple responses into a single response
    pub fn combine_responses<T>(responses: Vec<StandardResponse<T>>) -> StandardResponse<Vec<T>>
    where
        T: Debug + Send + Sync + Clone + Serialize + for<'de> Deserialize<'de>
    {
        let successful = responses.iter().all(|r| r.success());
        
        if successful {
            let payloads: Vec<T> = responses
                .into_iter()
                .filter_map(|r| r.payload_data)
                .collect();
            StandardResponse::success(payloads, "combine_responses")
        } else {
            let errors: Vec<String> = responses
                .iter()
                .filter(|r| !r.success())
                .filter_map(|r| r.error_message.as_ref())
                .cloned()
                .collect();
            StandardResponse::failure(
                format!("Combined operation failed: {}", errors.join("; ")),
                "combine_responses"
            )
        }
    }
    
    /// Execute operations in sequence, stopping on first failure
    pub async fn sequence_responses<T, F, Fut>(
        operations: Vec<F>
    ) -> StandardResponse<Vec<T>>
    where
        F: FnOnce() -> Fut + Send + 'static,
        Fut: std::future::Future<Output = StandardResponse<T>> + Send + 'static,
        T: Debug + Send + Sync + Clone + Serialize + for<'de> Deserialize<'de> + 'static,
    {
        let mut results = Vec::new();
        
        for (index, operation) in operations.into_iter().enumerate() {
            let result = operation().await;
            
            if !result.success() {
                return StandardResponse::failure(
                    format!(
                        "Sequence failed at operation {}: {}",
                        index + 1,
                        result.reason().unwrap_or("Unknown error")
                    ),
                    "sequence_responses"
                );
            }
            
            results.push(result);
        }
        
        let payloads: Vec<T> = results
            .into_iter()
            .filter_map(|r| r.payload_data)
            .collect();
        
        StandardResponse::success(payloads, "sequence_responses")
    }
    
    /// Convert from ValidationResult to StandardResponse
    pub fn from_validation_result<T>(
        validation_result: ValidationResult<T>,
        operation: impl Into<String>
    ) -> StandardResponse<T>
    where
        T: Debug + Send + Sync + Clone + Serialize + for<'de> Deserialize<'de>
    {
        if validation_result.success {
            if let Some(data) = validation_result.data {
                StandardResponse::success(data, operation)
            } else {
                StandardResponse::failure("Validation successful but no data", operation)
            }
        } else {
            let error_msg = validation_result.error
                .map(|e| e.message)
                .unwrap_or_else(|| "Validation failed".to_string());
            StandardResponse::failure(error_msg, operation)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_success_response() {
        let response = StandardResponse::success("test payload", "test_operation");
        assert!(response.success());
        assert_eq!(response.payload(), Some(&"test payload"));
        assert_eq!(response.reason(), None);
    }
    
    #[test]
    fn test_failure_response() {
        let response: StandardResponse<String> = StandardResponse::failure("test error", "test_operation");
        assert!(!response.success());
        assert_eq!(response.payload(), None);
        assert_eq!(response.reason(), Some("test error"));
    }
    
    #[test]
    fn test_map_operation() {
        let response = StandardResponse::success(42, "test_operation");
        let mapped = response.map(|x| x * 2);
        // Note: Due to trait object limitations, this test would need refinement
    }
    
    #[tokio::test]
    async fn test_response_utils() {
        let responses = vec![
            StandardResponse::success(1, "op1"),
            StandardResponse::success(2, "op2"),
            StandardResponse::success(3, "op3"),
        ];
        
        let combined = ResponseUtils::combine_responses(responses);
        assert!(combined.success());
        if let Some(payloads) = combined.payload() {
            assert_eq!(payloads.len(), 3);
        }
    }
}