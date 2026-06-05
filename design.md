# `osno2` System Architecture Specification

This document details the architectural layout of `osno2` as a native, single-purpose terminal music appliance. By combining the native windowing capabilities of **Ghostty** with the immediate-mode terminal rendering framework **Ratatui**, the application runs inside an isolated environment without the overhead of raw graphics pipelines or native OS window management.

---

## 1. The Launcher/Worker Process Topology

To create the illusion of a dedicated application window while remaining a true terminal user interface (TUI), `osno2` leverages a self-forking executable architecture. The binary shifts its runtime behavior entirely based on the presence of the internal command-line flag: `--worker`.

```
                  +-----------------------------------+
                  |        User types "osno2"         |
                  +-----------------------------------+
                                    |
                                    v
                  +-----------------------------------+
                  |          LAUNCHER MODE            |
                  |     (Validates environments)      |
                  +-----------------------------------+
                                    |
            Executes ghostty -e osno2 --worker and exits
                                    |
                                    v
         +-----------------------------------------------------+
         |               NATIVE GHOSTTY WINDOW                 |
         |  - Spawns fresh GPU-accelerated canvas context       |
         |  - Bypasses default user shells (bash/zsh)          |
         +-----------------------------------------------------+
                                    |
                                    v
                  +-----------------------------------+
                  |           WORKER MODE             |
                  |   (Locks raw TUI & Boots Engine)  |
                  +-----------------------------------+

```

### The Launcher Lifecycle

1. The user executes the `osno2` binary from their primary working terminal.
2. The launcher intercepts execution, identifies that no `--worker` flag exists, and queries the host system environment paths for the native `ghostty` executable.
3. It spawns an detached operating system process running `ghostty -e <path_to_current_exe> --worker`.
4. The parent launcher process exits immediately, freeing the user's host shell.

### The Worker Lifecycle

1. A new, detached Ghostty terminal frame instantiates.
2. The window runs the `osno2` binary with the `--worker` flag.
3. The worker disables standard shell behavior, activates terminal **Raw Mode**, claims the screen surface using the `EnterAlternateScreen` command sequence, and takes absolute ownership of standard input/output.

---

## 2. Multi-Threaded Core Processing Layers

The backend system is cleanly segregated into three operational boundaries communicating across asynchronous pipelines and lock-free channels to ensure rendering rendering calculations never starve the audio pipeline.

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

Responsible strictly for interface composition and layout constraints. Running an explicit immediate-mode drawing pattern, it queries internal states and structural layouts via **Ratatui** at up to 60 FPS. Through double-buffering frame diffs, it targets only specific changed terminal matrix coordinates, eliminating screen flickering during heavy text transformations.

### 2. Async I/O Core (Tokio Runtime)

Spawns background tasks handling the asset discovery paths. This includes tracking down music directory maps, scanning binary track trees, unpacking indexing formats, and loading configuration properties. It outputs verified indexing structs directly to the runtime database and passes commands to the audio layer.

### 3. Core Audio Engine (CPAL + Symphonia)

A dedicated, real-time operating system priority thread. It continuously feeds a ring buffer using Symphonia's media format decoding pipelines. It mixes volume vectors, maintains sample tracking, and runs window functions across multi-channel arrays, extracting short-term frames for fast Fourier transform processing.

---

## 3. Sandboxed REPL and Command Layout

Because the worker thread intercepts keyboard capture sequences natively through Crossterm before any underlying host terminal shell processes the key, the interface functions as a secured container.

* **Security Traps:** Standard control codes (`Ctrl+C`, `Ctrl+Z`) are handled explicitly by the application's input processing switch blocks. The user cannot access `sh`, `bash`, `powershell`, or invoke standard host paths like `ls` or `cd`.
* **Clap Integration:** The input command field passes typed string arrays directly into a custom `clap::Parser` instance matching strict subcommands:

| Command Sequence | Target System Component | Parameter Scope |
| --- | --- | --- |
| `/play <query>` | Audio Engine Track Selector | Fuzzy string query sequence or exact UID hashes |
| `/volume <level>` | DSP Audio Gain Matrix | Numeric scalar (`0` to `100`) |
| `/queue <query>` | Playlist Tracking Log | Appends asset paths to the active play stream |
| `/purge` | SQLite/TOML Library Index | Wipes indexed catalog configurations from disk |

---

## 4. Local File Structure Configuration

The worker maintains data configurations inside standardized machine metadata directories based on OS specifications via the `directories` crate crate layouts.

```
~/.config/osno2/
├── config.toml           # User preferences, themes, path arrays, hotkeys
├── library/
│   ├── index.db          # Embedded catalog mapping database
│   └── tracks.toml       # Local pointers and metadata indexing
└── playlists/
    ├── default.toml      # Active user playback array specifications
    └── cyberpunk.toml    # Saved play state tracking

```

---

## 5. Production Blueprint: `Cargo.toml`

```toml
[package]
name = "osno2"
version = "0.1.0"
edition = "2021"
authors = ["Osno Dev Team"]
description = "Dedicated Cyberpunk Terminal Music Appliance Wrapper"

[dependencies]
# 1. Interface & Rendering Engine
ratatui = { version = "0.30.1", features = ["crossterm", "macros", "all-widgets"] }
crossterm = { version = "0.29", features = ["event-stream"] }

# 2. Async Infrastructure & Multithreading Communication
tokio = { version = "1.43", features = ["rt-multi-thread", "macros", "fs", "time", "sync"] }
crossbeam-channel = "0.5"

# 3. Native Audio Core & Frequency Decoding
cpal = "0.15"
symphonia = { version = "0.5", features = ["all"] }
rustfft = "6.2"

# 4. Command Pipeline & Structured Data Formats
clap = { version = "4.5", features = ["derive", "string"] }
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
directories = "5.0"

# Optimize performance targets across the layout matrix
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"

```
