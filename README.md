# Dead by Daylight Queue Times CLI (`dbdqueue`)

A fast, zero-dependency, compiled native CLI tool for Dead by Daylight players to view real-time matchmaking queue times directly from their terminal via a beautiful full-screen interactive TUI dashboard. Compatible with both **Linux** and **Windows**.

---

## Features

- **Live Interactive Dashboard**: Built with `ratatui`, providing a clean, auto-refreshing full-screen experience.
- **Real-Time Auto-Refresh**: Background worker fetches live data every 60 seconds without needing to restart the app.
- **Accurate Refresh Timestamps**: Shows the exact, true time when the queue data was last updated at the source.
- **Dynamic Sorting & Filtering**:
  - Sort by survivor times (`s`), killer times (`k`), ping (`p`), or default region name (`d`).
  - Filter by game modes (`m` to toggle Standard/Event).
- **Region Locker**: Block or whitelist specific AWS server regions directly from the TUI (`l`) or CLI (`dbdq lock` / `dbdq unlock`).
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
- `s` : Sort by Survivor queue times
- `k` : Sort by Killer queue times
- `p` : Sort by Ping
- `d` : Sort by Default (region name)
- `m` : Toggle Matchmaking Mode (Standard / Event)
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
