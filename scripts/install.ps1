# ORDYN Toolchain Installer for Windows (PowerShell)
# Usage:
#   irm https://raw.githubusercontent.com/M1k3lee/ORDYN/main/scripts/install.ps1 | iex
#   & ([scriptblock]::Create((irm https://raw.githubusercontent.com/M1k3lee/ORDYN/main/scripts/install.ps1))) -FromSource

param (
    [switch]$FromSource
)

$ErrorActionPreference = "Stop"
$Repo = "M1k3lee/ORDYN"
$InstallDir = "$env:LOCALAPPDATA\Programs\ordyn\bin"

Write-Host ""
Write-Host "  ██████╗ ██████╗ ██████╗ ██╗   ██╗███╗   ██╗" -ForegroundColor Cyan
Write-Host " ██╔═══██╗██╔══██╗██╔══██╗╚██╗ ██╔╝████╗  ██║" -ForegroundColor Cyan
Write-Host " ██║   ██║██████╔╝██║  ██║ ╚████╔╝ ██╔██╗ ██║" -ForegroundColor Cyan
Write-Host " ██║   ██║██╔══██╗██║  ██║  ╚██╔╝  ██║╚██╗██║" -ForegroundColor Cyan
Write-Host " ╚██████╔╝██║  ██║██████╔╝   ██║   ██║ ╚████║" -ForegroundColor Cyan
Write-Host "  ╚═════╝ ╚═╝  ╚═╝╚═════╝    ╚═╝   ╚═╝  ╚═══╝" -ForegroundColor Cyan
Write-Host ""
Write-Host "=== Installing ORDYN Toolchain for Windows ===" -ForegroundColor Cyan

function Install-FromSource {
    Write-Host "[*] Building ORDYN from source using Cargo..." -ForegroundColor Yellow
    $cargoCmd = Get-Command cargo -ErrorAction SilentlyContinue
    if (-not $cargoCmd) {
        $userCargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
        if (Test-Path $userCargo) {
            $cargoCmd = $userCargo
        } else {
            Write-Error "Cargo/Rust toolchain not found. Please install Rust from https://rustup.rs first."
            exit 1
        }
    }
    & $cargoCmd install --git "https://github.com/$Repo.git" ordyn-cli --locked --force
    Write-Host "[OK] Installed ordyn.exe via Cargo!" -ForegroundColor Green
}

if ($FromSource) {
    Install-FromSource
} else {
    $Target = "x86_64-pc-windows-msvc"
    $ReleaseUrl = "https://github.com/$Repo/releases/latest/download/ordyn-$Target.zip"
    $TempZip = "$env:TEMP\ordyn-$Target.zip"
    $TempExtract = "$env:TEMP\ordyn-extract"

    Write-Host "[1/3] Checking for pre-built Windows x64 binary..." -ForegroundColor Gray
    $downloadSuccess = $false

    try {
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -Uri $ReleaseUrl -OutFile $TempZip -UseBasicParsing -ErrorAction Stop
        $downloadSuccess = $true
    } catch {
        Write-Host "[!] Pre-built binary not found on latest release. Falling back to Cargo install..." -ForegroundColor Yellow
    }

    if ($downloadSuccess) {
        Write-Host "[2/3] Extracting release archive..." -ForegroundColor Gray
        if (Test-Path $TempExtract) { Remove-Item -Recurse -Force $TempExtract }
        Expand-Archive -Path $TempZip -DestinationPath $TempExtract -Force

        if (-not (Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        }

        Copy-Item "$TempExtract\ordyn.exe" "$InstallDir\ordyn.exe" -Force
        Remove-Item -Force $TempZip -ErrorAction SilentlyContinue
        Remove-Item -Recurse -Force $TempExtract -ErrorAction SilentlyContinue

        Write-Host "[3/3] Successfully installed ordyn.exe to $InstallDir\ordyn.exe" -ForegroundColor Green

        # Add to User PATH if not already present
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if ($userPath -notlike "*$InstallDir*") {
            $newPath = "$userPath;$InstallDir"
            [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
            $env:Path = "$env:Path;$InstallDir"
            Write-Host "[NOTE] Added $InstallDir to your User PATH." -ForegroundColor Cyan
        }
    } else {
        Install-FromSource
    }
}

Write-Host ""
Write-Host "=== Installation Complete! ===" -ForegroundColor Cyan
Write-Host "Verify with:" -ForegroundColor White
Write-Host "  ordyn --version" -ForegroundColor Gray
Write-Host "  ordyn new my_app --template web" -ForegroundColor Gray
Write-Host ""
