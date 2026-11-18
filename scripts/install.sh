#!/usr/bin/env bash
# ORDYN Toolchain Installer for macOS & Linux
set -e

echo "=== Installing ORDYN Toolchain (v0.1.0 Sovereign) ==="

# Check for Rust toolchain
if command -v cargo >/dev/null 2>&1; then
    echo "[1/2] Rust toolchain detected. Installing ordyn-cli directly via cargo..."
    cargo install --git https://github.com/M1k3lee/ORDYN.git ordyn-cli ordyn-lsp --locked
    echo "[2/2] ORDYN binaries installed successfully!"
else
    echo "[*] Rust/Cargo not found. Cloning repository to build..."
    TEMP_DIR=$(mktemp -d)
    git clone --depth 1 https://github.com/M1k3lee/ORDYN.git "$TEMP_DIR/ORDYN"
    cd "$TEMP_DIR/ORDYN"
    if command -v cargo >/dev/null 2>&1; then
        cargo build --release --bin ordyn
        mkdir -p ~/.local/bin
        cp target/release/ordyn ~/.local/bin/
        echo "[OK] Installed 'ordyn' to ~/.local/bin/ordyn"
    else
        echo "[ERROR] Please install Rust (https://rustup.rs) to build ORDYN from source."
        exit 1
    fi
fi

echo ""
echo "=== Installation Complete! ==="
echo "Run 'ordyn --help' or 'ordyn new my-app' to get started."
