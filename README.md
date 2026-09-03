# Dead By Queue (`dbdq`)

> *Disclaimer: Dead By Queue is an independent, community-built TUI utilizing the public deadbyqueue.com API. It is not affiliated with, endorsed by, or partnered with deadbyqueue.com or Behaviour Interactive.*

A fast, zero-dependency, compiled native TUI for Dead by Daylight players. Beyond monitoring live matchmaking queue times and server pings directly from your terminal, Dead By Queue empowers players with full control over matchmaking through integrated AWS region locking. Compatible with both **Linux** and **Windows**.

---

## Features

- **Live Interactive Dashboard**: Built with `ratatui`, providing a clean, auto-refreshing full-screen experience.
- **Matchmaking Region Control**: Block or whitelist specific AWS server regions directly from the TUI (`l`) or CLI (`dbdq lock` / `dbdq unlock`).
- **Real-Time Auto-Refresh & Manual Trigger**: Background worker auto-fetches live data every 60 seconds, with immediate non-blocking refresh via `r`.
- **Accurate Refresh Timestamps**: Shows the exact, true time when the queue data was last updated at the source.
- **Dynamic Sorting & Filtering**:
  - Cycle sort modes with `s` (Killer → Survivor → Ping).
  - Filter by game modes (`m` to toggle Standard/Event).
- **Multi-Layout Keyboard Support**: Works on any keyboard layout (e.g. Russian Cyrillic).
- **Local Settings**: Persistent config saving your preferences (mode, sorting, locked regions):
  - Linux: `~/.config/dbdqueue/config.toml`
  - Windows: `%APPDATA%\dbdqueue\config.toml`

---

## Installation & Quick Start

### Windows (PowerShell)
Run this single command in PowerShell to automatically install and launch `dbdq`:
```powershell
irm https://raw.githubusercontent.com/trazxdxne/dbdqueue/master/install.ps1 | iex
```
*(Downloads the latest binary, adds it to your user `PATH`, and launches the dashboard).*

### Linux
Run this single command in your terminal to automatically install and launch `dbdq`:
```bash
curl -fsSL https://raw.githubusercontent.com/trazxdxne/dbdqueue/master/install.sh | bash
```
*(Or `sh -c "$(curl -fsSL https://raw.githubusercontent.com/trazxdxne/dbdqueue/master/install.sh)"`)*

---

## Manual Installation / Build from Source

### Linux
```bash
git clone https://github.com/trazxdxne/dbdqueue.git
cd dbdqueue
cargo build --release
sudo cp target/release/dbdq /usr/local/bin/
```

### Windows
1. Download the latest compiled Windows binary `dbdqueue-windows-x64.exe` from our [GitHub Releases](https://github.com/trazxdxne/dbdqueue/releases) page.
2. Rename it to `dbdq.exe` and place it in a directory in your user `PATH`.

Alternatively, compile from source (requires Rust toolchain):
```powershell
git clone https://github.com/trazxdxne/dbdqueue.git
cd dbdqueue
cargo build --release
# The compiled executable will be in: target\release\dbdq.exe
```

---

## Usage

Simply run `dbdq` to launch the interactive dashboard:
```bash
dbdq
```

### Keyboard Controls
- `↑` / `↓` : Scroll table
- `l` : Open Region Locker modal
- `s` : Cycle sort (Killer → Survivor → Ping)
- `m` : Toggle Matchmaking Mode (Standard / Event)
- `r` : Refresh queue data and ping in background
- `Esc` : Quit (or close modal)

### CLI Configuration Flags
You can start the app with specific settings, which will also be saved to your config for the next time:
```bash
dbdq --sort ping
dbdq --mode standard
dbdq lock
dbdq unlock
```

---

## Technical Specifications

- **Language**: 100% Rust
- **Dependencies**: 
  - `ratatui` & `crossterm`: Full-screen TUI rendering and event handling.
  - `clap`: Command-line parser
  - `ureq`: Lightweight HTTP client
  - `serde_json` & `serde`: Structured JSON parsing from API
  - `toml`: Config storage formatting
  - `chrono`: Displaying local/api timestamps
- **Platform Support**: Linux and Windows.
