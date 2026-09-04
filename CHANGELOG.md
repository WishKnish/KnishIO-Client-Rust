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


## [Unreleased]

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

[Unreleased]: https://github.com/WishKnish/KnishIO-Client-Rust/compare/0.9.5...HEAD
[0.9.5]: https://github.com/WishKnish/KnishIO-Client-Rust/releases/tag/0.9.5
[0.9.4]: https://github.com/WishKnish/KnishIO-Client-Rust/releases/tag/0.9.4
[0.9.3]: https://github.com/WishKnish/KnishIO-Client-Rust/releases/tag/0.9.3
[0.9.2]: https://github.com/WishKnish/KnishIO-Client-Rust/releases/tag/0.9.2
[0.9.0]: https://github.com/WishKnish/KnishIO-Client-Rust/releases/tag/0.9.0
[0.2.2]: https://github.com/WishKnish/KnishIO-Client-Rust/releases/tag/0.2.2
[0.2.1]: https://crates.io/crates/knishio-client/0.2.1
[0.2.0]: https://crates.io/crates/knishio-client/0.2.0
