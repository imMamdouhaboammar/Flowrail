#!/usr/bin/env bash
set -e

# Flowrail 1-Command Quickstart Demo

SOURCE="${BASH_SOURCE[0]}"
while [ -h "$SOURCE" ]; do
  DIR="$(cd -P "$(dirname "$SOURCE")" >/dev/null 2>&1 && pwd)"
  SOURCE="$(readlink "$SOURCE")"
  [[ $SOURCE != /* ]] && SOURCE="$DIR/$SOURCE"
done
SCRIPT_DIR="$(cd -P "$(dirname "$SOURCE")" >/dev/null 2>&1 && pwd)"

echo "=== Flowrail 1-Command Quickstart ==="
echo "Running system doctor..."
"${SCRIPT_DIR}/bin/flowrail" doctor

echo ""
echo "Running interactive refactoring state-machine demo..."
"${SCRIPT_DIR}/bin/flowrail" demo

echo ""
echo "To install flowrail to PATH and initialize MCP for your editor:"
echo "  1. Run ./install.sh (or 'cargo install --path crates/flowrail-cli')"
echo "  2. Run 'flowrail init' in your TypeScript project"
