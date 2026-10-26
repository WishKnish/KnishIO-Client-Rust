#!/usr/bin/env bash
# This repo's CycloneDX SBOM — one definition shared by ci, audit, publish and local runs.
# Usage: bash .github/scripts/sbom.sh [OUT_DIR]   (default sbom-out)
# Needs cargo-cyclonedx 0.5.9 (cargo install cargo-cyclonedx --version 0.5.9 --locked).
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
OUT="${1:-sbom-out}"
case "$OUT" in ''|/) echo "SBOM FAIL refusing OUT_DIR '$OUT'" >&2; exit 1 ;; esac
VERSION="$(bash .github/scripts/check-version.sh | sed -n 's/^OK: version is \([^ ]*\).*/\1/p')"
[ -n "$VERSION" ] || { echo "SBOM FAIL could not read the version" >&2; exit 1; }
rm -rf "$OUT"; mkdir -p "$OUT/gen"
# Describes the committed Cargo.lock, every target platform, default features. The optional
# `kcore` feature links KnishIO-Crypto-Core at the consumer's build, so it is not listed.
rm -f sbom-gen-cargo*.json
cargo cyclonedx --format json --spec-version 1.5 --target all --override-filename sbom-gen-cargo
[ -f sbom-gen-cargo.json ] || { echo "SBOM FAIL cargo-cyclonedx wrote: $(ls sbom-gen-cargo* 2>/dev/null)" >&2; exit 1; }
mv sbom-gen-cargo.json "$OUT/gen/cargo.cdx.json"
python3 .github/scripts/sbom.py --name knishio-client-rust --version "$VERSION" --type library \
  --base "$OUT/gen/cargo.cdx.json" --out "$OUT"
