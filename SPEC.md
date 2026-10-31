# Dead by Daylight Queue (dbdqueue) specification

## Problem

Dead by Daylight (Behaviour Interactive) uses Amazon GameLift fleet servers across 11 global AWS regions for multiplayer matchmaking. The in-game matchmaking client provides no visibility into regional queue durations or server latency, frequently placing players into distant matchmaking pools with high ping (>150ms) or extended queue times (>5 minutes).

Players attempting to restrict matchmaking to low-latency or fast-queue regions must manually look up regional AWS endpoints, edit system hosts files with elevated privileges, and manage DNS cache flushes. Manual edits risk hosts file corruption, network syntax errors, or accidental total matchmaking blackouts. Furthermore, existing community scripts lack real-time telemetry, terminal safety, robust cross-platform elevation handling, and structured integration points for streamers and automated tooling.

`dbdqueue` solves this by providing a high-performance terminal dashboard (TUI) and scriptable CLI that displays live regional queue estimates and ICMP ping latency, manages surgical hosts-file-based region locks with safe privilege elevation, and exposes structured interfaces for automation.

## Users

The target users are PC Dead by Daylight players across Steam, Epic Games Store, and Windows Store:

1. **Competitive Solo & Team Players**: Players seeking the lowest latency (<60ms) who need to lock local regions (e.g., Frankfurt, London, Virginia) to prevent high-ping matches.
2. **Casual & Asymmetric Players**: Players who switch roles (Survivor vs. Killer) depending on active queue incentive bonuses and seek the fastest queue times regardless of moderate ping.
3. **Streamers & Content Creators**: Broadcasters using OBS or streaming overlays who require machine-readable JSON data streams to render live queue and ping widgets on stream.
4. **Automation & Homelab Users**: Power users launching Dead by Daylight through scripts, wrappers, or Game Launchers who require automated best-region selection (`--auto`) and POSIX-compliant CLI exit codes.

## Operating principles

1. **Working code only**: Every feature must reflect verifiable, functional code. Plausibility is not correctness.
2. **Minimal necessary changes**: Touch only the components required to solve the stated problem. Drive-by refactorings and speculative abstractions are strictly avoided.
3. **Surgical hosts file isolation**: System hosts modifications are restricted strictly to a tagged, marker-delimited block (`# --- DBDQUEUE START ---` to `# --- DBDQUEUE END ---`). The remainder of the hosts file is never mutated.
4. **Stream separation**: Informational data, JSON streams, and user prompts route strictly to `stdout`. Diagnostic logs, warnings, spinner animations, and error messages route strictly to `stderr`.
5. **Deterministic exit codes**: Operations return `0` on complete success, `1` on operational, validation, or elevation failure, `2` on CLI syntax/argument errors, and `130` on interactive user cancellation.

## Architecture

The application is structured into modular crates within a single binary:

```text
                               ┌────────────────────────────────┐
                               │     CLI Input (clap v4)        │
                               │ src/main.rs & src/config.rs    │
                               └──────────────┬─────────────────┘
                                              │
                 ┌────────────────────────────┼────────────────────────────┐
                 ▼                            ▼                            ▼
       [One-Shot CLI Commands]      [Machine Data Stream]       [Interactive TUI Loop]
       - dbdq lock <regions>        - dbdq --json               - src/tui.rs
       - dbdq unlock                - dbdq watch --threshold    - src/ui.rs
       - dbdq config                                            - src/app.rs
                 │                            │                            │
                 ├────────────────────────────┼────────────────────────────┤
                 │                            ▼                            │
                 │              ┌───────────────────────────┐              │
                 │              │    Asynchronous Workers   │              │
                 │              │ - src/api.rs (HTTP queues)│              │
                 │              │ - src/ping.rs (ICMP ms)   │              │
                 │              │ - src/cache.rs (disk)     │              │
                 │              └─────────────┬─────────────┘              │
                 │                            │                            │
                 ▼                            ▼                            ▼
   ┌──────────────────────────┐  ┌─────────────────────────┐  ┌─────────────────────────┐
   │ System Hosts Management  │  │ Structured JSON Stream   │  │ Ratatui Terminal Frame  │
   │ - src/hosts.rs           │  │ - stdout: JSON payload   │  │ - Responsive layouts    │
   │ - Tagged marker block    │  │ - stderr: Diagnostics    │  │ - Single-line chrome    │
   │ - PowerShell / pkexec    │  └─────────────────────────┘  │ - TerminalGuard RAII    │
   └──────────────────────────┘                                └─────────────────────────┘
```

### Component Breakdown

- **`src/main.rs`**: Entry point, clap CLI definition (`#[command(version)]`), subcommand dispatch, process exit code propagation, and stream separation.
- **`src/config.rs`**: Persistent configuration (`Config`), strongly typed enums (`SortOrder`, `GameMode`, `TimeFormat`) deriving `clap::ValueEnum`, and JSON serialization.
- **`src/api.rs`**: Data models (`QueueData`, `MiscResponse`), HTTP fetch client (`ureq`), AWS region endpoint definitions, and strict region token resolution (`resolve_region_names`).
- **`src/hosts.rs`**: Hosts file parser, tagged block builder, DNS cache flushing (`ipconfig /flushdns` or `resolvectl flush-caches`), Windows PowerShell elevation quoting, and Linux direct write / `pkexec` fallback.
- **`src/ping.rs`**: Background ICMP latency measurement against regional DynamoDB endpoints (`dynamodb.<region>.amazonaws.com`), ping color classification, and offline-isolated unit tests.
- **`src/cache.rs`**: Disk cache (`ping_cache.json`) for instant startup latency rendering before fresh network polls complete.
- **`src/tui.rs`**: Terminal lifecycle management, raw mode initialization, background async poll threads, RAII `TerminalGuard`, and panic recovery hook.
- **`src/ui.rs`**: Immediate-mode Ratatui rendering, semantic theme tokens (`Theme`), single-line border framing, and responsive breakpoints (Standard 80x24, Narrow 60x24, Floor 48x12).
- **`src/app.rs`**: Application state machine, table row selection, sort cycling, mode toggles, notice banners, and vim navigation (`j`/`k`, `Esc`/`q` quit ladder).

## Required behavior

### 1. Telemetry and Queue Measurement

- Poll regional matchmaking queues from the public API (`https://dbd.trident8.xyz/api/queues`).
- Measure round-trip ping latency to regional AWS endpoints asynchronously without blocking UI rendering or CLI commands.
- Support 11 global server regions:
  - Europe: Frankfurt (`eu-central-1`), London (`eu-west-2`), Stockholm (`eu-north-1`).
  - Americas: N. Virginia (`us-east-1`), Ohio (`us-east-2`), Oregon (`us-west-2`), São Paulo (`sa-east-1`).
  - Asia / Oceania: Tokyo (`ap-northeast-1`), Singapore (`ap-southeast-1`), Sydney (`ap-southeast-2`).
  - Middle East: UAE (`me-central-1`).
- Cache ping measurements to disk (`~/.cache/dbdqueue/ping_cache.json` on Linux, `%LOCALAPPDATA%\dbdqueue\ping_cache.json` on Windows) to render immediate latency estimates on startup.

### 2. Interactive Terminal Dashboard (TUI)

- **Layout Hierarchy**: Four-section vertical structure consisting of Header, High-Density Queue Table, Summary Card, and Footer/Hint Bar.
- **Clutter Audit**: Maximum 1 border between screen edge and content; eliminate nested boxes, heavy outer banners, and bracket spam. Keybindings render with muted delimiters (e.g., `L lock ⋅ S sort ⋅ q quit`).
- **Responsive Viewport Ladder**:
  - *Standard (≥80 columns)*: Full table (Region, Ping, Survivor Queue, Killer Queue, Status), two-column Summary Card, and full shortcut bar.
  - *Narrow (60–79 columns)*: Condensed table dropping secondary columns, collapsed single-line summary, compact key hints.
  - *Floor (<48 columns or <12 rows)*: Halts standard rendering and displays a polite "Terminal too small: need 48x12" fallback frame.
- **Terminal Safety & Panic Guard**: RAII `TerminalGuard` and custom panic hook guarantee that terminal raw mode is disabled, alternate screen is vacated, and cursor is made visible upon both clean exit and application panic.
- **Keyboard Ergonomics**: Full arrow key and vim key support (`j`/`k` move, `g`/`G` top/bottom). `Esc` quit ladder: cancels modals and notices first, while `q` cleanly exits at the root.

### 3. Surgical Region Locking & CLI Operations

- `dbdq lock <regions...>`: Resolves user region tokens (e.g., `frankfurt`, `eu-central-1`, `virginia`) against canonical AWS codes. Invalid tokens write an error to `stderr` and exit with code `1`.
- If the resolved list is empty, `dbdq lock` writes an error to `stderr` and exits code `1` without updating the hosts file.
- Unselected regions are redirected to `0.0.0.0` inside the tagged `# --- DBDQUEUE START ---` block in the system hosts file.
- `dbdq unlock`: Clears all entries within the tagged block and restores full matchmaking.
- Cancelling the interactive lock menu (`Esc` or `q`) prints `"Cancelled."` to `stderr` and exits code `130` or `1`.

## Grounded feature proposals

### Feature A: Automated Best-Region Locking (`dbdq lock --auto`)

#### Problem Statement
Players currently have to manually review queue times and pings in the TUI, select regions in the lock modal, and trigger elevation. During role incentive switches, players repeat this manual cycle every few hours.

#### Functional Specification
- Command: `dbdq lock --auto [OPTIONS]`
- Flags:
  - `--role <killer|survivor>`: Role to optimize for (defaults to active config role).
  - `--max-ping <ms>`: Maximum acceptable round-trip latency in milliseconds (default: `60`).
  - `--prefer <ping|queue>`: Tie-breaker policy when times or pings are comparable (default: `queue`).
- **Selection Algorithm**:
  1. Fetch current queue durations and measure pings across all 11 AWS regions.
  2. Filter candidates: Exclude disabled regions and regions where `ping > max_ping`.
  3. If candidate set is non-empty:
     - Select candidate with the lowest queue time for the specified role.
     - If multiple candidates tie within a 15-second queue window, select the candidate with the lowest ping.
  4. If candidate set is empty (all pings exceed `--max-ping`):
     - Fallback: Select the single region with the lowest absolute ping, emit a warning to `stderr` (`"Warning: No region met max ping threshold (60ms). Best fallback: <region> (<ping>ms)"`), and apply lock.
  5. Apply surgical hosts lock to redirect all other regions to `0.0.0.0`, flush DNS, and report the locked region to `stdout`.

### Feature B: Structured Machine Output Stream (`dbdq --json`)

#### Problem Statement
Broadcasters and tool authors need live queue times and locked region status to power stream overlays (OBS browser sources), Discord bot integrations, and shell automation scripts without screen-scraping ANSI terminal output.

#### Functional Specification
- Command: `dbdq --json [OPTIONS]`
- Flags:
  - `--mode <standard|event>`: Matchmaking pool to query (default: `standard`).
  - `--role <killer|survivor>`: Filter summary to a specific role.
  - `--watch <interval_secs>`: Stream periodic updates as newline-delimited JSON (NDJSON).
- Output Discipline:
  - Standard output (`stdout`) receives strictly valid, machine-readable JSON.
  - All logs, progress indicators, and error messages route to `stderr`.
- **JSON Schema (v1)**:
```json
{
  "version": 1,
  "timestamp": "2026-10-31T03:30:00Z",
  "active_role": "survivor",
  "locked_regions": ["eu-central-1"],
  "regions": [
    {
      "aws_code": "eu-central-1",
      "name": "Frankfurt",
      "flag": "🇩🇪",
      "ping_ms": 28,
      "survivor_seconds": 12,
      "killer_seconds": 45,
      "survivor_formatted": "12s",
      "killer_formatted": "45s",
      "is_locked": true,
      "is_disabled": false
    }
  ],
  "best_survivor": {
    "aws_code": "eu-central-1",
    "name": "Frankfurt",
    "queue_seconds": 12,
    "ping_ms": 28
  },
  "best_killer": {
    "aws_code": "us-east-1",
    "name": "Virginia",
    "queue_seconds": 30,
    "ping_ms": 95
  }
}
```

### Feature C: Queue Duration Alert Trigger (`dbdq watch --threshold <secs>`)

#### Problem Statement
Players queuing for Killer during peak Survivor hours often wait in queues exceeding 5 to 10 minutes while tabbed out, missing the match lobby or lobby load countdown.

#### Functional Specification
- Command: `dbdq watch --threshold <secs> [OPTIONS]`
- Flags:
  - `--role <killer|survivor>`: Role to monitor (default: `killer`).
  - `--threshold <secs>`: Queue duration in seconds that triggers the alert (default: `60`).
  - `--region <aws_code>`: Target region to monitor (default: currently locked region or lowest ping region).
  - `--interval <secs>`: Polling interval in seconds (default: `15`, minimum: `5`).
  - `--bell`: Trigger ASCII terminal bell (`\x07`) on trigger (default: `true`).
  - `--notify`: Send OS desktop notification via platform notification daemon (default: `true`).
- **Execution Lifecycle**:
  1. Polls target region queue time every `--interval` seconds.
  2. Displays single-line status updates to `stderr` with timestamp and current estimate.
  3. When queue estimate drops to or below `--threshold`:
     - Emits `\x07` (bell) to `stdout`.
     - Dispatches OS desktop notification: `"dbdqueue: Killer queue in Frankfurt is now 45s (below 60s threshold)"`.
     - Implements a 120-second alert cooldown to prevent repetitive notification spam.
  4. Exits with code `0` on clean interrupt (`Ctrl+C` / `q`).

### Feature D: Matchmaking Role Profiles (`dbdq --profile killer|survivor`)

#### Problem Statement
Killer gameplay is server-authoritative for hit validation and latency-critical; playing Killer with >70ms ping produces noticeable hit lag. Conversely, Survivor gameplay can comfortably tolerate up to 100ms ping in exchange for instant queues. Players currently reconfigure locked regions every time they switch roles.

#### Functional Specification
- Command: `dbdq profile <killer|survivor> [regions...]` or `dbdq --profile <killer|survivor>`
- Subcommands & Flags:
  - `dbdq profile set <killer|survivor> <regions...>`: Saves a region list for the specified role.
  - `dbdq profile apply <killer|survivor>`: Applies the saved hosts lock for the role immediately.
  - `dbdq switch <killer|survivor>`: Shortcut alias to apply the role profile.
- Configuration Schema Extension:
  ```json
  {
    "profiles": {
      "survivor": ["eu-central-1", "eu-west-2", "eu-north-1"],
      "killer": ["eu-central-1"]
    },
    "active_profile": "survivor"
  }
  ```
- Seamless TUI Integration: Pressing `P` in the TUI toggles between Survivor and Killer profiles, updating the hosts file and status bar with a 3.5s confirmation notice.

## Security and privacy

1. **Surgical Scope**: Hosts file writes mutate only the region endpoints enclosed between `# --- DBDQUEUE START ---` and `# --- DBDQUEUE END ---`. System DNS and preexisting localhost/custom mappings are never modified.
2. **Command Injection Prevention**:
   - On Windows, paths interpolated into PowerShell elevation commands (`Start-Process powershell -Verb RunAs -ArgumentList ...`) are escaped with quadrupled single quotes (`''''`) and passed via `-LiteralPath` to prevent code injection via directory names containing spaces, quotes, or semicolons.
3. **Privilege Demarcation**:
   - Telemetry querying, ping measurement, configuration management, JSON streaming, and TUI monitoring run unprivileged.
   - Elevation is requested strictly when applying or removing hosts file locks.
   - On Linux, direct `std::fs::write` is attempted first (handling environments running as root or under `sudo`) before falling back to `pkexec`.
4. **Zero Telemetry**: `dbdqueue` collects no player telemetry, account data, or analytics. Outbound network traffic is limited strictly to public queue endpoints and ICMP latency pings.

## Compatibility

- **Operating Systems**:
  - Windows 10 and Windows 11 (PowerShell UAC elevation).
  - Linux (glibc and musl, systemd-resolved / nscd / dnsmasq flush, `pkexec` elevation).
- **Terminal Emulators**:
  - Modern emulators: Windows Terminal, Alacritty, WezTerm, Kitty, Foot.
  - Classic environments: Windows `conhost.exe`, tmux, screen.
  - Fallback: `NO_COLOR` environment variable suppresses ANSI color styling without breaking structural alignment or border drawing.
- **Offline Isolation**:
  - Unit tests run completely offline. Tests requiring live AWS network endpoints (`test_all_regions_ping`) are tagged with `#[ignore]` and omitted from standard `cargo test` executions.

## Non-goals

1. **Process Hooking & Memory Injection**: `dbdqueue` will never hook Dead by Daylight's game process memory, inject DLLs, or manipulate anti-cheat (Easy Anti-Cheat). All region locking operates strictly via standard OS DNS/hosts resolution.
2. **Network Adapter or VPN Management**: `dbdqueue` does not configure virtual network adapters, TAP devices, or third-party VPN tunnels.
3. **Speculative Abstractions**: Generic plugin engines, Lua scripting runtimes, and unnecessary cloud sync features are explicitly rejected.
4. **Third-Party Review or Metric Tracking**: No external telemetry, crash reporting daemons, or unverified remote dependencies.

## Acceptance criteria

- `cargo check`, `cargo test`, and `cargo clippy -- -D warnings` pass cleanly from a fresh checkout with zero warnings.
- Passing invalid region tokens to `dbdq lock` writes clear diagnostics to `stderr` and exits with non-zero code `1`.
- Passing an empty resolved region set refuses to lock all regions, logs an error to `stderr`, and exits code `1`.
- Aborting interactive prompts via `Esc` or `q` exits with non-zero code `1`.
- Hosts elevation failures exit with non-zero code `1`.
- `dbdq --version` and `dbdq -V` display the crate version and exit with code `0`.
- TUI respects terminal floor (<48 cols or <12 rows) by rendering the resize notification rather than panicking or wrapping.
- Unhandled panics in TUI restore terminal raw mode, alternate screen, and cursor visibility via `TerminalGuard`.
- Windows elevation command strings safely escape single quotes and spaces in file paths.

## Unresolved questions

1. **macOS Hosts Locking**: Whether to add macOS support (`/etc/hosts` and `killall -HUP mDNSResponder`) if Dead by Daylight introduces native macOS Apple Silicon client support.
2. **Desktop Notification Protocol on Linux**: Selecting the most robust headless fallback when D-Bus notification daemons are absent (e.g. falling back to console bell `\x07`).
