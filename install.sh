#!/usr/bin/env bash
set -e

# Flowrail 1-Command Installer
# Usage:
#   ./install.sh
# or remotely:
#   curl -fsSL https://raw.githubusercontent.com/imMamdouhaboammar/Flowrail/master/install.sh | bash

echo "=========================================================="
echo "  🚀 Installing Flowrail (Project Ariadne)"
echo "  Deterministic Execution & Mutation State-Machine"
echo "=========================================================="

# 1. Check for Rust / Cargo
if ! command -v cargo >/dev/null 2>&1; then
    echo "❌ Cargo not found! Flowrail requires Rust (2024 edition)."
    echo "Installing Rust via rustup.rs..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

SCRIPT_DIR=""
if [ -n "${BASH_SOURCE[0]:-}" ]; then
    SOURCE="${BASH_SOURCE[0]}"
    while [ -h "$SOURCE" ]; do
        DIR="$(cd -P "$(dirname "$SOURCE")" >/dev/null 2>&1 && pwd)"
        SOURCE="$(readlink "$SOURCE")"
        [[ $SOURCE != /* ]] && SOURCE="$DIR/$SOURCE"
    done
    SCRIPT_DIR="$(cd -P "$(dirname "$SOURCE")" >/dev/null 2>&1 && pwd)"
fi

if [ -n "$SCRIPT_DIR" ] && [ -f "$SCRIPT_DIR/Cargo.toml" ] && [ -d "$SCRIPT_DIR/crates/flowrail-cli" ]; then
    ROOT_DIR="$SCRIPT_DIR"
else
    # Remote install fallback: clone to temporary or ~/.flowrail-src
    INSTALL_DIR="${HOME}/.flowrail-src"
    echo "Cloning Flowrail repository into ${INSTALL_DIR}..."
    rm -rf "${INSTALL_DIR}"
    git clone --depth 1 https://github.com/imMamdouhaboammar/Flowrail.git "${INSTALL_DIR}"
    ROOT_DIR="${INSTALL_DIR}"
fi

ORIGINAL_PATH="$PATH"
CARGO_BIN_DIR="${CARGO_INSTALL_ROOT:-${CARGO_HOME:-$HOME/.cargo}/bin}"

echo "📦 Building Flowrail CLI in release mode..."
cargo install --path "${ROOT_DIR}/crates/flowrail-cli" --force

# Check if flowrail is already visible in caller's PATH
if ! command -v flowrail >/dev/null 2>&1; then
    export PATH="${CARGO_BIN_DIR}:${PATH}"

    if ! command -v flowrail >/dev/null 2>&1; then
        echo "Adding symlink to ~/.local/bin..."
        mkdir -p "${HOME}/.local/bin"
        ln -sf "${CARGO_BIN_DIR}/flowrail" "${HOME}/.local/bin/flowrail"
        export PATH="${HOME}/.local/bin:${PATH}"
    fi

    echo ""
    echo "⚠️  ${CARGO_BIN_DIR} is not in your original shell PATH."
    echo "To run flowrail directly from any terminal, add this to ~/.bashrc or ~/.zshrc:"
    echo "  export PATH=\"${CARGO_BIN_DIR}:\$PATH\""
fi

echo ""
echo "🔍 Running Flowrail Doctor..."
flowrail doctor

echo ""
echo "🎉 Flowrail is successfully installed!"
echo ""
echo "Quick Commands:"
echo "  flowrail demo           # Run instant live demonstration on ecommerce fixture"
echo "  flowrail init           # Scaffold .flowrail/ and configure MCP for Cursor & Claude Code"
echo "  flowrail search <query> # Fuzzy symbol search with Git status & frecency"
echo "  flowrail grep <query>   # In-memory definition grep"
echo "  flowrail mcp            # Start stdio MCP server"
echo ""
echo "To connect to Cursor or Claude Code, run in your TypeScript project:"
echo "  flowrail init"
echo "=========================================================="
