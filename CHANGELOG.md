# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.9.0] - 2026-09-18

### Added
- Persistent region ping caching: AWS ping measurements are cached in `cache.json` in the user configuration directory (`%APPDATA%\dbdqueue` or `~/.config/dbdqueue`).
- Instant startup: ping cache is loaded immediately on launch, allowing the Best Pick summary and region pings to display on the very first frame without the ~3-second delay.
- Stale-while-revalidate background updates: live ping measurements continue in the background and silently update values when ready.
- Timeout-resilient ping merging: retains previously known pings if an individual AWS region times out or drops packets during background checks.
- Accelerated region locker: `dbdq lock` interactive CLI menu loads cached pings directly, opening immediately without waiting for ping checks.

### Changed
- Refined ping update handlers (`handle_ping_update` and `handle_manual_refresh_complete`) to merge results incrementally rather than overwriting.

## [0.8.1] - 2026-09-18

### Fixed
- Fixed event countdown rounding in `~Rounded` mode: durations now round to the nearest unit with round-half-up logic (e.g. 10 days and 23 hours rounds to `11d`, 2 days and 12 hours rounds to `3d`).

## [0.8.0] - 2026-09-18

### Added
- In-game event integration via deadbyqueue.com `/misc` API endpoint (`currentEvents` and `upcomingEvents`).
- Inline event badge in top header displayed next to `Mode: Event` (`[<Name> - <countdown> left]`, e.g. `[2v8 event - 11d left]`), visible only when Event mode is active.
- Event countdown timer formatting adhering to active `Time` toggle (`Exact` vs `~Rounded`).
- Local machine timezone conversion (`chrono::Local`) for all event dates and timestamps.
- Upcoming event placeholder in Event mode table when no active queues are running (`Upcoming: <Name> (starts <Date> - in <countdown>)`).
- Background auto-refresh worker (60s loop) and manual refresh (`[R]`) integration for event data.
- English and Russian translations for all event strings and badges.

## [0.7.0] - 2026-09-16

### Added
- Queue duration format toggle (`[T]` key, with Cyrillic `'е'`/`'Е'` support):
  - **Exact Mode** (default): Displays precise queue durations formatted as `{m}:{s:02}` (e.g. `0:01`, `0:53`, `4:54`, `30:52`) for waits under 1 hour and `{h}:{m:02}:{s:02}` (e.g. `1:52:31`) for waits 1 hour or longer.
  - **Rounded Mode**: Displays simplified, rounded durations: `{s}s` (e.g. `1s`, `53s`) for waits under 60 seconds, `{m}m` (e.g. `5m`, `31m`) rounded to the nearest minute, and `{h}h` (e.g. `1h`, `2h`) rounded to the nearest hour.
- Header time format status indicator placed directly after `Mode:`: `Time: Exact` (RU: `Время: Точно`) and `Time: ~Rounded` (RU: `Время: ~Округлённо`).
- Footer keybinding shortcut indicator `[T] Time` (RU: `[T] Время`) placed directly after `[M] Mode`.
- CLI argument `-t, --time <TIME>` (`exact` / `rounded`) to set the initial queue duration display mode.
- Persistent `time_format` option in `config.toml` (`"exact"` or `"rounded"`).

## [0.6.5] - 2026-09-16

### Changed
- Improved queue duration display: formatted as `Xs` for durations < 60s, `m:ss` (`1:00` to `59:59`) for multi-minute waits, and `1h+` / `2h+` for long queues, stabilizing column alignment and eliminating cramped letter strings.
- Refined summary panel title from "Best pick now" / "Лучший выбор сейчас" to "Best Pick" / "Лучший выбор".

### Fixed
- Fixed layout height allocation on standard 80x24 (and >= 14) terminals to guarantee the Best Pick summary panel remains visible without disappearing when full region lists are loaded.
- Refactored `dbdq lock` CLI interactive menu to use Ratatui on `EnterAlternateScreen`, eliminating terminal clear-screen flickering, screen scrolling, downward shifting, and scrollback pollution.
- Added `ListState` scrolling to the region lock modal to ensure the selected region is always visible on compact terminal screens.

## [0.6.4] - 2026-09-15

### Changed
- Refactored `GameMode` into a cohesive binary state (`Standard` / `Event`) with dedicated `toggle()` method; existing configs with `mode = "both"` deserialize into `GameMode::Standard` for backwards compatibility.
- Deepened region resolution module into `api.rs` (`resolve_region_names` and `resolve_to_aws_codes`), eliminating raw string parsing and lookups in `main.rs`.

### Removed
- Removed deprecated `GameMode::Both` variant and `--mode both` CLI option.
- Removed legacy `priority` feature: `--priority` CLI flag, `config.priority` field, and `interactive_priority_menu()` from `hosts.rs`. Existing config files containing `priority` continue loading without error.

## [0.6.3] - 2026-09-15

### Added
- Scalable vector graphics dashboard mockup (`assets/dashboard.svg`).
- Cross-platform release cleanup scripts (`scripts/cleanup-releases.ps1`, `scripts/cleanup-releases.sh`) to prune obsolete binary assets from GitHub releases.
- Open-source project LICENSE file.

### Changed
- Upgraded multi-platform release workflow (`release.yml`) with automated SHA256 checksum generation, changelog extraction, and legacy asset cleanup.
- Hardened CI (`ci.yml`) and Release (`release.yml`) workflows with concurrency control, least-privilege permissions, and execution timeouts.
- Overhauled README documentation with architecture details, latency formulas, badges, and project navigation.

## [0.6.2] - 2026-09-05

### Changed
- Reworked 'Best pick' recommendation algorithm to balance queue wait time and latency using a role-aware cost function (Killer vs Survivor) rather than a rigid greedy queue-time window.
- Excluded unmeasured ping regions from Best pick recommendations until latency is measured.
- Refined `+N similar` metric to require both latency proximity (<= 25 ms) and queue time proximity (<= 15 s).
- Removed redundant `Ping:` row from the 'Best pick now' summary panel.
- Tightly constrained summary panel height to 4 lines (content + borders) ending immediately after the Survivor row, keeping extra space outside below the panel.

### Fixed
- Aligned summary panel rows into consistent columns (`label`, `time`, `region`, `ping`, `similar`).
- Cleaned up ping formatting in summary panel to display clean values (e.g. `76 ms`) without parentheses.

## [0.6.1] - 2026-09-05

### Added
- Added 'best pick' summary panel below table showing top killer/survivor queue times and lowest ping region with i18n support (En/Ru).

### Fixed
- Fixed table layout: fixed column widths and horizontal table centering inside terminal chunks.
- Fixed Linux dead_code warning on `UpdateHostsResult::ElevationFailed`.

## [0.6.0] - 2026-09-04

### Changed
- Refactored `app.rs` architecture: separated state machine (`app.rs`), rendering (`ui.rs`), and localization (`i18n.rs`).
- `App` is now a pure state machine with zero disk, network, or hosts I/O; all side-effects are communicated via `AppAction` and handled in the event loop.
- Replaced string representations for sort order and game mode with typed enums (`SortOrder`, `GameMode`) in `config.rs`, backed by backward-compatible deserialization and exhaustive pattern matching.
- Optimized performance hot paths: row data is referenced rather than cloned each frame, timestamps are parsed once via cached sorting keys, and static region maps are cached using `LazyLock`.
- Locked regions stored in a `HashSet` for $O(1)$ lookup complexity.
- Hosts updates now execute off the UI thread so Windows UAC elevation prompts do not freeze UI rendering.
- Layout responsiveness: removed full-screen `Clear`, clamped table height dynamically to available screen space to prevent pushing the footer off small terminals, and replaced magic numbers with named constants.

### Added
- Centralized `i18n.rs` localization supporting English and Russian, with configurable `lang` in `AppConfig` (`auto`, `en`, `ru`) resolving system locale via `sys-locale` and environment variables (`LC_ALL` > `LC_MESSAGES` > `LANG`).
- Unified `Notice` system (`Error`, `Info`, `Success`) with injected-time expiration testing.

### Fixed
- **UX Fix 1**: Renamed main dashboard footer label `[↑↓] Scroll` to `[↑↓] Select` (and Russian `[↑↓] Выбор`) for consistency with the lock modal.
- **UX Fix 2**: Styled modal action labels with red brackets and default foreground text, preventing border color bleed.
- **UX Fix 3**: Auto-expired hosts update notices ("Hosts file is up to date" / "Region locks updated!") after 3 seconds, returning status to "API Updated".
- Fixed unlocalized "Error: " prefix in the footer status bar for Russian locale.

## [0.5.4] - 2026-09-03

### Fixed
- Filtered out disabled matchmaking servers from both the `[L]` Region Locker modal and `dbdq lock` CLI menu.
- Fixed hotkey text color in the Region Locker modal and CLI interactive menu to match default terminal text color.
- Cleaned up Region Locker modal title by removing `(select whitelisted)` text.
- Added missing space after region flag brackets in `dbdq lock` CLI menu.

## [0.5.3] - 2026-09-03

### Changed
- Rebranded project to **Dead By Queue** with updated titles and clean CLI/TUI descriptions across the codebase.
- Changed theming: red for frames, titles, and accents, white for table headers.
- Removed Lock column from the queue table, relying on accent-highlighted region names and header lock status.
- Redesigned top header into a resilient, compact box (`Sort: ... │ Mode: ... │ Lock: ...`) with capitalized Mode display and defensive global lock status.
- Consolidated table sorting into a single `[S]` key that smoothly cycles through `Killer` → `Survivor` → `Ping`.

### Added
- Fully non-blocking background refresh flow on `[R]` keypress: API queue fetch and AWS ping measurements now run in parallel without freezing the TUI event loop.
- Live animated braille spinner (`⠋ Fetching...`) in the status bar while a refresh is in-flight.
- Transient feedback indicators (`[✓ Up to date]` / `[✓ Updated]`) automatically reverting to true source data age (`API Updated: Xm ago`) after 2.5 seconds.

## [0.5.2] - 2026-09-03

### Fixed
- Fixed inflated ping numbers: replaced cold HTTPS HEAD requests (which suffered from multi-RTT TLS handshake and DNS overhead) with warm HTTP/1.1 Keep-Alive requests over the established TLS pool, measuring exact 1-RTT network latency identical to native ICMP ping.

## [0.5.1] - 2026-09-03

### Fixed
- Fixed fake 0–3 ms ping measurements caused by local TUN/transparent proxy drivers intercepting raw TCP SYN packets; pings now use end-to-end TLS/HTTPS HEAD requests to measure true round-trip latency to AWS datacenters.
- Enhanced API error diagnostics: when receiving an HTML block page (ISP block/TSPU, Cloudflare challenge) or empty response, clear status codes and body snippets are displayed instead of cryptic JSON parser errors.
- Prevented TUI table collapse when API fetch fails; added informative error/loading placeholder rows.

### Added
- Added `[R]` keyboard shortcut (and Cyrillic `[К]`) to trigger an immediate background refresh of queue times and ping measurements.
- Added support for custom API mirror endpoints via `DBD_API_URL` environment variable and `api_url` in `config.toml`.
- Added automatic support for standard HTTP/HTTPS/ALL proxy environment variables.

## [0.5.0] - 2026-09-02

### Added
- Multi-layout keyboard support: all keyboard shortcuts now work seamlessly across any layout, including Russian (Cyrillic to QWERTY mapping).
- Ping-based sorting (`--sort ping`, `P` shortcut in TUI), replacing the legacy priority sort.
- Region Locker modal is now always sorted by ping ascending (lowest ping first).

### Fixed
- Fixed bug where raw `println!` messages from hosts updater leaked onto the TUI screen and broke layout boundaries.
- Replaced quit shortcut `q` with `Esc` for both the main dashboard and closing modals.
- Cleaned up table clutter by removing repetitive `[BLOCKED]` indicators, displaying only `[LOCKED]` on whitelisted regions.
- Regions with no active matchmaking queues are now completely dimmed in dark gray, with unmeasured ping hidden (`—`).

### Performance
- Fixed sluggish navigation when holding arrow keys on Windows by supporting `KeyEventKind::Repeat` and batch-draining pending input events before redraws.
- Replaced dynamic regex parsing with an optimized, zero-allocation time parser.

## [0.3.1] - 2026-06-15

### Fixed
- Fixed an issue with the 0.3.0 release where several modified files (like `README.md`, `install.sh`, `src/tui.rs`, etc.) were not included in the git commit, resulting in a broken release build. This patch deploys all remaining files from the major refactor.

## [0.3.0] - 2026-06-15

### Added
- Complete rewrite of the terminal user interface using `ratatui`, replacing the old interactive raw-mode menu with a full-screen, scrollable dashboard.
- Real-time API fetching loop in the background while the TUI is active.
- Restored automatic language detection (English/Russian) for UI elements based on system environment variables (`LANG`, `LC_ALL`, `LC_MESSAGES`) in the new TUI.

### Removed
- Removed the region locking feature (`/etc/hosts` modification via `pkexec tee`) and its interactive lock menu, simplifying the application scope to queue time monitoring.

## [0.1.5] - 2026-06-15

### Fixed
- Fixed GitHub Actions release workflow to extract correct release notes from `CHANGELOG.md` instead of using a hardcoded placeholder for every release.

## [0.1.4] - 2026-06-15

### Added
- Implemented automatic language detection (English/Russian) for UI elements based on system environment variables (`LANG`, `LC_ALL`, `LC_MESSAGES`).
- Added Russian translations for table headers, mode labels, relative time strings, and interactive menus.

## [0.1.3] - 2026-06-15

### Fixed
- Localized the API update timestamp display to Russian and formatted it as a relative time (e.g., "Обновлено: X мин. назад").

## [0.1.2] - 2026-06-15

### Fixed
- Updated domain block IP strategy from `0.0.0.0` to `127.0.0.1` and fixed related tests.

## [0.1.1] - 2026-06-15

### Fixed
- Fixed interactive TUI alignment issues and added Windows compatibility enhancements.

## [0.1.0] - 2026-06-12

### Added
- Complete Rust port of the original Python script `dbdqueue.py`.
- Native binary compilation resulting in a fast, zero-dependency executable.
- Dynamic table display with ANSI colors showing live survivor and killer queue times.
- Sorting options (`survivor`, `killer`, `priority`, `default`).
- Filtering options for game modes (`standard`, `event`, `both`).
- Priority region whitelisting displaying preferred regions at the top of the table.
- Region locking and unlocking by editing `/etc/hosts` safely using `pkexec tee`.
- Interactive raw-mode configuration menus using `crossterm` for choosing locked and priority regions.
- Automated TOML configuration migration from legacy JSON format.
- GitHub Actions CI/CD workflow for automated binary builds and release generation.
- Automated installation shell script (`install.sh`).
