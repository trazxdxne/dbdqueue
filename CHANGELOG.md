# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.6.2] - 2026-09-05

### Changed
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
