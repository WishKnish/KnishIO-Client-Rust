//! Frozen secret-storage envelope fixtures.
//!
//! Shared single-source-of-truth between `tests/secret_storage.rs` (which decrypts it standalone)
//! and `tests/cross_platform_vectors.rs` (which verifies parity with the master vector when fixtures are present).

pub const FROZEN_TS_ENVELOPE: &str = r#"{"version":1,"ciphertext":"dQjZ4cR+ZBefuF4xSib8Qv/H2oZ5Qv8mRCRmQuLaCDYoBaRMqPQRullxZsID","iv":"3Q8ArAH0ZEgYFpoE","salt":"4LWNzAFGrY4SzcPulKMVcg==","algorithm":"AES-GCM","iterations":100000,"metadata":{"bundleHash":"deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef","label":"probe","createdAt":1788558526546,"hardwareBacked":false,"providerType":"webcrypto-aes-gcm"}}"#;

pub const FROZEN_LEGACY_0_9_5_ENVELOPE: &str = r#"{"version":1,"ciphertext":"jf1+FJAP8itTOuBH8gur0lnmkGYps1TQyx/Q+3Ah+kAk8eC9/gt2DjZ5tz4v","iv":"XjR5FNNDRipgM6hx","salt":"lhpi9ywH55D6ZnUIrnr8+Q==","algorithm":"AES-GCM","iterations":100000,"metadata":{"bundle_hash":"deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef","label":"probe","created_at":1788558634482,"hardware_backed":false,"provider_type":"aes-gcm"}}"#;

pub const XSDK_BUNDLE: &str = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";
pub const XSDK_PASSPHRASE: &str = "cross-sdk-pass";
pub const XSDK_PLAINTEXT: &str = "MASTER-SECRET-CROSS-SDK-PROBE";
