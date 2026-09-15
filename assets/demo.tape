# Charmbracelet VHS recording script for Dead By Queue (dbdq)
# Best practices: Output first, early Require, ordered settings, explicit sleep units

# 1. Output specification
Output assets/demo.gif

# 2. Dependency validation
Require dbdq

# 3. Terminal & visual configuration
Set Shell "bash"
Set FontSize 14
Set FontFamily "JetBrains Mono"
Set Width 1000
Set Height 600
Set Padding 20
Set TypingSpeed 60ms
Set PlaybackSpeed 1.0
Set Theme "Catppuccin Mocha"

# 4. Recording scenario
# Launch dbdq interactive TUI
Type "dbdq"
Sleep 500ms
Enter

# Observe live dashboard & initial AWS ping measurements
Sleep 3s

# Cycle sorting mode (Default -> Killer -> Survivor -> Ping)
Type "s"
Sleep 1500ms
Type "s"
Sleep 1500ms

# Toggle matchmaking mode (Standard / Event)
Type "m"
Sleep 1500ms
Type "m"
Sleep 1500ms

# Trigger non-blocking background refresh with live spinner
Type "r"
Sleep 2500ms

# Open Region Locker modal
Type "l"
Sleep 1500ms

# Navigate down and toggle a region whitelist
Down 2
Sleep 600ms
Space
Sleep 800ms

# Close modal
Escape
Sleep 1500ms

# Exit dashboard
Escape
Sleep 1s

# Final pause before loop
Sleep 2s
