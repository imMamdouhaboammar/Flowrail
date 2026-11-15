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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd || echo "")"

if [ -n "$SCRIPT_DIR" ] && [ -f "$SCRIPT_DIR/Cargo.toml" ]; then
    ROOT_DIR="$SCRIPT_DIR"
else
    # Remote install fallback: clone to temporary or ~/.flowrail-src
    INSTALL_DIR="${HOME}/.flowrail-src"
    echo "Cloning Flowrail repository into ${INSTALL_DIR}..."
    rm -rf "${INSTALL_DIR}"
    git clone --depth 1 https://github.com/imMamdouhaboammar/Flowrail.git "${INSTALL_DIR}"
    ROOT_DIR="${INSTALL_DIR}"
fi

echo "📦 Building Flowrail CLI in release mode..."
cargo install --path "${ROOT_DIR}/crates/flowrail-cli" --force

# Ensure ~/.cargo/bin is in PATH for this session if not already
export PATH="${HOME}/.cargo/bin:${PATH}"

if ! command -v flowrail >/dev/null 2>&1; then
    echo "⚠️  ~/.cargo/bin is not in your current PATH."
    echo "Adding symlink to /usr/local/bin or ~/.local/bin..."
    mkdir -p "${HOME}/.local/bin"
    ln -sf "${HOME}/.cargo/bin/flowrail" "${HOME}/.local/bin/flowrail"
    export PATH="${HOME}/.local/bin:${PATH}"
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
