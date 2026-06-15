# Design Spec: `dbdqueue` (CLI Tool)

## Problem Statement
How Might We create a fast, zero-dependency, and highly compatible terminal command for Dead by Daylight players to view queue times and manage region locks directly from their terminal?

## Recommended Direction
A compiled native binary (`dbdqueue`) that runs as a standard command-line utility. 
- Running `dbdqueue` launches a beautiful full-screen interactive dashboard using `ratatui` that fetches and displays live queue times, with background auto-refreshing.
- It supports command-line flags and interactive keybindings for sorting (by survivor times, killer times, or priority list), toggling mode, and respects a configuration file (`~/.config/dbdqueue/config.toml`).

---

## MVP Scope (What's In / What's Out)

### **In Scope (MVP):**
1. **Main command:** `dbdqueue` (launches the ratatui dashboard).
2. **Sorting/Filtering features:**
   - Interactive keybindings (`s` for survivor, `k` for killer, `r` for priority, `m` for mode toggle)
   - CLI flags: `--sort survivor`, `--mode standard`
3. **Configuration file:** `~/.config/dbdqueue/config.toml` (saves priority regions, mode, and sorting state).
