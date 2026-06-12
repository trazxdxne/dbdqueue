# Design Spec: `dbdqueue` (CLI Tool)

## Problem Statement
How Might We create a fast, zero-dependency, and highly compatible terminal command for Dead by Daylight players to view queue times and manage region locks directly from their terminal?

## Recommended Direction
A compiled native binary (`dbdqueue`) that runs as a standard command-line utility. 
- Running `dbdqueue` fetches live queue times and prints a beautifully formatted, ANSI-colored table of live queue times.
- It supports command-line flags for sorting (by survivor times, killer times, or priority list) and respects a configuration file (`~/.config/dbdqueue/config.toml`) for a custom priority region list.
- Region locking is managed via subcommands (`lock` / `unlock`) that interface with `pkexec` to elevate privileges on-demand only when `/etc/hosts` changes.

---

## MVP Scope (What's In / What's Out)

### **In Scope (MVP):**
1. **Main command:** `dbdqueue` (fetches and prints queue times).
2. **Sorting flags:**
   - `--sort survivor` (or `-s survivor`)
   - `--sort killer` (or `-s killer`)
   - `--sort priority` (orders by the user's priority list in the config)
3. **Locking subcommands:**
   - `dbdqueue lock <regions...>` (updates hosts file using `pkexec tee`)
   - `dbdqueue unlock` (restores original hosts file)
4. **Configuration file:** `~/.config/dbdqueue/config.toml` (saves priority regions and last selected lock configuration).
