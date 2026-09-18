# VHS Tape: Dead By Queue (dbdq) Demo
# Documentation & Best Practices: https://github.com/charmbracelet/vhs

# Output declaration
Output assets/demo.gif

# Terminal Settings
Set Shell "powershell"
Set FontSize 15
Set FontFamily "JetBrains Mono"
Set Theme "Catppuccin Mocha"
Set Width 1100
Set Height 640
Set WindowBar "Colorful"
Set Framerate 30
Set LoopOffset 50%
Set PlaybackSpeed 1.0
Set TypingSpeed 60ms
Set Padding 20

# Setup: quietly configure environment and clean prompt
Hide
Type "$binary = (Resolve-Path './target/release/dbdq.exe').Path; function dbdq { & $binary @args }; Clear-Host"
Enter
Sleep 600ms
Show

# 1. Natural CLI invocation
Type "dbdq"
Sleep 500ms
Enter

# 2. Wait for live dashboard table and AWS latency probes to render
Sleep 3s

# 3. Interactive table navigation
Down@300ms 3
Sleep 1s
Up@300ms 2
Sleep 1s

# 4. Cycle sorting rules (s)
Type "s"
Sleep 1200ms
Type "s"
Sleep 1200ms

# 5. Toggle game mode filter (m: Standard / Event)
Type "m"
Sleep 1500ms
Type "m"
Sleep 1200ms

# 6. Trigger manual background refresh (r) - shows braille spinner & confirmation
Type "r"
Sleep 2500ms

# 7. Preview AWS Matchmaking Region Locker modal (l)
Type "l"
Sleep 1s
Down@300ms 2
Sleep 800ms
Up@300ms 1
Sleep 800ms

# Safely close modal without modifying hosts or requiring UAC
Escape
Sleep 1200ms

# 8. Cleanly exit the dashboard
Escape
Sleep 1500ms
