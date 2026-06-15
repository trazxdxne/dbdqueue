# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
