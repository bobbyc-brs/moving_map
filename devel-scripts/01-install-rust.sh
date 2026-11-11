#!/usr/bin/env bash
# Purpose: install rustup and the Rust toolchain pinned in rust-toolchain.toml.
# Changes ~/.rustup, ~/.cargo and the shell profile (rustup adds ~/.cargo/bin
# to PATH). No sudo; leaves any apt-installed rustc alone. Re-runnable.
# Why not apt: Ubuntu's newest packaged rustc (1.82 on 24.10) is too old for
# the GUI crates, which need edition 2024 (Rust ≥ 1.85) — DD-010.
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
INSTALLER="$(mktemp)"
trap 'rm -f "$INSTALLER"' EXIT

echo "===== rustup ====="
if command -v "$HOME/.cargo/bin/rustup" >/dev/null; then
  echo "rustup already installed: $("$HOME/.cargo/bin/rustup" --version 2>/dev/null)"
else
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o "$INSTALLER"
  sh "$INSTALLER" -y --profile minimal --default-toolchain none
fi

echo "===== toolchain from rust-toolchain.toml ====="
cd "$REPO"
"$HOME/.cargo/bin/rustup" show active-toolchain || "$HOME/.cargo/bin/rustup" toolchain install
"$HOME/.cargo/bin/rustc" --version
"$HOME/.cargo/bin/cargo" --version

echo "===== PATH ====="
if [ "$(command -v cargo)" != "$HOME/.cargo/bin/cargo" ]; then
  echo "This shell still finds $(command -v cargo || echo 'no cargo')."
  echo "Open a new login shell, or run:  source \"\$HOME/.cargo/env\""
fi
