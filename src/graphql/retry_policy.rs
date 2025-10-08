//! Advanced retry policies for GraphQL operations
//!
//! This module provides sophisticated retry logic with different strategies
//! for handling various types of failures in GraphQL operations.

use crate::error::{KnishIOError, Result};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// Retry policy for GraphQL operations
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Strategy to use for retries
    pub strategy: RetryStrategy,
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Jitter factor (0.0 to 1.0) to add randomness
    pub jitter_factor: f64,
    /// Conditions that should trigger a retry
    pub retry_conditions: Vec<RetryCondition>,
}

/// Different retry strategies
#[derive(Debug, Clone, PartialEq)]
pub enum RetryStrategy {
    /// Fixed delay between retries
    Fixed,
    /// Exponential backoff with configurable multiplier
    ExponentialBackoff { multiplier: f64 },
    /// Linear backoff (delay increases linearly)
    LinearBackoff { increment: Duration },
    /// Custom function-based strategy
    Custom,
}

/// Conditions that determine when to retry
#[derive(Debug, Clone, PartialEq)]
pub enum RetryCondition {
    /// Retry on network errors (timeouts, connection failures)
    NetworkError,
    /// Retry on HTTP 5xx server errors
    ServerError,
    /// Retry on specific HTTP status codes
    HttpStatus(u16),
    /// Retry on GraphQL errors with specific messages
    GraphQLError { message_contains: String },
    /// Retry on rate limiting (HTTP 429)
    RateLimit,
    /// Retry on timeouts
    Timeout,
    /// Custom condition function
    Custom,
}


/// Retry executor that implements the retry logic
pub struct RetryExecutor {
    policy: RetryPolicy,
    current_attempt: u32,
    debug: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        RetryPolicy {
            strategy: RetryStrategy::ExponentialBackoff { multiplier: 2.0 },
            max_attempts: 3,
            initial_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            jitter_factor: 0.1,
            retry_conditions: vec![
                RetryCondition::NetworkError,
                RetryCondition::ServerError,
                RetryCondition::Timeout,
                RetryCondition::RateLimit,
            ],
        }
    }
}

impl RetryPolicy {
    /// Create a new retry policy with default settings
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Create a retry policy optimized for network operations
    pub fn network_optimized() -> Self {
        RetryPolicy {
            strategy: RetryStrategy::ExponentialBackoff { multiplier: 2.0 },
            max_attempts: 5,
            initial_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(60),
            jitter_factor: 0.2,
            retry_conditions: vec![
                RetryCondition::NetworkError,
                RetryCondition::Timeout,
                RetryCondition::HttpStatus(502),
                RetryCondition::HttpStatus(503),
                RetryCondition::HttpStatus(504),
            ],
        }
    }
    
    /// Create a retry policy optimized for API rate limiting
    pub fn rate_limit_optimized() -> Self {
        RetryPolicy {
            strategy: RetryStrategy::ExponentialBackoff { multiplier: 1.5 },
            max_attempts: 10,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(120),
            jitter_factor: 0.25,
            retry_conditions: vec![
                RetryCondition::RateLimit,
                RetryCondition::HttpStatus(429),
            ],
        }
    }
    
    /// Create a retry policy for GraphQL-specific errors
    pub fn graphql_optimized() -> Self {
        RetryPolicy {
            strategy: RetryStrategy::Fixed,
            max_attempts: 3,
            initial_delay: Duration::from_millis(2000),
            max_delay: Duration::from_secs(10),
            jitter_factor: 0.1,
            retry_conditions: vec![
                RetryCondition::GraphQLError {
                    message_contains: "rate limit".to_string(),
                },
                RetryCondition::GraphQLError {
                    message_contains: "timeout".to_string(),
                },
                RetryCondition::GraphQLError {
                    message_contains: "unavailable".to_string(),
                },
            ],
        }
    }
    
    /// Builder pattern methods
    pub fn with_max_attempts(mut self, max_attempts: u32) -> Self {
        self.max_attempts = max_attempts;
        self
    }
    
    pub fn with_strategy(mut self, strategy: RetryStrategy) -> Self {
        self.strategy = strategy;
        self
    }
    
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }
    
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }
    
    pub fn with_jitter(mut self, jitter_factor: f64) -> Self {
        self.jitter_factor = jitter_factor.clamp(0.0, 1.0);
        self
    }
    
    pub fn with_conditions(mut self, conditions: Vec<RetryCondition>) -> Self {
        self.retry_conditions = conditions;
        self
    }
    
    pub fn add_condition(mut self, condition: RetryCondition) -> Self {
        self.retry_conditions.push(condition);
        self
    }
    
    /// Calculate the delay for a specific attempt number
    pub fn calculate_delay(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return Duration::from_millis(0);
        }
        
        let base_delay = match self.strategy {
            RetryStrategy::Fixed => self.initial_delay,
            RetryStrategy::ExponentialBackoff { multiplier } => {
                let delay_ms = self.initial_delay.as_millis() as f64 * multiplier.powi((attempt - 1) as i32);
                Duration::from_millis(delay_ms as u64)
            },
            RetryStrategy::LinearBackoff { increment } => {
                Duration::from_millis(
                    self.initial_delay.as_millis() as u64 + 
                    increment.as_millis() as u64 * (attempt - 1) as u64
                )
            },
            RetryStrategy::Custom => {
                // For custom strategies, fallback to exponential
                let delay_ms = self.initial_delay.as_millis() as f64 * 2.0_f64.powi((attempt - 1) as i32);
                Duration::from_millis(delay_ms as u64)
            }
        };
        
        // Apply maximum delay limit
        let capped_delay = std::cmp::min(base_delay, self.max_delay);
        
        // Apply jitter if configured
        if self.jitter_factor > 0.0 {
            let jitter_ms = (capped_delay.as_millis() as f64 * self.jitter_factor * rand::random::<f64>()) as u64;
            let jitter_duration = Duration::from_millis(jitter_ms);
            
            if rand::random::<bool>() {
                capped_delay + jitter_duration
            } else {
                capped_delay.saturating_sub(jitter_duration)
            }
        } else {
            capped_delay
        }
    }
    
    /// Check if an error should trigger a retry
    pub fn should_retry(&self, error: &KnishIOError) -> bool {
        for condition in &self.retry_conditions {
            if self.matches_condition(condition, error) {
                return true;
            }
        }
        false
    }
    
    /// Check if an error matches a specific retry condition
    fn matches_condition(&self, condition: &RetryCondition, error: &KnishIOError) -> bool {
        match condition {
            RetryCondition::NetworkError => {
                matches!(error, KnishIOError::Network(_))
            },
            RetryCondition::ServerError => {
                if let KnishIOError::Custom(msg) = error {
                    msg.contains("HTTP error: 5") // Matches 5xx errors
                } else {
                    false
                }
            },
            RetryCondition::HttpStatus(status) => {
                if let KnishIOError::Custom(msg) = error {
                    msg.contains(&format!("HTTP error: {}", status))
                } else {
                    false
                }
            },
            RetryCondition::GraphQLError { message_contains } => {
                error.to_string().to_lowercase().contains(&message_contains.to_lowercase())
            },
            RetryCondition::RateLimit => {
                if let KnishIOError::Custom(msg) = error {
                    msg.contains("HTTP error: 429") || 
                    msg.to_lowercase().contains("rate limit")
                } else {
                    false
                }
            },
            RetryCondition::Timeout => {
                if let KnishIOError::Custom(msg) = error {
                    msg.to_lowercase().contains("timeout")
                } else {
                    false
                }
            },
            RetryCondition::Custom => {
                // For custom conditions, always return false
                // This should be handled by the caller with custom logic
                false
            }
        }
    }
    
    /// Create an executor for this policy
    pub fn executor(&self, debug: bool) -> RetryExecutor {
        RetryExecutor::new(self.clone(), debug)
    }
}

impl RetryExecutor {
    /// Create a new retry executor
    pub fn new(policy: RetryPolicy, debug: bool) -> Self {
        RetryExecutor {
            policy,
            current_attempt: 0,
            debug,
        }
    }
    
    /// Execute a closure with retry logic
    pub async fn execute<F, Fut, T>(&mut self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;
        
        for attempt in 1..=self.policy.max_attempts {
            self.current_attempt = attempt;
            
            match operation().await {
                Ok(result) => {
                    if self.debug && attempt > 1 {
                        debug!("Operation succeeded on attempt {}", attempt);
                    }
                    return Ok(result);
                }
                Err(error) => {
                    last_error = Some(error.clone());
                    
                    // Check if we should retry this error
                    if !self.policy.should_retry(&error) {
                        if self.debug {
                            debug!("Error does not match retry conditions: {}", error);
                        }
                        return Err(error);
                    }
                    
                    // Don't retry if we've reached max attempts
                    if attempt >= self.policy.max_attempts {
                        if self.debug {
                            warn!("Max retry attempts ({}) reached, failing", self.policy.max_attempts);
                        }
                        return Err(error);
                    }
                    
                    // Calculate delay and wait
                    let delay = self.policy.calculate_delay(attempt);
                    
                    if self.debug {
                        warn!(
                            "Operation failed on attempt {} ({}), retrying in {:?}",
                            attempt, error, delay
                        );
                    }
                    
                    if !delay.is_zero() {
                        sleep(delay).await;
                    }
                }
            }
        }
        
        // This should never be reached due to the loop logic above,
        // but just in case...
        Err(last_error.unwrap_or_else(|| {
            KnishIOError::custom("Retry executor failed with no recorded error")
        }))
    }
    
    /// Execute with custom retry condition logic
    pub async fn execute_with_custom_condition<F, Fut, T, C>(
        &mut self,
        operation: F,
        custom_condition: C,
    ) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
        C: Fn(&KnishIOError) -> bool,
    {
        let mut last_error = None;
        
        for attempt in 1..=self.policy.max_attempts {
            self.current_attempt = attempt;
            
            match operation().await {
                Ok(result) => {
                    if self.debug && attempt > 1 {
                        debug!("Operation succeeded on attempt {}", attempt);
                    }
                    return Ok(result);
                }
                Err(error) => {
                    last_error = Some(error.clone());
                    
                    // Check both built-in and custom conditions
                    let should_retry = self.policy.should_retry(&error) || custom_condition(&error);
                    
                    if !should_retry {
                        if self.debug {
                            debug!("Error does not match retry conditions: {}", error);
                        }
                        return Err(error);
                    }
                    
                    if attempt >= self.policy.max_attempts {
                        if self.debug {
                            warn!("Max retry attempts ({}) reached, failing", self.policy.max_attempts);
                        }
                        return Err(error);
                    }
                    
                    let delay = self.policy.calculate_delay(attempt);
                    
                    if self.debug {
                        warn!(
                            "Operation failed on attempt {} ({}), retrying in {:?}",
                            attempt, error, delay
                        );
                    }
                    
                    if !delay.is_zero() {
                        sleep(delay).await;
                    }
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| {
            KnishIOError::custom("Retry executor failed with no recorded error")
        }))
    }
    
    /// Get the current attempt number
    pub fn current_attempt(&self) -> u32 {
        self.current_attempt
    }
    
    /// Reset the executor for reuse
    pub fn reset(&mut self) {
        self.current_attempt = 0;
    }
}

/// Convenience function to execute an operation with retry logic
pub async fn execute_with_retry<F, Fut, T>(
    policy: RetryPolicy,
    debug: bool,
    operation: F,
) -> Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
{
    let mut executor = policy.executor(debug);
    executor.execute(operation).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;
    
    #[test]
    fn test_retry_policy_defaults() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 3);
        assert_eq!(policy.initial_delay, Duration::from_millis(1000));
        assert!(matches!(policy.strategy, RetryStrategy::ExponentialBackoff { multiplier } if multiplier == 2.0));
    }
    
    #[test]
    fn test_delay_calculation() {
        let policy = RetryPolicy::new()
            .with_initial_delay(Duration::from_millis(100))
            .with_strategy(RetryStrategy::ExponentialBackoff { multiplier: 2.0 })
            .with_jitter(0.0); // No jitter for predictable testing
        
        assert_eq!(policy.calculate_delay(0), Duration::from_millis(0));
        assert_eq!(policy.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(policy.calculate_delay(2), Duration::from_millis(200));
        assert_eq!(policy.calculate_delay(3), Duration::from_millis(400));
    }
    
    #[test]
    fn test_fixed_strategy() {
        let policy = RetryPolicy::new()
            .with_initial_delay(Duration::from_millis(500))
            .with_strategy(RetryStrategy::Fixed)
            .with_jitter(0.0);
        
        assert_eq!(policy.calculate_delay(1), Duration::from_millis(500));
        assert_eq!(policy.calculate_delay(2), Duration::from_millis(500));
        assert_eq!(policy.calculate_delay(3), Duration::from_millis(500));
    }
    
    #[test]
    fn test_linear_backoff() {
        let policy = RetryPolicy::new()
            .with_initial_delay(Duration::from_millis(100))
            .with_strategy(RetryStrategy::LinearBackoff {
                increment: Duration::from_millis(50)
            })
            .with_jitter(0.0);
        
        assert_eq!(policy.calculate_delay(1), Duration::from_millis(100));
        assert_eq!(policy.calculate_delay(2), Duration::from_millis(150));
        assert_eq!(policy.calculate_delay(3), Duration::from_millis(200));
    }
    
    #[test]
    fn test_max_delay_capping() {
        let policy = RetryPolicy::new()
            .with_initial_delay(Duration::from_millis(1000))
            .with_max_delay(Duration::from_millis(2000))
            .with_strategy(RetryStrategy::ExponentialBackoff { multiplier: 10.0 })
            .with_jitter(0.0);
        
        assert_eq!(policy.calculate_delay(1), Duration::from_millis(1000));
        assert_eq!(policy.calculate_delay(2), Duration::from_millis(2000)); // Capped
        assert_eq!(policy.calculate_delay(3), Duration::from_millis(2000)); // Still capped
    }
    
    #[test]
    fn test_retry_conditions() {
        let policy = RetryPolicy::new();
        
        // Network error should trigger retry
        let network_error = KnishIOError::Network("Connection error".to_string());
        assert!(policy.should_retry(&network_error));
        
        // Server error should trigger retry
        let server_error = KnishIOError::custom("HTTP error: 500");
        assert!(policy.should_retry(&server_error));
        
        // Rate limit should trigger retry
        let rate_limit_error = KnishIOError::custom("HTTP error: 429");
        assert!(policy.should_retry(&rate_limit_error));
        
        // Other errors should not trigger retry
        let validation_error = KnishIOError::AtomsMissing;
        assert!(!policy.should_retry(&validation_error));
    }
    
    #[test]
    fn test_graphql_error_condition() {
        let policy = RetryPolicy::new()
            .with_conditions(vec![
                RetryCondition::GraphQLError {
                    message_contains: "rate limit".to_string(),
                }
            ]);
        
        let graphql_error = KnishIOError::custom("GraphQL error: rate limit exceeded");
        assert!(policy.should_retry(&graphql_error));
        
        let other_error = KnishIOError::custom("GraphQL error: validation failed");
        assert!(!policy.should_retry(&other_error));
    }
    
    #[tokio::test]
    async fn test_retry_executor_success() {
        let policy = RetryPolicy::new().with_max_attempts(3);
        let mut executor = policy.executor(false);
        
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let result = executor.execute(move || {
            let counter = counter_clone.clone();
            async move {
                let count = counter.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    Err(KnishIOError::custom("HTTP error: 500"))
                } else {
                    Ok("success")
                }
            }
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "success");
        assert_eq!(counter.load(Ordering::SeqCst), 3); // 3 attempts
    }
    
    #[tokio::test]
    async fn test_retry_executor_max_attempts() {
        let policy = RetryPolicy::new().with_max_attempts(2);
        let mut executor = policy.executor(false);
        
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let result = executor.execute(move || {
            let counter = counter_clone.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Err::<(), _>(KnishIOError::custom("HTTP error: 500"))
            }
        }).await;
        
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 2); // Only 2 attempts
    }
    
    #[tokio::test]
    async fn test_retry_executor_no_retry_condition() {
        let policy = RetryPolicy::new()
            .with_max_attempts(3)
            .with_conditions(vec![RetryCondition::NetworkError]); // Only retry network errors
        
        let mut executor = policy.executor(false);
        
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let result = executor.execute(move || {
            let counter = counter_clone.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Err::<(), _>(KnishIOError::custom("HTTP error: 400")) // Client error, not retryable
            }
        }).await;
        
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1); // Only 1 attempt, no retries
    }
    
    #[test]
    fn test_specialized_policies() {
        let network_policy = RetryPolicy::network_optimized();
        assert_eq!(network_policy.max_attempts, 5);
        assert!(network_policy.retry_conditions.contains(&RetryCondition::NetworkError));
        
        let rate_limit_policy = RetryPolicy::rate_limit_optimized();
        assert_eq!(rate_limit_policy.max_attempts, 10);
        assert!(rate_limit_policy.retry_conditions.contains(&RetryCondition::RateLimit));
        
        let graphql_policy = RetryPolicy::graphql_optimized();
        assert!(matches!(graphql_policy.strategy, RetryStrategy::Fixed));
    }
}