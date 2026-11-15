#!/usr/bin/env bash
set -e

# Flowrail 1-Command Quickstart Demo

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "=== Flowrail 1-Command Quickstart ==="
echo "Running system doctor..."
"${SCRIPT_DIR}/bin/flowrail" doctor

echo ""
echo "Running interactive refactoring state-machine demo..."
"${SCRIPT_DIR}/bin/flowrail" demo

echo ""
echo "To initialize MCP for your current editor (Cursor, Claude Code, Windsurf):"
echo "  ${SCRIPT_DIR}/bin/flowrail init"
