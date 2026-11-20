#!/usr/bin/env bash
# ORDYN Toolchain Installer for macOS & Linux
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/M1k3lee/ORDYN/main/scripts/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/M1k3lee/ORDYN/main/scripts/install.sh | bash -s -- --from-source

set -euo pipefail

REPO="M1k3lee/ORDYN"
INSTALL_DIR="${ORDYN_INSTALL_DIR:-$HOME/.local/bin}"
FROM_SOURCE=false

for arg in "$@"; do
    case "$arg" in
        --from-source|--build)
            FROM_SOURCE=true
            ;;
    esac
done

echo ""
echo "  ██████╗ ██████╗ ██████╗ ██╗   ██╗███╗   ██╗"
echo " ██╔═══██╗██╔══██╗██╔══██╗╚██╗ ██╔╝████╗  ██║"
echo " ██║   ██║██████╔╝██║  ██║ ╚████╔╝ ██╔██╗ ██║"
echo " ██║   ██║██╔══██╗██║  ██║  ╚██╔╝  ██║╚██╗██║"
echo " ╚██████╔╝██║  ██║██████╔╝   ██║   ██║ ╚████║"
echo "  ╚═════╝ ╚═╝  ╚═╝╚═════╝    ╚═╝   ╚═╝  ╚═══╝"
echo ""
echo "=== Installing ORDYN Toolchain ==="

mkdir -p "$INSTALL_DIR"

detect_target() {
    local os arch
    os="$(uname -s)"
    arch="$(uname -m)"

    case "$os" in
        Linux)
            case "$arch" in
                x86_64) echo "x86_64-unknown-linux-gnu" ;;
                *) echo "unsupported" ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64) echo "x86_64-apple-darwin" ;;
                arm64|aarch64) echo "aarch64-apple-darwin" ;;
                *) echo "unsupported" ;;
            esac
            ;;
        *)
            echo "unsupported"
            ;;
    esac
}

install_from_source() {
    echo "[*] Building ORDYN from source using Cargo..."
    if ! command -v cargo >/dev/null 2>&1; then
        echo "[ERROR] Cargo/Rust toolchain not found. Please install Rust from https://rustup.rs first."
        exit 1
    fi
    cargo install --git "https://github.com/${REPO}.git" ordyn-cli --locked --force
    echo "[OK] Installed 'ordyn' to Cargo bin directory ($(which ordyn 2>/dev/null || echo '~/.cargo/bin/ordyn'))"
}

TARGET="$(detect_target)"

if [ "$FROM_SOURCE" = true ] || [ "$TARGET" = "unsupported" ]; then
    install_from_source
else
    echo "[1/3] Detected platform: $TARGET"
    RELEASE_URL="https://github.com/${REPO}/releases/latest/download/ordyn-${TARGET}.tar.gz"
    TMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$TMP_DIR"' EXIT

    echo "[2/3] Fetching latest pre-built binary..."
    if curl -sSL -f "$RELEASE_URL" -o "$TMP_DIR/ordyn.tar.gz" 2>/dev/null; then
        tar -xzf "$TMP_DIR/ordyn.tar.gz" -C "$TMP_DIR"
        cp "$TMP_DIR/ordyn" "$INSTALL_DIR/ordyn"
        chmod +x "$INSTALL_DIR/ordyn"
        echo "[3/3] Successfully installed ordyn to $INSTALL_DIR/ordyn"
    else
        echo "[!] Pre-built binary not found on latest release. Falling back to Cargo install..."
        install_from_source
    fi
fi

# Ensure INSTALL_DIR is in PATH
if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
    echo ""
    echo "[NOTE] To use 'ordyn' from any terminal, add $INSTALL_DIR to your PATH:"
    echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    echo "  (Add this line to your ~/.bashrc or ~/.zshrc)"
fi

echo ""
echo "=== Installation Succeeded! ==="
echo "Verify with:"
echo "  ordyn --version"
echo "  ordyn new my_project --template web"
echo ""
