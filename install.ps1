# kb install script for Windows (PowerShell)
# Usage: iwr -useb https://raw.githubusercontent.com/shedrackgodstime/kb-cli/master/install.ps1 | iex

$ErrorActionPreference = "Stop"

$Repo = "shedrackgodstime/kb-cli"
$Binary = "kb"
$InstallDir = if ($env:KB_INSTALL_DIR) { $env:KB_INSTALL_DIR } else { "$env:USERPROFILE\.local\bin" }

function Write-Info($msg)  { Write-Host "✓ $msg" -ForegroundColor Green }
function Write-Warn($msg)  { Write-Host "! $msg" -ForegroundColor Yellow }
function Write-Error($msg) { Write-Host "✗ $msg" -ForegroundColor Red; exit 1 }

function Get-LatestVersion {
    try {
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -UseBasicParsing
        return $release.tag_name
    } catch {
        Write-Error "Failed to fetch latest version. Check your network connection."
    }
}

function Install-KB {
    $version = Get-LatestVersion
    $arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "x86" }
    $zipfile = "$Binary-$version-$arch-pc-windows-msvc.zip"
    $url = "https://github.com/$Repo/releases/download/$version/$zipfile"

    Write-Host "Installing kb $version (windows/$arch)..."

    # Create install dir
    if (!(Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }

    # Download to temp
    $tmpdir = Join-Path $env:TEMP "kb-install-$(Get-Random)"
    New-Item -ItemType Directory -Path $tmpdir -Force | Out-Null

    try {
        Write-Host "Downloading from $url..."
        $zippath = Join-Path $tmpdir $zipfile
        Invoke-WebRequest -Uri $url -OutFile $zippath -UseBasicParsing

        # Extract
        Write-Host "Extracting..."
        Expand-Archive -Path $zippath -DestinationPath $tmpdir -Force

        # Move binary
        $src = Join-Path $tmpdir "$Binary.exe"
        $dst = Join-Path $InstallDir "$Binary.exe"
        Copy-Item $src $dst -Force

        Write-Info "Installed $Binary to $dst"
    } finally {
        Remove-Item $tmpdir -Recurse -Force -ErrorAction SilentlyContinue
    }

    # Check if in PATH
    $pathDirs = $env:PATH -split ";"
    if ($pathDirs -contains $InstallDir) {
        Write-Info "$InstallDir is in your PATH"
    } else {
        Write-Warn "$InstallDir is not in your PATH"
        Write-Host ""
        Write-Host "  Add it to your PATH:" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "    `$env:PATH = `"$InstallDir`";`$env:PATH" -ForegroundColor Yellow
        Write-Host ""
        Write-Host "  Or add it permanently:" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "    [Environment]::SetEnvironmentVariable(`"PATH`", `"$InstallDir;`$(``[Environment]::GetEnvironmentVariable(``"PATH`"`, ``"User``"))`", `"User`")" -ForegroundColor Yellow
        Write-Host ""
        Write-Host "  Then restart your terminal." -ForegroundColor Cyan
        Write-Host ""
    }

    Write-Host ""
    Write-Info "Run 'kb --help' to get started"
}

Install-KB
