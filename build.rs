//! Build script (cycle 127): gate the cross-SDK parity tests on the shared fixtures.
//!
//! `tests/patent_vector_validation.rs` and `tests/cross_platform_vectors.rs` use
//! `include_str!("../../../sdks/shared-test-results/*.json")` — fixtures that live in
//! the monorepo (one level above this crate) and are ABSENT in a standalone CI checkout
//! of this SDK alone, where `include_str!` of a missing file is a COMPILE error (which
//! fails `cargo clippy --all-targets`). Set the `has_shared_fixtures` cfg when they
//! exist so those tests compile only in the monorepo and are cfg'd out (empty) in a
//! standalone checkout. This mirrors the JS `jest.config.cjs` / TS `vitest.config.ts`
//! fixture gates added the same cycle.
use std::path::Path;

fn main() {
    // Declare the custom cfg so rustc (1.80+) doesn't warn it's unexpected.
    // Single-colon form: the crate's MSRV is 1.75, and cargo rejects the `cargo::`
    // (1.77+) syntax below that MSRV. (On <1.80 cargo this instruction is an unknown
    // no-op; on CI's 1.96 it's honored.)
    println!("cargo:rustc-check-cfg=cfg(has_shared_fixtures)");

    // build.rs runs with the crate root as CWD; the monorepo fixtures are at
    // ../shared-test-results/ (crate root -> sdks/ -> shared-test-results/).
    let dir = Path::new("..").join("shared-test-results");
    let present = dir.join("canonical-patent-vectors.json").exists()
        && dir.join("cross-platform-test-vectors.json").exists();
    if present {
        // single-colon form: supported across all cargo versions
        println!("cargo:rustc-cfg=has_shared_fixtures");
    }

    println!("cargo:rerun-if-changed=../shared-test-results/canonical-patent-vectors.json");
    println!("cargo:rerun-if-changed=../shared-test-results/cross-platform-test-vectors.json");
}
