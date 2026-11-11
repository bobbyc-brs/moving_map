#!/usr/bin/env bash
# Purpose: copy every crate in Cargo.lock into vendor/ and pack it as
# vendor-<date>.tar.gz, so a machine without crates.io access can build
# offline (DD-010). Needs network; writes vendor/ and the tarball only.
# On the offline machine:  tar xzf vendor-*.tar.gz && cargo build --offline --config .cargo/vendor.toml
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
CARGO="$HOME/.cargo/bin/cargo"
STAMP="$(date +%Y%b%d)"
cd "$REPO"

echo "===== vendor ====="
"$CARGO" vendor --locked vendor > /dev/null
du -sh vendor

echo "===== pack ====="
tar czf "vendor-$STAMP.tar.gz" vendor
ls -lh "vendor-$STAMP.tar.gz"

echo "===== check: offline build from vendor/ ====="
"$CARGO" build --offline --config .cargo/vendor.toml
echo "Offline build OK."
