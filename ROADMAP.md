# Dead by Daylight Queue (dbdqueue) roadmap

## Phase 1: Core Reliability, Ergonomics & Terminal Safety

Status: Complete

### Objectives

Establish rock-solid foundational reliability across the dbdqueue codebase by resolving all known review defects in CLI argument validation, exit code contracts, stream separation, terminal safety, elevation security, and test suite isolation.

### Deliverables & Included Work

- **Crossterm Deduplication & Metadata (Milestone 1)**:
  - Bumped `crossterm` dependency to `0.29` in `Cargo.toml`, matching `ratatui 0.30.1` and eliminating duplicate crate versions in `Cargo.lock`.
  - Added comprehensive publishing metadata (`description`, `license`, `repository`, `categories`, `keywords`) to `Cargo.toml`.
  - Isolated live network ping tests in `src/ping.rs` by tagging `test_all_regions_ping` with `#[ignore]` so standard `cargo test` executes 100% offline.
- **Region Validation, CLI Ergonomics & Elevation Security (Milestone 2)**:
  - Replaced permissive region token parsing with strict `resolve_region_names` returning `Result<Vec<String>, Vec<String>>`. Invalid tokens emit actionable error messages to `stderr` and exit with code `1`.
  - Prevented accidental total matchmaking blackouts: refusing to apply empty region locks when all tokens are invalid.
  - Aligned CLI exit codes with POSIX standards: elevation failures and interactive modal cancellations (`Esc`/`q`) exit with code `1`.
  - Added `#[command(version)]` to the CLI parser for `--version` and `-V`.
  - Converted `SortOrder`, `GameMode`, and `TimeFormat` to derive `clap::ValueEnum`, eliminating fragile string lowercasing.
  - Hardened Windows hosts elevation against command injection by escaping single quotes (`''''`) and utilizing `-LiteralPath`.
  - Hardened Linux hosts updates to attempt direct `fs::write` before falling back to `pkexec`.
- **TUI Overhaul, Clutter Audit & Terminal Guard (Milestone 3)**:
  - Replaced nested double borders with a clean, single-line outer frame and single-border visual hierarchy.
  - Implemented semantic theme tokens (`Theme`) with complete `NO_COLOR` environment variable compliance.
  - Implemented responsive breakpoint ladder: Standard (80x24), Narrow (60x24 tmux split with column prioritization), and Floor (<48x12 with graceful resize notice).
  - Added vim key navigation (`j`/`k`, `g`/`G`) and standardized `Esc`/`q` quit ladder.
  - Added RAII `TerminalGuard` and panic recovery hook in `src/tui.rs` guaranteeing raw mode deactivation, alternate screen exit, and cursor restoration on panic.

### Concrete Risks & Mitigations

- **Risk**: User hosts file corrupted by interrupted write or power failure.
  - *Mitigation*: Atomic temporary file generation, tag-isolated block replacement (`# --- DBDQUEUE START ---`), and permission checks prior to writing.
- **Risk**: Terminal emulator remains stuck in raw mode with hidden cursor after an unhandled panic.
  - *Mitigation*: Custom panic hook registered via `std::panic::set_hook` coupled with an RAII drop guard on `TerminalGuard`.
- **Risk**: CLI breaking existing scripts when rejecting unrecognized region arguments.
  - *Mitigation*: Case-insensitive aliases (e.g., `frankfurt`, `eu-central-1`, `germany`, `fra`) preserved and mapped deterministically to canonical AWS codes.

### Explicit Exit Criteria

- `cargo test` executes completely offline, with 87 passing unit tests and 1 ignored live test.
- `cargo clippy -- -D warnings` passes with 0 warnings.
- `dbdq lock invalid_name` writes to `stderr` and exits code `1` without modifying the hosts file.
- `dbdq --version` outputs crate version and exits code `0`.
- All TUI responsive breakpoints render cleanly without layout panics down to 48x12.

---

## Phase 2: Feature Planning, CI Automation & Extended Platform Support

Status: In Progress

### Objectives

Document system specifications and future features adhering strictly to titus-ai planning standards, implement automated continuous integration pipelines across Linux and Windows runners, and prepare repository governance for release readiness.

### Deliverables & Included Work

- **Titus-AI Planning Documentation (Milestone 4)**:
  - Authoritative `SPEC.md` defining system architecture, data flow diagrams, security boundaries, non-goals, and concrete feature proposals.
  - Phased `ROADMAP.md` defining dependency-ordered milestones, risks, mitigations, and explicit exit criteria.
  - Granular `TASKS.md` with status checkboxes, acceptance criteria, and verification evidence pointers.
- **Continuous Integration Pipeline**:
  - Add GitHub Actions CI workflow covering `ubuntu-latest` and `windows-latest`.
  - Validate `cargo check`, `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, and `cargo fmt -- --check`.
  - Validate offline isolation: enforce that standard CI test steps pass without external network access.
- **End-to-End Verification & Forensic Audit (Milestone 5)**:
  - Comprehensive end-to-end audit verifying real state management, exit code contracts, and error handling across all targets.

### Concrete Risks & Mitigations

- **Risk**: Discrepancies between documentation and actual implementation behavior over time.
  - *Mitigation*: Specification review step in PR readiness workflow; every code change requires updating `SPEC.md` and `TASKS.md`.
- **Risk**: Windows CI runner permission differences affecting hosts elevation tests.
  - *Mitigation*: Mock filesystem paths and simulate elevation failure modes in automated tests; avoid mutating runner system hosts file.

### Explicit Exit Criteria

- `SPEC.md`, `ROADMAP.md`, and `TASKS.md` checked in at repository root conforming to titus-ai standards.
- CI pipeline running and passing on both Linux and Windows runners.
- Forensic audit completed with zero unverified claims or hardcoded facade implementations.

---

## Phase 3: Automated Best-Region Locking (`--auto`) & Structured Machine Output (`--json`)

Status: Planned

### Objectives

Eliminate manual region selection through intelligent latency- and queue-aware auto-locking, and expose machine-readable JSON streaming for OBS overlays, streaming toolkits, and shell automation.

### Deliverables & Included Work

- **Automated Best-Region Locking (`dbdq lock --auto`)**:
  - Extend `dbdq lock` with `--auto`, `--max-ping <ms>`, `--role <killer|survivor>`, and `--prefer <ping|queue>` flags.
  - Implement deterministic candidate selection algorithm: filter non-disabled regions with ping <= threshold, select minimum queue duration, and break ties with lowest ping.
  - Implement automatic fallback warning when all regions exceed max ping threshold.
  - Apply surgical hosts lock atomically and output selected region summary to `stdout`.
- **Machine-Readable Structured JSON Output (`dbdq --json`)**:
  - Implement `--json` flag on root `dbdq` command.
  - Emit versioned JSON payload (v1) containing timestamp, active role, locked regions array, per-region metrics, and best survivor/killer recommendations.
  - Implement NDJSON streaming under `--watch <interval_secs>`.
  - Enforce strict stream separation: pure JSON to `stdout`, all diagnostics and error messages to `stderr`.
- **Test Suite Enhancements**:
  - Unit tests for `--auto` candidate selection with synthetic queue times and pings.
  - Schema validation tests for `--json` output structure and serialization accuracy.

### Concrete Risks & Mitigations

- **Risk**: Fluctuating ping measurements causing rapid regional flapping under `--auto`.
  - *Mitigation*: Integrate ping cache averages and apply a 15-second queue time tolerance band before re-locking.
- **Risk**: Extraneous log or warning statements contaminating `stdout` during `--json` operations and breaking downstream JSON parsers (e.g. `jq`).
  - *Mitigation*: Audit all output macros across the codebase to ensure non-JSON messages strictly use `eprintln!`.

### Explicit Exit Criteria

- `dbdq lock --auto --role killer --max-ping 70` selects the optimal region deterministically and applies lock.
- `dbdq --json | jq .version` executes cleanly with exit code `0` and valid output.
- All unit and integration tests for `--auto` and `--json` pass with 100% reliability.

---

## Phase 4: Matchmaking Profiles & Queue Alerts (`watch`)

Status: Planned

### Objectives

Provide seamless role-dependent regional configurations for Killer vs. Survivor players, and introduce real-time queue threshold alerting via terminal bell and desktop notifications.

### Deliverables & Included Work

- **Matchmaking Role Profiles (`dbdq --profile killer|survivor`)**:
  - Extend configuration model with `profiles` storing independent region lists for `killer` and `survivor`.
  - Implement CLI subcommands: `dbdq profile set`, `dbdq profile apply`, and quick switch alias `dbdq switch <killer|survivor>`.
  - Add TUI shortcut `P` to cycle active profile and update hosts file with immediate visual feedback.
- **Queue Alert Watcher (`dbdq watch --threshold <secs>`)**:
  - Implement `dbdq watch` daemon loop polling queue telemetry at configurable intervals (`--interval <secs>`).
  - Implement trigger alerting via terminal bell (`\x07`) and native desktop notifications when queue time falls below `--threshold`.
  - Add stateful debouncing and a 120-second alert cooldown to prevent notification fatigue.
  - Support graceful termination on `Ctrl+C` or `q` with exit code `0`.
- **Cross-Platform Notification Support**:
  - Integrate platform-specific notification handlers (Windows Toast notifications, Linux D-Bus `notify-send` / freedesktop).

### Concrete Risks & Mitigations

- **Risk**: Continuous background polling overwhelming public API endpoints or triggering rate limits.
  - *Mitigation*: Enforce a hard minimum polling interval of 5 seconds (default 15 seconds) and respect HTTP cache/retry headers.
- **Risk**: Missing notification daemons on headless or minimal Linux systems causing panics or unhandled errors.
  - *Mitigation*: Gracefully degrade to standard terminal bell (`\x07`) and stderr warning if desktop notification dispatcher fails.

### Explicit Exit Criteria

- `dbdq switch killer` switches active lock to the saved Killer profile and applies hosts lock.
- `dbdq switch survivor` switches active lock to the saved Survivor profile and applies hosts lock.
- `dbdq watch --threshold 45` triggers audible bell and desktop notice when queue duration drops below 45s.
- Graceful exit on `Ctrl+C` cleanly restores terminal state and exits code `0`.
