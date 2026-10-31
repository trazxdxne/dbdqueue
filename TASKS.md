# Dead by Daylight Queue (dbdqueue) tasks

## Completed phases

### Phase 1: Core Reliability, Ergonomics & Terminal Safety

#### Milestone 1: Dependencies, Test Suite Architecture & Metadata
- [x] Deduplicate `crossterm` dependency: Bumped `crossterm` to `0.29` in `Cargo.toml` to align with `ratatui 0.30.1`.
  - *Acceptance Criteria*: Single version of crossterm compiled without crate version duplication in `Cargo.lock`.
  - *Verification Evidence*: `Cargo.toml:16`, `Cargo.lock` verified; clean compilation in `cargo test`.
- [x] Add standard publishing crate metadata to `Cargo.toml`.
  - *Acceptance Criteria*: `description`, `license`, `repository`, `categories`, and `keywords` present.
  - *Verification Evidence*: `Cargo.toml:5-11`.
- [x] Isolate live network ping tests from standard offline test runs.
  - *Acceptance Criteria*: `test_all_regions_ping` marked with `#[ignore]` so default test runs do not contact external AWS endpoints.
  - *Verification Evidence*: `src/ping.rs:252`; `cargo test` executes 87 tests and 1 ignored test offline.

#### Milestone 2: Region Validation, CLI Ergonomics & Elevation Security
- [x] Implement strict CLI region token validation in `src/api.rs` and `src/main.rs`.
  - *Acceptance Criteria*: Unrecognized region tokens emit actionable error messages to `stderr` and exit with non-zero code `1`.
  - *Verification Evidence*: `src/api.rs:341` (`resolve_region_names`), `src/main.rs:94-106`; unit tests `api::tests::test_resolve_region_names` and `api::tests::test_resolve_to_aws_codes` pass.
- [x] Prevent accidental matchmaking blackout on empty resolved region set.
  - *Acceptance Criteria*: Refuse to lock `0.0.0.0` to all endpoints when no valid regions are provided; report error to `stderr` and exit `1`.
  - *Verification Evidence*: `src/main.rs:104-109`.
- [x] Propagate non-zero exit codes on hosts elevation failures and write errors.
  - *Acceptance Criteria*: `UpdateHostsResult::ElevationFailed` and `UpdateHostsResult::Error` emit to `stderr` and terminate with exit code `1`.
  - *Verification Evidence*: `src/main.rs:114-122` and `src/main.rs:141-149`.
- [x] Propagate non-zero exit code on interactive lock modal cancellation.
  - *Acceptance Criteria*: Pressing `Esc` or `q` in interactive modal returns `None`, logs `"Cancelled."` to `stderr`, and exits code `1`.
  - *Verification Evidence*: `src/main.rs:98-101`; unit test `app::tests::test_quit_ladder_main_and_modal` passes.
- [x] Propagate non-zero exit code on configuration save failure.
  - *Acceptance Criteria*: When saving configuration fails during CLI flag updates, log error to `stderr` and exit code `1`.
  - *Verification Evidence*: `src/main.rs:60-63, 73-76, 85-88`.
- [x] Add version output (`--version`, `-V`) to the root CLI parser.
  - *Acceptance Criteria*: `dbdq --version` and `dbdq -V` display the crate version and exit with code `0`.
  - *Verification Evidence*: `src/main.rs:24` (`#[command(version)]`); unit test `tests::test_cli_version_flag` passes.
- [x] Strongly type CLI enum arguments via `clap::ValueEnum`.
  - *Acceptance Criteria*: `SortOrder`, `GameMode`, and `TimeFormat` derive `clap::ValueEnum`, replacing manual string lowercasing.
  - *Verification Evidence*: `src/config.rs:15, 33, 44`; unit tests `config::tests::test_value_enum_roundtrip_and_aliases`, `tests::test_cli_sort_order_parsing`, `tests::test_cli_game_mode_parsing`, and `tests::test_cli_time_format_parsing` pass.
- [x] Harden Windows PowerShell elevation commands against command injection.
  - *Acceptance Criteria*: File paths interpolated into PowerShell arguments escape single quotes (`''''`) and utilize `-LiteralPath`.
  - *Verification Evidence*: `src/hosts.rs:197-227`; unit tests `hosts::tests::test_escape_powershell_single_quotes` and `hosts::tests::test_build_windows_elevation_command_format` pass.
- [x] Harden Linux hosts file updates with direct write attempt.
  - *Acceptance Criteria*: On Linux, attempt direct `std::fs::write` to `/etc/hosts` first (for root/sudo) before spawning `pkexec`.
  - *Verification Evidence*: `src/hosts.rs:247-268` (`write_hosts_file_unix`).

#### Milestone 3: TUI Design Overhaul & Terminal Panic Guard
- [x] Install RAII `TerminalGuard` and panic recovery hook in `src/tui.rs`.
  - *Acceptance Criteria*: Raw mode disabled, alternate screen vacated, and cursor restored cleanly on both normal exit and application panic.
  - *Verification Evidence*: `src/tui.rs:30-80`; unit tests `tui::tests::test_install_panic_hook_idempotent`, `tui::tests::test_terminal_guard_drop_idempotent`, and `tui::tests::test_restore_terminal_does_not_panic` pass.
- [x] Complete TUI clutter audit and eliminate nested boxes.
  - *Acceptance Criteria*: Single-line outer framing, removal of heavy nested rectangular blocks, and elimination of high-contrast bracket highlights.
  - *Verification Evidence*: `src/ui.rs:26-34, 160-240, 600-660`; unit test `ui::tests::test_draw_with_full_regions_on_80x24_terminal` passes.
- [x] Implement semantic color tokens (`Theme`) with `NO_COLOR` environment support.
  - *Acceptance Criteria*: Color palette respects functional roles; when `NO_COLOR` is present, colors fall back to monochrome/ASCII without breaking layout.
  - *Verification Evidence*: `src/ui.rs:36-120`; unit test `ui::tests::test_no_color_theme` passes.
- [x] Implement responsive viewport ladder with floor fallback.
  - *Acceptance Criteria*: Full 80x24 standard layout, 60-col narrow tmux split with prioritized columns, and dedicated fallback screen below 48x12.
  - *Verification Evidence*: `src/ui.rs:600-650`; unit tests `ui::tests::test_narrow_layout_60x24` and `ui::tests::test_terminal_floor_under_48x12` pass.
- [x] Implement vim navigation and standardized `Esc`/`q` quit ladder.
  - *Acceptance Criteria*: Support `j`/`k` and `g`/`G` in table and modal; `Esc` dismisses modals/notices without quitting; `q` cleanly quits at root.
  - *Verification Evidence*: `src/app.rs:370-490`; unit tests `app::tests::test_vim_keys_table_navigation` and `app::tests::test_quit_ladder_main_and_modal` pass.

---

## Current phase: Feature Planning, CI Automation & Extended Platform Support

- [x] Author authoritative `SPEC.md` strictly adhering to titus-ai conventions with grounded feature proposals (`--auto`, `--json`, `watch`, role profiles).
  - *Acceptance Criteria*: File exists at repository root; contains problem definition, users, architecture diagram, security boundaries, non-goals, and concrete proposals.
  - *Verification Evidence*: `SPEC.md` authored and verified at repository root.
- [x] Author phased `ROADMAP.md` following titus-ai structure.
  - *Acceptance Criteria*: File exists at repository root; outlines Phases 1 through 4 with objectives, deliverables, concrete risks, mitigations, and exit criteria.
  - *Verification Evidence*: `ROADMAP.md` authored and verified at repository root.
- [x] Author actionable `TASKS.md` following titus-ai structure.
  - *Acceptance Criteria*: File exists at repository root; documents completed tasks with verification pointers and planned tasks with checkboxes.
  - *Verification Evidence*: `TASKS.md` authored and verified at repository root.
- [ ] Configure GitHub Actions CI workflow for Linux and Windows runners (`cargo test`, `cargo clippy`, `cargo fmt`).
- [ ] Execute Milestone 5: End-to-end integration verification and forensic audit.

---

## Future phases

### Phase 3: Automated Best-Region Locking & Structured Machine Output
- [ ] Implement `dbdq lock --auto` flag with deterministic region selection based on `--max-ping` and queue times.
- [ ] Implement fallback notification logic when all server pings exceed the configured latency threshold.
- [ ] Implement `dbdq --json` output stream formatting full telemetry and recommendations to `stdout`.
- [ ] Implement newline-delimited JSON (NDJSON) streaming under `dbdq --json --watch <interval>`.
- [ ] Add unit tests for `--auto` deterministic selection across various network conditions.
- [ ] Add schema validation tests for `--json` output structure.

### Phase 4: Matchmaking Profiles & Queue Alerts
- [ ] Extend `Config` data model to support independent `profiles` for Killer and Survivor roles.
- [ ] Implement CLI subcommands `dbdq profile set`, `dbdq profile apply`, and alias `dbdq switch <killer|survivor>`.
- [ ] Add TUI keybinding (`P`) to toggle between Survivor and Killer profiles with instant hosts file update.
- [ ] Implement `dbdq watch --threshold <secs>` monitoring daemon.
- [ ] Integrate audible terminal bell (`\x07`) and native desktop notification triggers.
- [ ] Implement stateful debouncing and a 120-second alert cooldown.

---

## Completion rule

Move current-phase tasks to completed only after their acceptance criteria pass, code builds cleanly, unit tests pass without errors, and clippy passes with zero warnings. Verify working code before marking tasks complete.
