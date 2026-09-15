<#
.SYNOPSIS
    Dead By Queue — GitHub Releases Cleanup & Repackaging Utility (PowerShell)

.DESCRIPTION
    Deletes legacy pre-v0.6.0 releases and orphaned tags, or repackages v0.6.x releases
    with standardized archives (.tar.gz, .zip) and SHA256SUMS.txt.

.PARAMETER DryRun
    Preview actions without deleting or uploading anything.

.PARAMETER DeleteLegacy
    Delete legacy pre-v0.6.0 GitHub releases and remote tags.

.PARAMETER RepackageV06
    Download existing v0.6.x binaries, build .tar.gz/.zip archives with README & LICENSE,
    generate SHA256SUMS.txt, and upload them to GitHub Releases.

.PARAMETER PruneAssets
    Prune obsolete legacy assets (dbdqueue-* binaries, install.ps1, install.sh) from GitHub releases.

.PARAMETER All
    Execute -DeleteLegacy, -RepackageV06, and -PruneAssets.

.EXAMPLE
    .\scripts\cleanup-releases.ps1 -DryRun -All
    .\scripts\cleanup-releases.ps1 -DeleteLegacy
    .\scripts\cleanup-releases.ps1 -RepackageV06
    .\scripts\cleanup-releases.ps1 -PruneAssets
#>

[CmdletBinding()]
param (
    [switch]$DryRun,
    [switch]$DeleteLegacy,
    [switch]$RepackageV06,
    [switch]$PruneAssets,
    [switch]$All,
    [string]$Repo = "trazxdxne/dbdqueue"
)

$ErrorActionPreference = 'Stop'

if ($All) {
    $DeleteLegacy = $true
    $RepackageV06 = $true
    $PruneAssets = $true
}

if (-not $DeleteLegacy -and -not $RepackageV06 -and -not $PruneAssets) {
    Write-Host "No action specified. Run with -DeleteLegacy, -RepackageV06, -PruneAssets, or -All." -ForegroundColor Yellow
    Write-Host "Use Get-Help .\scripts\cleanup-releases.ps1 -Detailed for help."
    exit 1
}

# Verify gh CLI when not in dry-run mode
if (-not $DryRun) {
    $ghPath = Get-Command gh -ErrorAction SilentlyContinue
    if (-not $ghPath) {
        Write-Error "GitHub CLI (gh) was not found on PATH. Please install it from https://cli.github.com/ or 'winget install GitHub.cli'."
        exit 1
    }

    try {
        gh auth status 2>&1 | Out-Null
    } catch {
        Write-Error "gh CLI is not authenticated. Please run 'gh auth login' first."
        exit 1
    }
}

Write-Host "==> Target repository: $Repo" -ForegroundColor Cyan
if ($DryRun) {
    Write-Host "==> DRY-RUN MODE: No changes will be made to GitHub." -ForegroundColor Yellow
}

# 1. Delete Legacy Pre-v0.6.0 Releases and Tags
$legacyTags = @(
    "v0.1.0", "v0.1.1", "v0.1.2", "v0.1.3", "v0.1.4", "v0.1.5",
    "v0.3.0", "v0.3.1", "v0.5.0", "v0.5.1", "v0.5.2", "v0.5.3", "v0.5.4"
)

if ($DeleteLegacy) {
    Write-Host "`n----------------------------------------------------------------------" -ForegroundColor DarkGray
    Write-Host "==> Processing Legacy Pre-v0.6.0 Releases..." -ForegroundColor Cyan
    Write-Host "----------------------------------------------------------------------" -ForegroundColor DarkGray

    foreach ($tag in $legacyTags) {
        if ($DryRun) {
            Write-Host "[DRY-RUN] Would delete GitHub release: $tag" -ForegroundColor Yellow
            Write-Host "[DRY-RUN] Would delete remote Git tag: refs/tags/$tag" -ForegroundColor Yellow
        } else {
            Write-Host "Checking release $tag..." -ForegroundColor Gray
            $check = gh release view $tag --repo $Repo 2>&1
            if ($LASTEXITCODE -eq 0) {
                Write-Host "Deleting release $tag on $Repo..." -ForegroundColor Yellow
                gh release delete $tag --repo $Repo --yes
            }

            Write-Host "Deleting remote tag $tag..." -ForegroundColor Gray
            git push --delete origin $tag 2>$null
        }
    }
}

# 2. Repackage v0.6.x Releases
$v06Tags = @("v0.6.0", "v0.6.1", "v0.6.2")

if ($RepackageV06) {
    Write-Host "`n----------------------------------------------------------------------" -ForegroundColor DarkGray
    Write-Host "==> Repackaging v0.6.x Releases (.tar.gz, .zip, SHA256SUMS.txt)..." -ForegroundColor Cyan
    Write-Host "----------------------------------------------------------------------" -ForegroundColor DarkGray

    $scriptRoot = $PSScriptRoot
    $repoRoot = Split-Path -Parent $scriptRoot
    $tempDir = Join-Path ([System.IO.Path]::GetTempPath()) "dbdq_repackage_$([System.Guid]::NewGuid().ToString('N'))"
    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

    try {
        $readmePath = Join-Path $repoRoot "README.md"
        $licensePath = Join-Path $repoRoot "LICENSE"

        foreach ($tag in $v06Tags) {
            Write-Host "==> Processing release: $tag" -ForegroundColor Cyan
            $tagDir = Join-Path $tempDir $tag
            New-Item -ItemType Directory -Path $tagDir -Force | Out-Null

            if ($DryRun) {
                Write-Host "[DRY-RUN] Would download binaries for $tag, assemble archives, and upload to $Repo" -ForegroundColor Yellow
                continue
            }

            # Check if release exists
            $check = gh release view $tag --repo $Repo 2>&1
            if ($LASTEXITCODE -ne 0) {
                Write-Host "Notice: Release $tag not found on GitHub. Skipping." -ForegroundColor DarkGray
                continue
            }

            # Download standalone binaries
            Write-Host "Downloading existing standalone binaries for $tag..." -ForegroundColor Gray
            gh release download $tag --repo $Repo --pattern "dbdq-*" --dir $tagDir

            # 1. Package Windows zip
            $winBinary = Join-Path $tagDir "dbdq-windows-x64.exe"
            $winZip = Join-Path $tagDir "dbdq-windows-x64.zip"
            if (Test-Path $winBinary) {
                $staging = Join-Path $tagDir "win_staging"
                New-Item -ItemType Directory -Path $staging -Force | Out-Null
                Copy-Item $winBinary (Join-Path $staging "dbdq.exe")
                Copy-Item $readmePath (Join-Path $staging "README.md")
                Copy-Item $licensePath (Join-Path $staging "LICENSE")
                Compress-Archive -Path "$staging\*" -DestinationPath $winZip -Force
                Remove-Item $staging -Recurse -Force
            }

            # 2. Package Linux tar.gz archives using tar (available in modern Windows)
            $tarCmd = Get-Command tar -ErrorAction SilentlyContinue
            if ($tarCmd) {
                $linuxX64 = Join-Path $tagDir "dbdq-linux-x86_64"
                if (Test-Path $linuxX64) {
                    $staging = Join-Path $tagDir "linux_x64_staging"
                    New-Item -ItemType Directory -Path $staging -Force | Out-Null
                    Copy-Item $linuxX64 (Join-Path $staging "dbdq")
                    Copy-Item $readmePath (Join-Path $staging "README.md")
                    Copy-Item $licensePath (Join-Path $staging "LICENSE")
                    tar -czf (Join-Path $tagDir "dbdq-linux-x86_64.tar.gz") -C $staging dbdq README.md LICENSE
                    Remove-Item $staging -Recurse -Force
                }

                $linuxArm = Join-Path $tagDir "dbdq-linux-aarch64"
                if (Test-Path $linuxArm) {
                    $staging = Join-Path $tagDir "linux_arm_staging"
                    New-Item -ItemType Directory -Path $staging -Force | Out-Null
                    Copy-Item $linuxArm (Join-Path $staging "dbdq")
                    Copy-Item $readmePath (Join-Path $staging "README.md")
                    Copy-Item $licensePath (Join-Path $staging "LICENSE")
                    tar -czf (Join-Path $tagDir "dbdq-linux-aarch64.tar.gz") -C $staging dbdq README.md LICENSE
                    Remove-Item $staging -Recurse -Force
                }
            }

            # 3. Generate SHA256SUMS.txt
            $hashFiles = Get-ChildItem -Path $tagDir -File | Where-Object { $_.Name -like "dbdq*" -and $_.Extension -ne ".txt" }
            $sumsLines = foreach ($f in $hashFiles) {
                $h = Get-FileHash -Path $f.FullName -Algorithm SHA256
                "$($h.Hash.ToLower())  $($f.Name)"
            }
            $sumsPath = Join-Path $tagDir "SHA256SUMS.txt"
            $sumsLines | Out-File -FilePath $sumsPath -Encoding utf8

            Write-Host "Generated SHA256SUMS.txt:" -ForegroundColor Green
            Get-Content $sumsPath

            # 4. Upload to release
            $uploadAssets = Get-ChildItem -Path $tagDir -File | Where-Object {
                $_.Name -in @("dbdq-windows-x64.zip", "dbdq-linux-x86_64.tar.gz", "dbdq-linux-aarch64.tar.gz", "SHA256SUMS.txt")
            } | Select-Object -ExpandProperty FullName

            if ($uploadAssets) {
                Write-Host "Uploading archives to release $tag..." -ForegroundColor Cyan
                gh release upload $tag $uploadAssets --repo $Repo --clobber
                Write-Host "Upload complete for $tag." -ForegroundColor Green
            }
        }
    } finally {
        Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

# 3. Prune Obsolete Release Assets
$obsoleteAssetNames = @(
    "dbdqueue-linux-aarch64", "dbdqueue-linux-x86_64", "dbdqueue-windows-x64.exe",
    "install.ps1", "install.sh"
)

if ($PruneAssets) {
    Write-Host "`n----------------------------------------------------------------------" -ForegroundColor DarkGray
    Write-Host "==> Pruning Obsolete Release Assets..." -ForegroundColor Cyan
    Write-Host "----------------------------------------------------------------------" -ForegroundColor DarkGray

    $allCheckTags = @($legacyTags) + @($v06Tags)
    foreach ($tag in $allCheckTags) {
        if ($DryRun) {
            Write-Host "[DRY-RUN] Would inspect and prune obsolete assets for release: $tag" -ForegroundColor Yellow
        } else {
            $check = gh release view $tag --repo $Repo 2>&1
            if ($LASTEXITCODE -eq 0) {
                Write-Host "Inspecting release $tag for obsolete assets..." -ForegroundColor Gray
                foreach ($asset in $obsoleteAssetNames) {
                    gh release delete-asset $tag $asset --repo $Repo --yes 2>$null
                }
            }
        }
    }
}

Write-Host "`n==> All operations completed successfully!" -ForegroundColor Green
