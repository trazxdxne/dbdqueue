# Project: dbdqueue Overhaul & Titus-AI Planning

## Architecture
- `src/main.rs`: CLI argument parsing via `clap` (`#[command(version)]`, `ValueEnum`), subcommand dispatch, process exit codes and stderr reporting.
- `src/config.rs`: Configuration data structures (`Config`, `SortOrder`, `GameMode`, `TimeFormat`) deriving `clap::ValueEnum`, serde persistence.
- `src/api.rs`: AWS region registry, endpoint definitions, region name token resolution and strict validation.
- `src/hosts.rs`: System hosts file parsing, marker block management, DNS flush, secure cross-platform elevation (PowerShell escaping on Windows, direct write attempt before pkexec on Linux), interactive CLI modal.
- `src/ping.rs`: Latency measurements to AWS DynamoDB endpoints; isolated offline tests.
- `src/tui.rs`: Event loop, terminal lifecycle management, RAII `TerminalGuard`, panic recovery hook, asynchronous fetch/ping threads.
- `src/ui.rs`: Immediate-mode Ratatui drawing, semantic theme tokens, responsive breakpoints (standard 80x24, narrow 60-col, floor 48x12), clutter reduction, table layouts, status indicators.
- `src/app.rs`: Application state, keyboard navigation (vim `j`/`k`, `Esc`/`q` quit ladder, `Ctrl+C`), sorting, filtering, lock modal state.

## Feature Inventory
Every feature from the Survey phase appears here with its assigned milestone:
| # | Feature | Description | Milestone | Source |
|---|---------|-------------|-----------|--------|
| 1 | Crossterm Deduplication | Bump `crossterm` to `0.29` matching `ratatui 0.30.1` | M1 | R5 |
| 2 | Crate Packaging Metadata | Add `description`, `license`, `repository`, `categories`, `keywords` to `Cargo.toml` | M1 | R5 |
| 3 | Offline Test Isolation | Mark `test_all_regions_ping` with `#[ignore]` in `src/ping.rs` | M1 | R5 |
| 4 | Strict Region Token Validation | Validate CLI tokens in `dbdq lock <regions>`, reject unknown tokens to stderr, exit non-zero | M2 | R3 |
| 5 | Non-Zero Exit on Elevation/Update Failure | Return non-zero exit codes if `hosts::update_hosts` fails or elevation is rejected | M2 | R3 |
| 6 | Non-Zero Exit on Modal Abort | Cancel in interactive lock menu (`Esc`/`q`) exits with non-zero code | M2 | R3 |
| 7 | Non-Zero Exit on Config Save Failure | Fail with non-zero exit code if saving config fails after flag update | M2 | R3 |
| 8 | CLI Version Output | Add `#[command(version)]` to `Cli` struct in `src/main.rs` | M2 | R4 |
| 9 | Structured Enum CLI Parsing | Derive `clap::ValueEnum` for `SortOrder`, `GameMode`, `TimeFormat` | M2 | R4 |
| 10 | Windows Elevation Quoting Security | Escape single quotes (`''''`) and use `-LiteralPath` in PowerShell elevation commands | M2 | R4 |
| 11 | Linux Direct Hosts Write | Attempt `std::fs::write` to `/etc/hosts` before falling back to `pkexec` | M2 | R4 |
| 12 | Terminal Lifecycle & Panic Hook | RAII `TerminalGuard` and panic hook restoring terminal state cleanly on panic | M3 | R2 & R5 |
| 13 | TUI Clutter & Border Reduction | Single-line outer framing, eliminate nested boxes and saturated bracket spam | M3 | R2 |
| 14 | Responsive Viewports & Floor | Support 80x24 standard, 60-col narrow split, and 48x12 floor fallback | M3 | R2 |
| 15 | Semantic Theme & Metric Alignment | Semantic color tokens with `NO_COLOR` safety, right-aligned metric columns, sort indicators | M3 | R2 |
| 16 | TUI Keyboard Ergonomics | Support vim `j`/`k` navigation, `Esc`/`q` quit ladder, and `Ctrl+C` interrupt | M3 | R2 |
| 17 | Titus-AI Planning Documents | Create `SPEC.md`, `ROADMAP.md`, `TASKS.md` following titus-ai conventions | M4 | R1 |
| 18 | Feature Proposals | Grounded proposals (`dbdq lock --auto`, `dbdq --json`, queue alerts, role profiles) | M4 | R1 |
| 19 | E2E Integration & Verification | Full test suite, clippy clean, CLI behavior verification, forensic audit | M5 | Acceptance |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | Dependencies, Test Isolation & Metadata | `Cargo.toml`, `Cargo.lock`, `src/ping.rs` | None | DONE |
| 2 | Region Validation, CLI Ergonomics & Elevation Security | `src/api.rs`, `src/main.rs`, `src/config.rs`, `src/hosts.rs` | M1 | DONE |
| 3 | TUI Design Overhaul & Terminal Panic Guard | `src/tui.rs`, `src/ui.rs`, `src/app.rs` | M1 | DONE |
| 4 | Titus-AI Planning Documents & Feature Proposals | `SPEC.md`, `ROADMAP.md`, `TASKS.md` | None | DONE |
| 5 | E2E Verification & Forensic Integrity Audit | Global verification across crate | M1, M2, M3, M4 | DONE |

## Interface Contracts
### `src/api.rs` ↔ `src/main.rs`
- `resolve_region_names(input: &[String]) -> Result<Vec<String>, Vec<String>>`:
  Returns `Ok(aws_codes)` when all tokens resolve successfully.
  Returns `Err(invalid_tokens)` containing any tokens that failed to resolve.
- `src/main.rs` checks if `Err(invalid)`: prints error to `eprintln!` and exits with code 1.
- `src/main.rs` checks if resolved `Ok(regions)` is empty: prints error to `eprintln!` and exits with code 1 without calling `update_hosts`.

### `src/hosts.rs` ↔ `src/main.rs`
- `update_hosts(regions: Option<&[String]>, flush_dns: bool) -> UpdateHostsResult`
  - `UpdateHostsResult::Success`: `process::exit(0)`
  - `UpdateHostsResult::ElevationFailed`: `eprintln!("Elevation failed/cancelled."); process::exit(1);`
  - `UpdateHostsResult::Error(err)`: `eprintln!("Error: {}", err); process::exit(1);`
- `interactive_lock_menu(...) -> Option<Vec<String>>`
  - When cancelled (`Esc`/`q`), returns `None`. Caller in `main.rs` prints `"Cancelled."` to stderr and calls `process::exit(1)`.

### `src/config.rs` ↔ `src/main.rs`
- `SortOrder`, `GameMode`, `TimeFormat` derive `clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq`.
- `save_config(path: &Path, config: &Config) -> io::Result<()>`:
  If `save_config` returns `Err(e)`, caller logs to `eprintln!` and exits with code 1.

## Code Layout
- `Cargo.toml`: Package configuration, metadata, dependencies.
- `src/main.rs`: CLI entry point, argument parsing, command execution.
- `src/config.rs`: Settings, enums, persistence.
- `src/api.rs`: Data models, AWS region definitions, resolution logic.
- `src/hosts.rs`: System hosts file manipulation, elevation scripts.
- `src/ping.rs`: Latency measurement and caching.
- `src/tui.rs`: Terminal initialization, event loop, panic guard.
- `src/ui.rs`: Ratatui rendering, styling, layout.
- `src/app.rs`: State management, event handling.
- `SPEC.md`: Formal specification conforming to titus-ai.
- `ROADMAP.md`: Phased roadmap conforming to titus-ai.
- `TASKS.md`: Actionable task checklist conforming to titus-ai.
