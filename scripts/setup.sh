#!/usr/bin/env bash
set -euo pipefail

echo -e "\033[36m// Initializing osno2 environment provisioning sequence...\033[0m"

# 1. Enforce dependencies
if ! command -v wezterm &> /dev/null; then
    echo -e "\033[33mWarning: WezTerm not found on PATH. osno2 will run in fallback terminal mode.\033[0m"
    echo -e "\033[33m         Install WezTerm for the dedicated appliance window experience.\033[0m"
    echo -e "\033[33m         https://wezfurlong.org/wezterm/installation.html\033[0m"
fi

if ! command -v cargo &> /dev/null; then
    echo -e "\033[31mError: Rust toolchain not found. Install via https://rustup.rs\033[0m"
    exit 1
fi

# 2. Determine target directories based on OS
case "$(uname -s)" in
    Linux*|Darwin*)
        CONFIG_DIR="$HOME/.config/osno2"
        ;;
    MINGW*|MSYS*|CYGWIN*)
        CONFIG_DIR="${APPDATA}/osno2"
        ;;
    *)
        CONFIG_DIR="$HOME/.config/osno2"
        ;;
esac

LIBRARY_DIR="$CONFIG_DIR/library"
PLAYLISTS_DIR="$CONFIG_DIR/playlists"

echo "Creating configuration directories..."
mkdir -p "$CONFIG_DIR" "$LIBRARY_DIR" "$PLAYLISTS_DIR"

# 3. Seed default configs without overwriting existing user data
if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    echo "Seeding default config to $CONFIG_DIR/config.toml"
    cp assets/default_config.toml "$CONFIG_DIR/config.toml"
else
    echo "Existing config detected. Skipping seed."
fi

if [ ! -f "$CONFIG_DIR/wezterm.lua" ]; then
    echo "Seeding WezTerm window config to $CONFIG_DIR/wezterm.lua"
    cp assets/wezterm.lua "$CONFIG_DIR/wezterm.lua"
else
    echo "Existing WezTerm config detected. Skipping seed."
fi

# 4. Scaffold dev environment
if [ ! -d "dev_env" ]; then
    echo "Creating dev_env sandbox..."
    mkdir -p dev_env/config dev_env/library dev_env/playlists
fi

echo -e "\033[32m// Environment sequence verified successfully.\033[0m"
