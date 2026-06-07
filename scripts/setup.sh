#!/usr/bin/env bash
set -euo pipefail

echo -e "\033[36m// Initializing osno2 environment provisioning sequence...\033[0m"

# 1. Enforce dependencies
if ! command -v wezterm &> /dev/null; then
    echo -e "\033[33mWarning: WezTerm not found on PATH. osno2 will run in fallback terminal mode.\033[0m"
    echo -e "\033[33m         Install WezTerm: https://wezfurlong.org/wezterm/installation.html\033[0m"
fi

if ! command -v cargo &> /dev/null; then
    echo -e "\033[31mError: Rust toolchain not found. Install via https://rustup.rs\033[0m"
    exit 1
fi

# 2. Determine platform config directory (mirrors ProjectDirs::from("", "", "osno2"))
case "$(uname -s)" in
    Linux*)
        CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/osno2"
        ;;
    Darwin*)
        CONFIG_DIR="$HOME/Library/Application Support/osno2"
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

echo "Platform config directory: $CONFIG_DIR"
echo "Creating production directories..."
mkdir -p "$CONFIG_DIR" "$LIBRARY_DIR" "$PLAYLISTS_DIR"

# 3. Seed production configs without overwriting existing user data
if [ ! -f "$CONFIG_DIR/config.toml" ]; then
    echo "Seeding default config to $CONFIG_DIR/config.toml"
    cp assets/default_config.toml "$CONFIG_DIR/config.toml"
else
    echo "Existing config detected. Skipping seed."
fi

if [ ! -f "$CONFIG_DIR/wezterm.lua" ]; then
    echo "Seeding WezTerm config to $CONFIG_DIR/wezterm.lua"
    cp assets/wezterm.lua "$CONFIG_DIR/wezterm.lua"
else
    echo "Existing WezTerm config detected. Skipping seed."
fi

# 4. Scaffold dev_env sandbox (mirrors debug AppPaths layout)
DEV_CONFIG="dev_env/config"
DEV_LIBRARY="dev_env/library"
DEV_PLAYLISTS="dev_env/playlists"

if [ ! -d "$DEV_CONFIG" ]; then
    echo "Creating dev_env sandbox..."
    mkdir -p "$DEV_CONFIG" "$DEV_LIBRARY" "$DEV_PLAYLISTS"
else
    echo "dev_env already exists. Skipping."
fi

# Seed dev wezterm config so the launcher works without a release build
if [ ! -f "$DEV_CONFIG/wezterm.lua" ]; then
    echo "Seeding WezTerm config to $DEV_CONFIG/wezterm.lua"
    cp assets/wezterm.lua "$DEV_CONFIG/wezterm.lua"
fi

if [ ! -f "$DEV_CONFIG/config.toml" ]; then
    echo "Seeding default config to $DEV_CONFIG/config.toml"
    cp assets/default_config.toml "$DEV_CONFIG/config.toml"
fi

echo -e "\033[32m// Environment sequence verified successfully.\033[0m"
