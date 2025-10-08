//! QueryUserActivity implementation
//!
//! Query for retrieving information about user activity,
//! equivalent to QueryUserActivity.js

use crate::query::Query;
use crate::response::{Response, ResponseQueryUserActivity};
use serde_json::{json, Value};

/// Query for retrieving information about user activity
#[derive(Debug, Default)]
pub struct QueryUserActivity {
    /// Bundle hash to filter by
    bundle_hash: Option<String>,
    /// Meta type to filter by
    meta_type: Option<String>,
    /// Meta ID to filter by
    meta_id: Option<String>,
    /// IP address to filter by
    ip_address: Option<String>,
    /// Browser to filter by
    browser: Option<String>,
    /// OS/CPU to filter by
    os_cpu: Option<String>,
    /// Resolution to filter by
    resolution: Option<String>,
    /// Timezone to filter by
    time_zone: Option<String>,
    /// Count by parameters
    count_by: Vec<String>,
    /// Interval span
    interval: Option<String>,
}

impl QueryUserActivity {
    /// Create a new QueryUserActivity instance
    pub fn new() -> Self {
        QueryUserActivity::default()
    }

    /// Set the bundle hash parameter
    pub fn with_bundle_hash(mut self, bundle_hash: impl Into<String>) -> Self {
        self.bundle_hash = Some(bundle_hash.into());
        self
    }

    /// Set the meta type parameter
    pub fn with_meta_type(mut self, meta_type: impl Into<String>) -> Self {
        self.meta_type = Some(meta_type.into());
        self
    }

    /// Set the meta ID parameter
    pub fn with_meta_id(mut self, meta_id: impl Into<String>) -> Self {
        self.meta_id = Some(meta_id.into());
        self
    }

    /// Set the IP address parameter
    pub fn with_ip_address(mut self, ip_address: impl Into<String>) -> Self {
        self.ip_address = Some(ip_address.into());
        self
    }

    /// Set the browser parameter
    pub fn with_browser(mut self, browser: impl Into<String>) -> Self {
        self.browser = Some(browser.into());
        self
    }

    /// Set the OS/CPU parameter
    pub fn with_os_cpu(mut self, os_cpu: impl Into<String>) -> Self {
        self.os_cpu = Some(os_cpu.into());
        self
    }

    /// Set the resolution parameter
    pub fn with_resolution(mut self, resolution: impl Into<String>) -> Self {
        self.resolution = Some(resolution.into());
        self
    }

    /// Set the timezone parameter
    pub fn with_time_zone(mut self, time_zone: impl Into<String>) -> Self {
        self.time_zone = Some(time_zone.into());
        self
    }

    /// Add a count by parameter
    pub fn add_count_by(mut self, count_by: impl Into<String>) -> Self {
        self.count_by.push(count_by.into());
        self
    }

    /// Set count by parameters (replacing any existing)
    pub fn with_count_by(mut self, count_by: Vec<String>) -> Self {
        self.count_by = count_by;
        self
    }

    /// Set the interval parameter
    pub fn with_interval(mut self, interval: impl Into<String>) -> Self {
        self.interval = Some(interval.into());
        self
    }

    /// Get the bundle hash
    pub fn bundle_hash(&self) -> Option<&str> {
        self.bundle_hash.as_deref()
    }

    /// Get the meta type
    pub fn meta_type(&self) -> Option<&str> {
        self.meta_type.as_deref()
    }

    /// Get the meta ID
    pub fn meta_id(&self) -> Option<&str> {
        self.meta_id.as_deref()
    }
}

#[async_trait::async_trait]
impl Query for QueryUserActivity {
    /// Get the GraphQL query string (equivalent to $__query in JS)
    /// Note: This includes GraphQL fragments for recursive structures
    fn get_query(&self) -> &str {
        r#"query UserActivity (
          $bundleHash:String,
          $metaType: String,
          $metaId: String,
          $ipAddress: String,
          $browser: String,
          $osCpu: String,
          $resolution: String,
          $timeZone: String,
          $countBy: [CountByUserActivity],
          $interval: span
        ) {
          UserActivity (
            bundleHash: $bundleHash,
            metaType: $metaType,
            metaId: $metaId,
            ipAddress: $ipAddress,
            browser: $browser,
            osCpu: $osCpu,
            resolution: $resolution,
            timeZone: $timeZone,
            countBy: $countBy,
            interval: $interval
          ) {
            createdAt,
            bundleHash,
            metaType,
            metaId,
            instances {
              bundleHash,
              metaType,
              metaId,
              jsonData,
              createdAt,
              updatedAt
            },
            instanceCount {
              ...SubFields,
              ...Recursive
            }
          }
        }

        fragment SubFields on InstanceCountType {
          id,
          count
        }

        fragment Recursive on InstanceCountType {
          instances {
            ...SubFields
            instances {
              ...SubFields,
              instances {
                ...SubFields
                instances {
                  ...SubFields
                  instances {
                    ...SubFields
                    instances {
                      ...SubFields
                      instances {
                        ...SubFields
                        instances {
                          ...SubFields
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }"#
    }

    /// Compile variables for the query (equivalent to compiledVariables in JS)
    fn compiled_variables(&self, variables: Option<Value>) -> Option<Value> {
        if let Some(provided_vars) = variables {
            Some(provided_vars)
        } else {
            let mut vars = json!({});

            if let Some(ref bundle_hash) = self.bundle_hash {
                vars["bundleHash"] = json!(bundle_hash);
            }
            if let Some(ref meta_type) = self.meta_type {
                vars["metaType"] = json!(meta_type);
            }
            if let Some(ref meta_id) = self.meta_id {
                vars["metaId"] = json!(meta_id);
            }
            if let Some(ref ip_address) = self.ip_address {
                vars["ipAddress"] = json!(ip_address);
            }
            if let Some(ref browser) = self.browser {
                vars["browser"] = json!(browser);
            }
            if let Some(ref os_cpu) = self.os_cpu {
                vars["osCpu"] = json!(os_cpu);
            }
            if let Some(ref resolution) = self.resolution {
                vars["resolution"] = json!(resolution);
            }
            if let Some(ref time_zone) = self.time_zone {
                vars["timeZone"] = json!(time_zone);
            }
            if !self.count_by.is_empty() {
                vars["countBy"] = json!(self.count_by);
            }
            if let Some(ref interval) = self.interval {
                vars["interval"] = json!(interval);
            }

            Some(vars)
        }
    }

    /// Create a response from the JSON data (equivalent to createResponse in JS)
    fn create_response(&self, json: Value) -> Box<dyn Response> {
        Box::new(ResponseQueryUserActivity::new(json))
    }
}

/// Convenience methods for common usage patterns
impl QueryUserActivity {
    /// Query by bundle hash
    pub fn by_bundle_hash(bundle_hash: impl Into<String>) -> Self {
        Self::new().with_bundle_hash(bundle_hash)
    }

    /// Query by meta type
    pub fn by_meta_type(meta_type: impl Into<String>) -> Self {
        Self::new().with_meta_type(meta_type)
    }

    /// Query by meta type and ID
    pub fn by_meta(meta_type: impl Into<String>, meta_id: impl Into<String>) -> Self {
        Self::new()
            .with_meta_type(meta_type)
            .with_meta_id(meta_id)
    }

    /// Query by IP address
    pub fn by_ip_address(ip_address: impl Into<String>) -> Self {
        Self::new().with_ip_address(ip_address)
    }

    /// Query by browser
    pub fn by_browser(browser: impl Into<String>) -> Self {
        Self::new().with_browser(browser)
    }

    /// Query with counting parameters
    pub fn with_counting(count_by: Vec<String>, interval: impl Into<String>) -> Self {
        Self::new()
            .with_count_by(count_by)
            .with_interval(interval)
    }

    /// Query all user activity (no filters)
    pub fn all() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_user_activity_creation() {
        let query = QueryUserActivity::new();
        assert!(query.bundle_hash().is_none());
        assert!(query.meta_type().is_none());
        assert!(query.meta_id().is_none());
        assert!(query.count_by.is_empty());
    }

    #[test]
    fn test_query_user_activity_with_parameters() {
        let query = QueryUserActivity::new()
            .with_bundle_hash("test-bundle")
            .with_meta_type("user")
            .with_meta_id("123")
            .with_ip_address("192.168.1.1")
            .with_browser("Chrome")
            .with_os_cpu("Windows")
            .with_resolution("1920x1080")
            .with_time_zone("UTC")
            .with_interval("day");

        assert_eq!(query.bundle_hash(), Some("test-bundle"));
        assert_eq!(query.meta_type(), Some("user"));
        assert_eq!(query.meta_id(), Some("123"));
    }

    #[test]
    fn test_count_by_parameters() {
        let query = QueryUserActivity::new()
            .add_count_by("browser")
            .add_count_by("ipAddress");

        assert_eq!(query.count_by.len(), 2);
        assert_eq!(query.count_by[0], "browser");
        assert_eq!(query.count_by[1], "ipAddress");
    }

    #[test]
    fn test_convenience_methods() {
        // Test by_bundle_hash
        let query = QueryUserActivity::by_bundle_hash("test-bundle");
        assert_eq!(query.bundle_hash(), Some("test-bundle"));

        // Test by_meta_type
        let query = QueryUserActivity::by_meta_type("user");
        assert_eq!(query.meta_type(), Some("user"));

        // Test by_meta
        let query = QueryUserActivity::by_meta("user", "123");
        assert_eq!(query.meta_type(), Some("user"));
        assert_eq!(query.meta_id(), Some("123"));

        // Test by_ip_address
        let query = QueryUserActivity::by_ip_address("192.168.1.1");
        assert!(query.ip_address.is_some());

        // Test by_browser
        let query = QueryUserActivity::by_browser("Chrome");
        assert!(query.browser.is_some());

        // Test with_counting
        let count_by = vec!["browser".to_string(), "ipAddress".to_string()];
        let query = QueryUserActivity::with_counting(count_by.clone(), "day");
        assert_eq!(query.count_by, count_by);
        assert_eq!(query.interval, Some("day".to_string()));

        // Test all
        let query = QueryUserActivity::all();
        assert!(query.bundle_hash().is_none());
        assert!(query.meta_type().is_none());
    }

    #[test]
    fn test_compiled_variables() {
        let query = QueryUserActivity::new()
            .with_bundle_hash("test-bundle")
            .with_meta_type("user")
            .with_meta_id("123")
            .with_ip_address("192.168.1.1")
            .with_browser("Chrome")
            .add_count_by("browser")
            .with_interval("day");

        let variables = query.compiled_variables(None).unwrap();
        assert_eq!(variables["bundleHash"], json!("test-bundle"));
        assert_eq!(variables["metaType"], json!("user"));
        assert_eq!(variables["metaId"], json!("123"));
        assert_eq!(variables["ipAddress"], json!("192.168.1.1"));
        assert_eq!(variables["browser"], json!("Chrome"));
        assert_eq!(variables["countBy"], json!(["browser"]));
        assert_eq!(variables["interval"], json!("day"));
    }

    #[test]
    fn test_query_string() {
        let query = QueryUserActivity::new();
        let query_string = query.get_query();
        
        // Check that the query string contains expected fields
        assert!(query_string.contains("UserActivity"));
        assert!(query_string.contains("bundleHash"));
        assert!(query_string.contains("metaType"));
        assert!(query_string.contains("metaId"));
        assert!(query_string.contains("ipAddress"));
        assert!(query_string.contains("browser"));
        assert!(query_string.contains("osCpu"));
        assert!(query_string.contains("resolution"));
        assert!(query_string.contains("timeZone"));
        assert!(query_string.contains("countBy"));
        assert!(query_string.contains("interval"));
        
        // Check for fragments
        assert!(query_string.contains("fragment SubFields"));
        assert!(query_string.contains("fragment Recursive"));
        assert!(query_string.contains("instanceCount"));
        assert!(query_string.contains("...SubFields"));
        assert!(query_string.contains("...Recursive"));
    }
}