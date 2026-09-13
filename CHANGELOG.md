# Changelog

All notable changes to the KnishIO Client Rust SDK are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Releases are published to crates.io (`knishio-client`) from a git tag.
Conventions for tags, commits, and these entries: `docs/SDK-RELEASE-CONVENTIONS.md`
in the KnishIOClientSDK monorepo.

This file was backfilled on 2026-07-27 from the repository's own tag and commit
history. Entries at and below `0.2.2` are reconstructed from commit messages
rather than written at release time; where the history does not substantiate a
detail, the entry says so instead of guessing.


## [1.1.0] — 2026-09-12
### Added

- **`SecureEnclaveSecretStorageProvider`** (`secure-enclave` feature, macOS only): hardware-backed secret storage. A P-256 key generated inside the Apple Secure Enclave (`security-framework` 3.7 / `security-framework-sys` 2.17) wraps a random device passphrase with `ECIES-Cofactor-VariableIV-X963-SHA256-AESGCM`; the wrapped passphrase is kept in the storage backend under `knishio:kek:secure-enclave:<alias>` and the key in the Data Protection Keychain under label `io.knish.secret-storage:kek:<alias>`. The master secret is stored as the standard cross-SDK envelope under that passphrase, so the wire format is unchanged. `provider_type()` is `secure-enclave-aes-gcm`, `is_hardware_backed()` is `true`, `store_secret` requires `recovery_passphrase` unless `allow_unrecoverable`, and `unenroll()` deletes the enclave key and its KEK record. Construction fails closed with `Secure Enclave unavailable: the process needs a keychain-access-groups entitlement (errSecMissingEntitlement); code-sign the binary or run inside an app bundle` whenever the process is not entitled for the Data Protection Keychain — which includes every unbundled CLI and `cargo test` (AMFI kills even a Developer-signed unbundled binary that carries `keychain-access-groups` without a provisioning profile, error -413). The positive path therefore requires a provisioned `.app` bundle and is covered only by the two `#[ignore]` tests in `tests/secure_enclave.rs`; the fail-closed and option-policy tests run in CI on `macos-latest`.
- **`SecureEnclaveSecretStorageProvider::validate_store_options(&StorageOptions) -> Result<()>`**: the pure policy check `store_secret` applies (`StorageOptions.passphrase` rejected; `recovery_passphrase` required unless `allow_unrecoverable`), exposed so callers and tests can evaluate it without an enclave.
- **Secondary recovery envelope (`knishio:recovery:`)**: `SecretStorageProvider` adds `recover_secret` for re-enrolling master secrets from a secondary recovery envelope sealed under a user-held `recovery_passphrase`. `StorageOptions` gains `recovery_passphrase: Option<Zeroizing<String>>` and `allow_unrecoverable: bool`. Implemented across `AesGcmSecretStorageProvider`, `MemorySecretStorageProvider`, `OsKeychainSecretStorageProvider`, and `Tpm2SecretStorageProvider`: when a recovery passphrase is provided, `store_secret` seals a software AES-GCM recovery envelope under `knishio:recovery:<bundleHash>`, `delete_secret` removes both primary and recovery records, and `list_secrets` excludes recovery keys.
- **TPM 2.0 PCR policy authorization (`Tpm2Policy`)**: `Tpm2SecretStorageProvider::new` accepts an optional `Tpm2Policy` specifying PCR bank and slot indices (default PCR 7) with optional auth. Sealing calculates the PCR policy digest via a trial auth session (`SessionType::Trial`) and passes it to `with_auth_policy`, while unsealing satisfies the policy using an active `PolicySession`. Policy parameters are preserved in the sealed record JSON at `knishio:kek:tpm2:<alias>`.
- **`envelope::seal` and `envelope::open`**: custody-agnostic functions for PBKDF2/AES-GCM-256 envelope encryption, extracted from `AesGcmSecretStorageProvider` so providers share one implementation.
- **`FileStorageBackend`**: atomic file-based key-value persistence backend with 0o600 file mode permissions on Unix.
- **`OsKeychainSecretStorageProvider`** (`keyring` feature): secret storage provider backed by platform credential store (macOS Keychain, Linux Secret Service, Windows Credential Manager).
- **`Tpm2SecretStorageProvider`** (`tpm` feature): TPM 2.0 hardware-enclave secret storage provider sealing a random device passphrase into a KeyedHash object under a deterministic ECC P-256 storage primary.
### Changed

- **BREAKING:** **`hardware_backed` is no longer a caller claim.** `AesGcmSecretStorageProvider::new` drops its third argument, `provider_type()` is always `aes-gcm` (it previously relabelled itself `tpm2-aes-gcm` on the flag alone), and `is_hardware_backed()` is always `false`. Envelopes previously written with a caller-supplied `true` were never attested and remain readable. Source-level break for callers that passed the option; the wire format (`metadata.hardwareBacked`, required boolean) is unchanged.
- **BREAKING:** **Fallible `StorageBackend` trait**: `get_item`, `set_item`, `remove_item`, and `keys` now return `crate::error::Result<T>`, enabling proper I/O and lock-poisoning error propagation for file, keychain, and hardware backends.
- **Optional metadata keys omitted when unset**: `SecretStorageMetadata` marks `label` with `#[serde(default, skip_serializing_if = "Option::is_none")]`, omitting unset optional keys from emitted envelope JSON instead of emitting `"label": null`, matching the cross-SDK convention.
- **BREAKING:** **Durable storage backend required on hardware providers**: `OsKeychainSecretStorageProvider::new` and `Tpm2SecretStorageProvider::new` now require `backend: Arc<dyn StorageBackend>` instead of accepting `Option<Arc<dyn StorageBackend>>`. The KEK is durable; the backend must be too. Callers must supply an explicit storage backend rather than relying on an ephemeral in-memory default.
- **Mandatory recovery on TPM PCR policy**: When `Tpm2Policy` is configured on `Tpm2SecretStorageProvider`, `store_secret` fails closed unless `recovery_passphrase` is supplied or `allow_unrecoverable: true` is explicitly opted into, preventing permanent identity loss across firmware updates or PCR drift.
- **BREAKING:** `SecretStorageProvider` gains the required trait method `recover_secret(bundle_hash, recovery_passphrase, options)`; third-party implementations of the trait must add it (every in-tree provider implements it).
### Fixed

- **Fail-closed TPM custody check**: `tpm2::tpm_identity` classifies connected TPMs into `TpmIdentity::{Hardware, Software, Unknown}` by querying `PropertyTag::Manufacturer` and `VendorString1..4`. Unreadable properties or unexpected errors fail closed to `Unknown` (never claiming hardware custody). Known software signatures (`IBM ` + `SW*` for swtpm/libtpms, `MSFT` simulator strings) are classified as `Software`. Only verified non-software TPMs with readable identity properties evaluate to `hardware_backed = true`.

## [1.0.0] — 2026-09-10

### Added

- **A wallet configured at ML-KEM-1024 now decrypts records addressed to its own ML-KEM-768
  identity**, and vice versa. The 64-byte ML-KEM seed is derived from the Knish.IO wallet key
  and takes no parameter-set input, so `Wallet::decrypt_message` dispatches on the ciphertext's
  decoded length and derives the matching identity on demand. Reading records a pre-1.0 client
  wrote needs no configuration change. The derived private key is zeroized and dropped inside
  the call — it is never stored on the wallet — and encapsulation and the advertised public key
  are unchanged and remain single-set: inbound is permissive, outbound stays strict.
- `Wallet::hash_share` and `Wallet::decrypt_my_message_ml`: the post-quantum `CipherHash` map
  key (`Base64(SHAKE256(pubkey, 8 bytes))`) and the map-addressed inbound path, matching the
  Rust validator's `hash_share` and the other SDKs' `hashShare`/`decryptMyMessageML`. The
  lookup tries the configured identity's share first and then the other parameter set's, so an
  envelope a pre-1.0 sender addressed to `hashShare(our_768_pubkey)` is still found. Returns the
  raw decrypted text rather than a parsed value.
- `Wallet::mlkem_decrypt_to_string` — ML-KEM decapsulation plus AES-256-GCM decryption to the
  raw plaintext string, for callers that need the response text rather than parsed JSON.
- `Wallet::mlkem_parameter_set_from_pubkey` — the parameter set implied by a serialized public
  key's raw byte length (1568 → ML-KEM-1024, 1184 → ML-KEM-768).
- **Backwards-compatibility tests** (`tests/cross_platform_vectors.rs`): a build at the default
  parameter set decrypts the frozen ML-KEM-768 envelope from
  `cross-platform-test-vectors.json`, finds a 768-addressed `CipherHash` envelope, still rejects
  a ciphertext matching neither set, and validates a frozen pre-1.0 ML-KEM-768 auth molecule —
  both its molecular hash and, through `Molecule::check`, its WOTS+ signature.
- **Cross-SDK envelope regression tests** (`tests/secret_storage.rs`): a frozen envelope produced
  by the TypeScript SDK 0.9.7 is decrypted here, a frozen 0.9.5 snake_case envelope proves the
  back-compat aliases, and the emitted key set is asserted against the peer-SDK contract. The
  first two fail against 0.9.5's serialization.

### Changed

- **ML-KEM-1024 by default with optional ML-KEM-768 step-back.** Added `MlKemParameterSet` enum
  (`MlKem1024` default, `MlKem768`) with accessor methods `pk_bytes()`, `sk_bytes()`, `ct_bytes()`.
- **Breaking API change:** `Wallet::new` and `Wallet::create` now accept an optional
  `mlkem_parameter_set: Option<MlKemParameterSet>`.
- Client and molecule operations now preserve the configured ML-KEM parameter set.
- Encapsulation is strict: `Wallet::encrypt_message` rejects a recipient public key whose length
  does not match the wallet's own parameter set rather than silently downgrading.
- **MSRV raised 1.75 → 1.89**, required by the RustCrypto 2026 line (`aes` 0.9.3). CI builds
  on rolling stable.
- **RustCrypto 2026 line**: `sha3` 0.10 → `shake` 0.1 (`sha3` 0.12 moved `Shake256` into its
  own crate; the API is unchanged), `sha2` 0.10 → 0.11, `aes-gcm` 0.10 → 0.11, `aes` 0.8 → 0.9,
  `pbkdf2` 0.12 → 0.13, `base64` 0.22 → 0.23. `aes-gcm` 0.11 replaced `generic-array` with
  `hybrid-array`, so nonces are constructed via `Nonce::from`/`Nonce::try_from` and AEAD
  methods take `&Nonce`. No cryptographic output changed.
- **`reqwest` 0.12 → 0.13**, whose `rustls-tls` feature was renamed `rustls`.
- **`rand` 0.9.3 → 0.10.2 and `num-bigint` 0.4 → 0.5.** In `rand` 0.10, `RngCore` was renamed
  `Rng` and the former `Rng` methods moved to `RngExt`.

### Fixed

- **A session snapshot now records its ML-KEM parameter set, and `AuthToken::restore` honours
  it.** `WalletSnapshot` gained `mlkem_parameter_set: Option<MlKemParameterSet>`, and `restore`
  resolves the set in three tiers: the snapshot's explicit value, else the set implied by the
  stored `pubkey`'s length, else ML-KEM-768. A session persisted by an 0.9.x build carries
  neither, and previously restored at the constructor default — which is now ML-KEM-1024 — so
  the restored wallet advertised a public key the validator never recorded for that token and
  outbound encryption failed against the stored 1184-byte validator key. The field is an
  `Option` so that an absent parameter set stays distinguishable from an explicit one; the
  fallback is deliberately ML-KEM-768 rather than the default, because a snapshot with neither
  marker can only have come from a 768-only build.
- **Cross-SDK envelope interoperability** (`src/storage/mod.rs`): `SecretStorageMetadata` and
  `EncryptedSecretPayload` now serialize their metadata keys as camelCase
  (`bundleHash`, `createdAt`, `hardwareBacked`, `providerType`), matching the TypeScript,
  JavaScript, and Kotlin providers that share this wire format. 0.9.5 emitted snake_case, so an
  envelope produced by `@wishknish/knishio-client-ts@0.9.7` failed here with
  `missing field bundle_hash` before decryption was attempted, and an envelope produced here
  deserialized in TypeScript with every typed metadata field `undefined`. The AES-256-GCM /
  PBKDF2-HMAC-SHA256 core was always identical across all four SDKs; only the JSON framing
  diverged. Envelopes written by 0.9.5 remain readable via `serde` field aliases.
  0.9.5 shipped this envelope as a format shared with the other SDKs; that was true of its
  crypto core and of molecular output (its self-test cross-validated 7/7 peers), but false of
  the envelope's JSON framing, which no shared vector covered at the time.

### Notes

- `0.10.0` was staged in `Cargo.toml` and in this changelog but was never tagged or published to
  crates.io; its entry became this one rather than being kept for a version that never shipped.
  The ML-KEM-1024 cutover is a breaking API change, so it takes the 1.0.0 line.
- Nothing on the wire changed and no algorithm identifier was added. The parameter set is
  recoverable from FIPS 203's disjoint key and ciphertext lengths, and molecular hashing treats
  `walletPubkey` as an opaque meta string, so no molecular hash changed and no migration is
  required.
- This SDK has no `CipherHash` request transport, so `decrypt_my_message_ml` is the inbound half
  of that contract only; `Wallet::encrypt_message` remains the single outbound entry point.

## [0.9.5] — 2026-09-04

### Added

- **Hardware Envelope Encryption & Secure Memory Provider**: Introduced `SecretStorageProvider`,
  `SecretStorageMetadata`, `EncryptedSecretPayload`, `StorageOptions`, and `StorageBackend` contracts
  (`src/storage/mod.rs`).
- **AES-GCM Envelope Encryption Provider** (`src/storage/aes_gcm.rs`): AES-256-GCM envelope encryption
  with PBKDF2-HMAC-SHA256 (100,000 iterations) key derivation, 12-byte random IV (`rand::RngCore`),
  16-byte random salt, pluggable `StorageBackend` (TPM NVRAM / OS Keyring ready), and auto-zeroized
  byte buffers.
- **In-Memory Storage Provider** (`src/storage/memory.rs`): Thread-safe in-memory fallback using
  `Arc<RwLock<HashMap>>` with synchronous `store_secret_sync()` for seamless client initialization.
- **Memory Hygiene & Zeroization Utilities** (`src/storage/secure_memory.rs`): Explicit buffer clearing
  (`zeroize_bytes`), scoped execution with RAII drop guards (`with_secure_bytes`, `with_secure_string`),
  and timing-safe comparison (`constant_time_equals`).
- **KnishIOClient Secret Storage Integration**: `KnishIOClient` accepts `secret_storage`, provides
  `set_secret_storage()`, `get_secret_storage()`, and `retrieve_secret()`, and unwraps the master secret
  just-in-time for molecule construction (`create_molecule()`) and auth token refresh (`request_auth_token()`)
  without permanently retaining cleartext secrets in client heap memory.
- **Error Types**: Added `SecretStorage`, `SecretNotFound`, `DecryptionFailed`, and `StorageUnavailable`
  variants to `KnishIOError` (`src/error/mod.rs`).
- **`pq_line_rate` benchmark** (`benches/pq_line_rate.rs`, `ring` dev-dependency): ML-KEM-768 KEM
  operations, AES-256-GCM data-plane throughput (hardware-accelerated and pure-software), and
  `Wallet` per-message encapsulation envelope throughput, with 1/10 Gbps line-rate CPU sizing.

## [0.9.4] — 2026-08-05

### Added

- `isotope_b()` and `isotope_f()` in `src/check_molecule.rs` enforce conservation over the
  combined V+B and V+F sets. `isotope_v()` skips the V-only sum whenever B or F atoms are
  present, and until these existed nothing enforced conservation in its place — the
  `has_cross_isotope` gate is keyed on B *or* F, so an F-isotope molecule skipped V-only
  conservation with no replacement.
- `isotope_p()` and `isotope_a()` validation.

### Changed — cross-SDK gauntlet reporting integrity

- The self-test now publishes cross-validation **coverage**, not just a verdict:
  `crossValidation.{ran,targetsExpected,targetsValidated}` and `runId` sit alongside
  `crossSdkCompatible` in the results file. The boolean alone could not distinguish
  "validated every peer, all passed" from "validated nothing and so found no failures".
- `crossSdkCompatible` now defaults to **false** and must be earned. It was `true`, so every
  early return out of cross-validation published a pass.
- Cross-validation **fails** instead of reporting "compatible" when the shared results
  directory is missing or holds no peer results. Absence of evidence is not evidence of
  compatibility.
- Round 1 no longer asserts a cross-SDK verdict it cannot have.
- A coverage floor is required before a pass: every expected peer must have been validated,
  in addition to no individual check having failed.
- Each peer is now checked for all 7 required molecule types. The validation loop iterates
  the molecule keys that are **present**, so an omitted molecule was indistinguishable from
  a validated one.
- Peer results are matched with `*-results.json`. ``ends_with(".json")`` also matched the canonical vector
  **masters** living in that directory and fed them into the peer loop as SDK results.

Contract for these fields: `sdks/canonical-test-keys.json` in the KnishIOClientSDK
monorepo. Audit: `docs/audits/REPORTING-INTEGRITY-2026-08-05.md`.

## [0.9.3] — 2026-07-24

### Security

- `libcrux-ml-kem` 0.0.9 → 0.0.10, clearing RUSTSEC-2026-0207, RUSTSEC-2026-0208,
  and RUSTSEC-2026-0212. Fixed at the source crate so every downstream consumer
  of `knishio-client` clears the advisories by bumping this SDK. `cargo audit`
  exits 0 on a fresh resolution.

## [0.9.2] — 2026-07-12

Coordinated dependency-security release across all 8 SDKs. Release record:
`docs/sdk-release-0.9.2-execution-2026-07-12.md` (monorepo).

### Security

- Cleared 8 RUSTSEC advisories, most importantly by moving to `libcrux-ml-kem`
  0.0.9 — which drops the `libcrux-sha3` 0.0.7 incorrect-SHAKE advisory
  (RUSTSEC-2026-0074), the one crates.io consumers could not otherwise escape.

### Added

- `cargo audit` job in CI (fresh `cargo generate-lockfile`, since `Cargo.lock`
  is not committed for a library).
- Tag-driven publish workflow using crates.io Trusted Publishing (OIDC);
  `CARGO_REGISTRY_TOKEN` dropped. The publish job runs in the `release` GitHub
  environment.

### Fixed

- Three clippy 1.97 lints in pre-existing code (`question_mark`,
  `useless_borrows_in_formatting`) that broke the rolling-`@stable` CI gate.

### Notes

- `0.9.1` was staged in `Cargo.toml` on 2026-06-30 (encrypt-guard parity: a clear
  error when a node advertises a non-ML-KEM recipient key) but was never tagged
  and never published to crates.io. That change ships in `0.9.2`.

## [0.9.0] — 2026-06-29

Coordinated `0.9.0` across all 8 SDKs, marking the post-quantum ML-KEM transport
milestone. Runbook: `docs/sdk-release-audit-2026-06-29.md` (monorepo).

Version jumped `0.2.x` → `0.9.0` to put the whole SDK ecosystem on one version
line; this is a numbering alignment, not a signal of a larger API break than the
changes listed here.

### Added

- Multi-recipient stackable (NFT) transfer builder (WP-544).
- `mlkem768` keygen + decrypt cross-SDK vector, and a "decrypt their message"
  ML-KEM768 cross-validation against the sibling SDKs.
- Regression locks for the c54 `cell_slug` NULL-decode and c93 ContinuID bugs,
  and a `buffer_deposit_conservation` lock vector.
- First CI workflow for this repo (clippy gate) — the gate had been local-only
  and bypassable until now.

### Fixed

- Buffer withdraw now debits the full source balance (UTXO conservation).
- `claim_shadow_wallet` repaired end to end (OTS reuse, cell clobber, batch id).
- Burn rebuilt as a canonical 3-atom zero-sum molecule; `tokenUnits` carried on
  stackable-burn V-atoms.
- `add_stackable_transfer` moves units (`split_units` + `tokenUnits` meta), and
  `init_value` V-atoms emit `tokenUnits`.
- `init_token_creation`, `init_wallet_creation`, and `init_shadow_wallet_claim`
  reconciled to the JS reference; `set_meta_wallet` is order-preserving.
- Value conservation enforced for plain 2-atom V transfers.
- The live request/response, query, and transfer-signing paths made
  live-consistent (`create_token`, `query_atom`).
- The auth ContinuID remainder is registered explicitly.
- All 34 failing doctests fixed — `cargo test` is fully green.

### Removed

- Dead `QueryUserActivity` query.

### Notes

- Local version `0.2.3` was staged on 2026-06-15 and never published; that work
  reaches consumers here.

## [0.2.2] — 2026-06-08

### Fixed

- `init_deposit_buffer` debits the full source balance (UTXO conservation).

### Changed

- Cross-platform test vectors consolidated onto the shared canonical master.

## [0.2.1] — 2026-06-05

Published to crates.io; no corresponding git tag exists in this repository.

### Fixed

- `cargo test --lib` no longer hangs: `warm_up_simd` took a re-entrant lock on a
  non-reentrant mutex. Also fixed a NaN parity discrepancy. Suite green, 469/0.

## [0.2.0] — 2026-06-04

Published to crates.io; no corresponding git tag exists in this repository.

### Changed

- **BREAKING:** `generate_secret` now outputs the canonical 2048-hex secret
  (previously 1024). The 1024 output was a prefix of the 2048 one, so derived
  bundle hashes change for callers that relied on the old length.

### Fixed

- `verify_ots_signature` and the WOTS+ test vector both use the two-pass
  protocol OTS-address derivation.

## Earlier releases

`0.1.0`–`0.1.2` predate this project's conventional-commit discipline; their
commit messages do not support accurate reconstruction. See the git history and
the [crates.io version list](https://crates.io/crates/knishio-client/versions).

[Unreleased]: https://github.com/WishKnish/KnishIO-Client-Rust/compare/1.1.0...HEAD
[1.1.0]: https://github.com/WishKnish/KnishIO-Client-Rust/releases/tag/1.1.0
[1.0.0]: https://github.com/WishKnish/KnishIO-Client-Rust/releases/tag/1.0.0
[0.9.5]: https://github.com/WishKnish/KnishIO-Client-Rust/releases/tag/0.9.5
[0.9.4]: https://github.com/WishKnish/KnishIO-Client-Rust/releases/tag/0.9.4
[0.9.3]: https://github.com/WishKnish/KnishIO-Client-Rust/releases/tag/0.9.3
[0.9.2]: https://github.com/WishKnish/KnishIO-Client-Rust/releases/tag/0.9.2
[0.9.0]: https://github.com/WishKnish/KnishIO-Client-Rust/releases/tag/0.9.0
[0.2.2]: https://github.com/WishKnish/KnishIO-Client-Rust/releases/tag/0.2.2
[0.2.1]: https://crates.io/crates/knishio-client/0.2.1
[0.2.0]: https://crates.io/crates/knishio-client/0.2.0
