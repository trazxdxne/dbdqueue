# Dead by Daylight Queue Times CLI (`dbdqueue`)

A fast, zero-dependency, compiled native CLI tool for Dead by Daylight players to view real-time matchmaking queue times directly from their terminal via a beautiful full-screen interactive TUI dashboard. Compatible with both **Linux** and **Windows**.

---

## Features

- **Live Interactive Dashboard**: Built with `ratatui`, providing a clean, auto-refreshing full-screen experience.
- **Real-Time Auto-Refresh**: Background worker fetches live data every 60 seconds without needing to restart the app.
- **Accurate Refresh Timestamps**: Shows the exact, true time when the queue data was last updated at the source.
- **Dynamic Sorting & Filtering**:
  - Sort by survivor times (`s`), killer times (`k`), custom priority list (`r`), or default region name.
  - Filter by game modes (`m` to toggle Standard/Event/Both).
- **Local Settings**: Persistent config saving your preferences (mode, sorting, priority regions):
  - Linux: `~/.config/dbdqueue/config.toml`
  - Windows: `%APPDATA%\dbdqueue\config.toml`

---

## Installation

### Linux
Install using our one-liner shell script:
```bash
curl -sSfL https://raw.githubusercontent.com/trazxdxne/dbdqueue/master/install.sh | sh
```

Alternatively, compile from source:
```bash
git clone https://github.com/trazxdxne/dbdqueue.git
cd dbdqueue
cargo build --release
sudo cp target/release/dbdqueue /usr/local/bin/
```

### Windows
1. Download the latest compiled Windows binary `dbdqueue-windows-x64.exe` from our [GitHub Releases](https://github.com/trazxdxne/dbdqueue/releases) page.
2. Rename it to `dbdqueue.exe`.
3. Add the folder containing the executable to your user `PATH` environment variable so you can run it from any PowerShell or Command Prompt.

Alternatively, compile from source (requires Rust toolchain):
```powershell
git clone https://github.com/trazxdxne/dbdqueue.git
cd dbdqueue
cargo build --release
# The compiled executable will be in: target\release\dbdqueue.exe
```

---

## Usage

Simply run `dbdqueue` to launch the interactive dashboard:
```bash
dbdqueue
```

### Keyboard Controls
- `s` : Sort by Survivor queue times
- `k` : Sort by Killer queue times
- `r` : Sort by your custom priority regions
- `m` : Toggle between Standard, Event, or Both matchmaking modes
- `q` or `Esc` : Quit

### CLI Configuration Flags
You can start the app with specific settings, which will also be saved to your config for the next time:
```bash
dbdqueue --sort survivor
dbdqueue --mode standard
dbdqueue --priority "Frankfurt, Dublin, Virginia"
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
