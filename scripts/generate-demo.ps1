<#
.SYNOPSIS
    Builds the Dead By Queue release binary and records a fresh demo GIF using Charmbracelet VHS.

.DESCRIPTION
    Verifies that required dependencies (vhs, ffmpeg, cargo) are available, offering to install
    missing tools via winget if needed. Compiles the project with `cargo build --release` and
    runs `vhs assets/demo.tape` to produce `assets/demo.gif`.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .\scripts\generate-demo.ps1
#>

[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [switch]$AutoInstall
)

$ErrorActionPreference = "Stop"

# Ensure script executes from repository root
$RepoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
Set-Location $RepoRoot

Write-Host "==> Dead By Queue Demo Generator" -ForegroundColor Cyan
Write-Host "    Repository root: $RepoRoot" -ForegroundColor DarkGray

# Helper: refresh environment PATH from Machine and User registry
function Refresh-SessionPath {
    $machinePath = [System.Environment]::GetEnvironmentVariable("Path", "Machine")
    $userPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
    $wingetLinks = Join-Path $env:LOCALAPPDATA "Microsoft\WinGet\Links"
    $env:Path = "$userPath;$machinePath;$wingetLinks;$env:Path"
}

# Initialize session PATH with latest registry values
Refresh-SessionPath

# Check for required tools
$missingTools = [System.Collections.Generic.List[string]]::new()

if (-not (Get-Command "vhs" -ErrorAction SilentlyContinue)) {
    $missingTools.Add("charmbracelet.vhs")
}
if (-not (Get-Command "ffmpeg" -ErrorAction SilentlyContinue)) {
    $missingTools.Add("Gyan.FFmpeg.Essentials")
}
if (-not (Get-Command "ttyd" -ErrorAction SilentlyContinue)) {
    $missingTools.Add("tsl0922.ttyd")
}

if ($missingTools.Count -gt 0) {
    Write-Host "`n[!] Missing prerequisite recording tools: $($missingTools -join ', ')" -ForegroundColor Yellow

    $shouldInstall = $false
    if ($AutoInstall) {
        $shouldInstall = $true
    } else {
        $answer = Read-Host "Would you like to install the missing tools using winget now? [Y/n]"
        if ([string]::IsNullOrWhiteSpace($answer) -or $answer.Trim().ToLower() -eq 'y') {
            $shouldInstall = $true
        }
    }

    if ($shouldInstall) {
        foreach ($toolId in $missingTools) {
            Write-Host "==> Installing $toolId via winget..." -ForegroundColor Cyan
            winget install --id $toolId -e --accept-package-agreements --accept-source-agreements
            if ($LASTEXITCODE -ne 0) {
                Write-Warning "Installation of $toolId exited with code $LASTEXITCODE."
            }
        }
        Refresh-SessionPath
    } else {
        Write-Error "Cannot generate VHS demo without prerequisite tools ($($missingTools -join ', ')). Aborting."
        exit 1
    }
}

# Re-verify tools in PATH
if (-not (Get-Command "vhs" -ErrorAction SilentlyContinue)) {
    Write-Error "VHS is still not recognized in PATH. Please restart your shell or check your PATH configuration."
    exit 1
}
if (-not (Get-Command "ffmpeg" -ErrorAction SilentlyContinue)) {
    Write-Error "FFmpeg is still not recognized in PATH. Please restart your shell or check your PATH configuration."
    exit 1
}

# Verify cargo is available
if (-not (Get-Command "cargo" -ErrorAction SilentlyContinue)) {
    Write-Error "Cargo was not found in PATH. Rust is required to build the release binary."
    exit 1
}

# Build release binary unless skipped
if (-not $SkipBuild) {
    Write-Host "`n==> Compiling Dead By Queue release binary (cargo build --release)..." -ForegroundColor Cyan
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        Write-Error "Cargo build failed with exit code $LASTEXITCODE."
        exit $LASTEXITCODE
    }
}

$binaryPath = Join-Path $RepoRoot "target\release\dbdq.exe"
if (-not (Test-Path $binaryPath)) {
    Write-Error "Binary not found at expected path: $binaryPath"
    exit 1
}

# Run VHS
$tapePath = Join-Path $RepoRoot "assets\demo.tape"
if (-not (Test-Path $tapePath)) {
    Write-Error "VHS tape not found at expected path: $tapePath"
    exit 1
}

Write-Host "`n==> Recording terminal demo using VHS..." -ForegroundColor Cyan
Write-Host "    Tape: assets/demo.tape" -ForegroundColor DarkGray
vhs "assets/demo.tape"

if ($LASTEXITCODE -ne 0) {
    Write-Error "VHS recording failed with exit code $LASTEXITCODE."
    exit $LASTEXITCODE
}

$outputPath = Join-Path $RepoRoot "assets\demo.gif"
if (Test-Path $outputPath) {
    $fileItem = Get-Item $outputPath
    $sizeKb = [math]::Round($fileItem.Length / 1KB, 1)
    $sizeMb = [math]::Round($fileItem.Length / 1MB, 2)
    Write-Host "`n[✓] Successfully generated VHS demo!" -ForegroundColor Green
    Write-Host "    Output: $outputPath" -ForegroundColor Green
    Write-Host "    Size  : $($sizeKb) KB ($($sizeMb) MB)" -ForegroundColor DarkGray
} else {
    Write-Warning "VHS completed, but output file was not found at $outputPath."
}
