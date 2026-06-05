#!/usr/bin/env bash
set -euo pipefail

echo -e "\033[36m// Initializing osno2 environment provisioning sequence...\033[0m"

# 1. Enforce dependencies
if ! command -v ghostty &> /dev/null; then
    echo -e "\033[31mError: Ghostty terminal application must be installed on your PATH.\033[0m"
    exit 1
fi

# 2. Determine target directories based on OS specifications
CONFIG_DIR="$HOME/.config/osno2"
LIBRARY_DIR="$CONFIG_DIR/library"
PLAYLISTS_DIR="$CONFIG_DIR/playlists"

echo "Creating target configuration arrays..."
mkdir -p "$CONFIG_DIR"
mkdir -p "$LIBRARY_DIR"
mkdir -p "$PLAYLISTS_DIR"

# 3. Seed default configurations safely without overwriting user data
if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    echo "Seeding default asset structures to $CONFIG_DIR/config.toml"
    cp assets/default_config.toml "$CONFIG_DIR/config.toml"
else
    echo "Existing configuration array detected. Skipping configuration seed."
fi

echo -e "\033[32m// Environment sequence verified successfully.\033[0m"
