//! Links the static KnishIO-Crypto-Core archive from an extracted release package.
//!
//! `KNISHIO_KCORE_DIR` names the package root (the directory holding `include/kcore.h` and
//! `lib/libkcore.a`). There is no fallback search: a build that asked for kcore and cannot find
//! it stops here instead of producing a binary that silently runs the pure-Rust path. The
//! package must be a 0.2.x release (`KCORE_VERSION_MAJOR`/`MINOR` in `include/kcore.h`).
use std::path::{Path, PathBuf};

const MISSING: &str =
    "KNISHIO_KCORE_DIR must point at an extracted KnishIO-Crypto-Core 0.2.0 package (lib/libkcore.a)";

/// The value of `#define <name> <n>` in `header`, if present.
fn define(header: &str, name: &str) -> Option<u32> {
    header.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        (words.next() == Some("#define") && words.next() == Some(name))
            .then(|| words.next()?.parse().ok())
            .flatten()
    })
}

fn check_version(dir: &Path) {
    let path = dir.join("include").join("kcore.h");
    println!("cargo:rerun-if-changed={}", path.display());
    let header = std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("{MISSING}"));
    let found = (
        define(&header, "KCORE_VERSION_MAJOR"),
        define(&header, "KCORE_VERSION_MINOR"),
    );
    if found != (Some(0), Some(2)) {
        let shown = match found {
            (Some(major), Some(minor)) => format!("{major}.{minor}"),
            _ => "an unknown version".to_string(),
        };
        panic!("{MISSING}; {} is {shown}", path.display());
    }
}

fn main() {
    // docs.rs builds documentation only and has no kcore package; skip linking there.
    println!("cargo:rerun-if-env-changed=DOCS_RS");
    if std::env::var_os("DOCS_RS").is_some() {
        return;
    }
    println!("cargo:rerun-if-env-changed=KNISHIO_KCORE_DIR");

    let dir = match std::env::var_os("KNISHIO_KCORE_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => panic!("{MISSING}"),
    };
    // Canonicalize so a relative KNISHIO_KCORE_DIR means the same directory to rustc's linker
    // invocation as it does here.
    let lib = match dir.join("lib").canonicalize() {
        Ok(lib) if lib.join("libkcore.a").is_file() => lib,
        _ => panic!("{MISSING}"),
    };
    check_version(&dir);

    println!("cargo:rerun-if-changed={}", lib.join("libkcore.a").display());
    println!("cargo:rustc-link-search=native={}", lib.display());
    println!("cargo:rustc-link-lib=static=kcore");
}
