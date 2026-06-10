# `osno2` System Architecture Specification

This document details the architectural layout of `osno2` as a native, single-purpose terminal music appliance. By combining **WezTerm** as the cross-platform window host with **Crossterm** and **Ratatui** for terminal rendering, the application delivers a dedicated appliance window on Windows, macOS, and Linux without GPU pipeline ownership or custom terminal emulator code.

---

## 1. The Launcher/Worker Process Topology

`osno2` uses a self-forking executable architecture. The binary shifts its runtime behavior entirely based on the presence of the internal flag `--worker`.

```
+-----------------------------------+
|        User types "osno2"         |
+-----------------------------------+
                |
                v
+-----------------------------------+
|          LAUNCHER MODE            |
|     (Validates environment)       |
+-----------------------------------+
                |
 Spawns: wezterm start --no-auto-connect -- osno2 --worker
                |
                v
+-----------------------------------------------------+
|               NATIVE WEZTERM WINDOW                 |
|  - Cross-platform: Windows / macOS / Linux          |
|  - Configured geometry, font, and no tab chrome     |
+-----------------------------------------------------+
                |
                v
+-----------------------------------+
|           WORKER MODE             |
|   (Locks raw TUI & Boots Engine)  |
+-----------------------------------+
```

### The Launcher Lifecycle

1. The user executes the `osno2` binary from any shell.
2. The launcher checks for `--worker` absence and resolves the `wezterm` executable on `PATH`. If WezTerm is not found, it falls back to raw terminal mode with a warning.
3. It spawns a detached OS process: `wezterm start --no-auto-connect -- <path_to_current_exe> --worker`.
4. The parent launcher exits immediately, freeing the user's host shell.

**WezTerm window configuration** is driven by a bundled `wezterm.lua` config or inline CLI overrides (font size, window dimensions, no tab bar, no scroll bar). This config is written to the OS config directory on first run if absent.

### The Worker Lifecycle

1. A new, detached WezTerm window instantiates.
2. The window runs the `osno2` binary with `--worker`.
3. The worker activates Crossterm raw mode, issues `EnterAlternateScreen`, and takes absolute ownership of stdin/stdout. A `Drop` guard ensures clean teardown on both normal exit and panic.

### Fallback Mode

If `wezterm` is not on `PATH`, the launcher skips window spawning and runs the worker inline in the current terminal. A status line notes the degraded context. This keeps the application functional in CI, SSH sessions, or minimal environments.

---

## 2. Multi-Threaded Core Processing Layers

The backend is segregated into three operational boundaries communicating across asynchronous pipelines and lock-free channels to ensure rendering calculations never starve the audio pipeline.

```
+-----------------------+              +-----------------------+
|   1. MAIN TUI THREAD  |              |   2. TOKIO RUNTIME    |
| - Ratatui Frame Loop  |              | - File System I/O     |
| - Crossterm Inputs    | <=========>  | - TOML Parse Worker   |
| - Layout Constraints  |  Async/Sync  | - Local SQLite/Index  |
+-----------------------+   Channels   +-----------------------+
            ^                                      ||
            || Sync Channel (FFT Stream)           || Sync Channel
            v                                      v
+--------------------------------------------------------------+
|                     3. CORE AUDIO ENGINE                     |
| - CPAL Callback Thread (High Priority Real-Time)             |
| - Symphonia Decoder Pipeline (MP3/FLAC/WAV Buffer Filling)   |
| - Dual-Channel DSP / Volume Vector Mix Matrix                |
+--------------------------------------------------------------+
```

### 1. Main UI Loop Thread

Responsible strictly for interface composition and layout. Running an immediate-mode drawing pattern, it queries internal state via **Ratatui** at up to 60 FPS. Double-buffered frame diffs target only changed terminal matrix coordinates, eliminating flicker during heavy text transformations.

### 2. Async I/O Core (Tokio Runtime)

Spawns background tasks for asset discovery: music directory scanning, track tree indexing, config loading, and database writes. Outputs verified indexing structs to the runtime database and forwards commands to the audio layer.

### 3. Core Audio Engine (CPAL + Symphonia)

A dedicated, real-time OS-priority thread. Continuously feeds a ring buffer via Symphonia's decoder pipeline, mixes volume vectors, maintains sample tracking, and extracts short-term frames for FFT processing.

---

## 3. Sandboxed REPL and Command Layout

Crossterm intercepts all keyboard input before the host shell sees it. The interface functions as a secured input container.

- **Control Interception:** `Ctrl+C`, `Ctrl+Z`, and other standard signals are handled explicitly by the application's input dispatch. The user cannot drop to a shell or invoke host commands.
- **Clap Integration:** The command field passes typed input directly into a `clap::Parser` instance matching strict subcommands:

| Command Sequence   | Target System Component      | Parameter Scope                              |
|--------------------|------------------------------|----------------------------------------------|
| `/play <query>`    | Audio Engine Track Selector  | Fuzzy string query or exact UID hash         |
| `/volume <level>`  | DSP Audio Gain Matrix        | Numeric scalar (`0` to `100`)                |
| `/queue <query>`   | Playlist Tracking Log        | Appends asset paths to the active play stream|
| `/purge`           | SQLite/TOML Library Index    | Wipes indexed catalog from disk              |

---

## 4. Local File Structure Configuration

Data is stored in OS-standard directories via the `directories` crate. On first launch, missing config files are scaffolded with defaults. The WezTerm window config is written here if not already present.

```
~/.config/osno2/
├── config.toml           # User preferences, themes, path arrays, hotkeys
├── wezterm.lua           # Bundled WezTerm window config (written on first run)
├── library/
│   ├── index.db          # Embedded catalog mapping database (redb)
│   └── meta/             # Individual human-readable track metadata snapshots
│       ├── <id_1>.toml
│       └── <id_2>.toml
└── playlists/
    ├── default.toml      # Active playback array
    └── cyberpunk.toml    # Saved play state
```

---

## 5. Production Blueprint: `Cargo.toml`

```toml
[package]
name = "osno2"
version = "0.1.0"
edition = "2021"
authors = ["Osno Dev Team"]
description = "Dedicated Cyberpunk Terminal Music Appliance"

[dependencies]
# Interface & Rendering
ratatui = { version = "0.30.1", features = ["crossterm", "macros", "all-widgets"] }
crossterm = { version = "0.29", features = ["event-stream"] }

# Async Infrastructure & Channel Communication
tokio = { version = "1.43", features = ["rt-multi-thread", "macros", "fs", "time", "sync"] }
crossbeam-channel = "0.5"

# Audio Core & Decoding
cpal = "0.15"
symphonia = { version = "0.5", features = ["all"] }
rustfft = "6.2"

# Command Pipeline & Structured Data
clap = { version = "4.5", features = ["derive", "string"] }
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
directories = "5.0"

# Local Storage Engine
redb = "2.0"
sha2 = "0.10"

# WezTerm process spawning & PATH resolution
which = "6.0"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
```

---

## 6. State Management & Command Dispatch Pipeline

The application adheres to a strict unidirectional data flow where the root `App` struct acts as the central coordinator between user actions, interface state (`ViewState`), and internal indexing engines.

```
┌─────────────────────────────────────────────┐
│                   App (UI)                  │
│  holds ViewState, sends Commands            │
└────────────────┬────────────────────────────┘
                 │  queries via
                 ▼
┌─────────────────────────────────────────────┐
│                LibraryService               │
│  single owned struct, lives on App          │
│  exposes a clean query/command API          │
│  owns the redb handle + TOML meta path      │
└────────────┬──────────────┬─────────────────┘
             │              │
             ▼              ▼
┌────────────────┐  ┌───────────────────┐
│   index.db     │  │    meta/*.toml    │
│  (redb fast)   │  │  source of truth  │
└────────────────┘  └───────────────────┘
```

### Command Routing Architecture (`App::execute_command`)

When a shell command or user interaction occurs, it funnels exclusively through `App::execute_command(&str)`. Execution splits into two tracks:

**Filesystem Traversal Context:** Commands like `cd` or `ls` manipulate ephemeral directory views. They call `FsReader::read_dir(path)` directly, loading a transient vector into `ViewContent::Filesystem` without touching the database.

**Library Management Context:** Library ingestion or catalog lookup operations (`add`, `remove`, `list_all`, `search`) route to `LibraryService`.

- Broad queries scan `index.db` (redb) for fast matching.
- Specific catalog detail loading hydrates from individual `meta/<id>.toml` files.
- Resolved records are loaded into `ViewContent::Tracklist` to trigger a localized Ratatui re-draw.

---

## 7. Runtime Cache Validation & Snapshot Resumption

To maintain synchronization when track parameters or TOML configurations are edited externally, `osno2` executes a deterministic runtime validation loop.

### Checksum Verification

Every file under the `meta/` directory undergoes a validation pass:

1. The background scanner reads the file content byte array.
2. It computes a SHA-256 checksum of the file payload.
3. The resulting hash is compared against the `[u8; 32]` entry stored alongside the track metadata in the redb index.

### Fault-Tolerant State Restoration

If the computed hash conflicts with the stored index hash, the file buffer is submitted to the parser pipeline for live index update reconciliation. If the parser catches a structural deserialization failure (e.g., malformed TOML from an external edit):

- `LibraryService` handles the `Err` gracefully without dropping runtime frames or triggering thread panics.
- The cache mutation cycle is aborted for that specific asset, and the application resumes from the last-known good snapshot stored in the `.db` record. This preserves playlist continuity and interface integrity.
