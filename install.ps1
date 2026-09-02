# dbdqueue Windows One-Liner Installer & Launcher
$ErrorActionPreference = 'Stop'

[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.ServicePointManager]::SecurityProtocol -bor [System.Net.SecurityProtocolType]::Tls12

$Repo = "trazxdxne/dbdqueue"
Write-Host "==> Installing dbdqueue..." -ForegroundColor Cyan

# 1. Installation directory in local AppData
$installDir = Join-Path $env:LOCALAPPDATA "Programs\dbdqueue"
if (-not (Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

$exePath = Join-Path $installDir "dbdq.exe"
$aliasPath = Join-Path $installDir "dbdqueue.exe"
$downloadUrl = "https://github.com/$Repo/releases/latest/download/dbdqueue-windows-x64.exe"
$tempFile = Join-Path ([System.IO.Path]::GetTempPath()) "dbdqueue_download_$([System.Guid]::NewGuid().ToString('N')).exe"

# 2. Download binary
Write-Host "==> Downloading latest binary..." -ForegroundColor Cyan
try {
    Invoke-WebRequest -Uri $downloadUrl -OutFile $tempFile -UseBasicParsing
} catch {
    Write-Error ("Failed to download binary from " + $downloadUrl + ": " + $_)
    exit 1
}

# 3. Copy binary into place
try {
    Copy-Item -Path $tempFile -Destination $exePath -Force
    Copy-Item -Path $tempFile -Destination $aliasPath -Force
} catch {
    Write-Warning "Failed to copy binary. If dbdq is currently running, please close it and run this script again."
    Remove-Item -Path $tempFile -Force -ErrorAction SilentlyContinue
    exit 1
} finally {
    Remove-Item -Path $tempFile -Force -ErrorAction SilentlyContinue
}

# 4. Add to User PATH if not present
try {
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -notlike "*$installDir*") {
        Write-Host "==> Adding $installDir to User PATH..." -ForegroundColor Cyan
        $newUserPath = if ([string]::IsNullOrWhiteSpace($userPath)) { $installDir } else { "${userPath};${installDir}" }
        [Environment]::SetEnvironmentVariable("Path", $newUserPath, "User")
    }
} catch {
    Write-Warning "Could not update User PATH automatically. You may need to add $installDir manually."
}

if ($env:Path -notlike "*$installDir*") {
    $env:Path = "${installDir};$env:Path"
}

Write-Host "==> dbdqueue installed successfully!" -ForegroundColor Green
Write-Host "You can now run it anytime from any terminal: dbdq or dbdqueue`n" -ForegroundColor Yellow

# 5. Launch dbdq
if (Test-Path $exePath) {
    & "$exePath"
}
