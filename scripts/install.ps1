# ORDYN Toolchain Installer for Windows (PowerShell)
$ErrorActionPreference = "Stop"

Write-Host "=== Installing ORDYN Toolchain (v0.1.0 Sovereign) ===" -ForegroundColor Cyan

if (Get-Command cargo -ErrorAction SilentlyContinue) {
    Write-Host "[1/2] Rust/Cargo detected. Installing ordyn-cli and ordyn-lsp via cargo..." -ForegroundColor Green
    cargo install --git https://github.com/M1k3lee/ORDYN.git ordyn-cli ordyn-lsp --locked
    Write-Host "[2/2] ORDYN binaries installed successfully!" -ForegroundColor Green
} else {
    Write-Host "[*] Cargo not found in current PATH. Checking for .cargo/bin..." -ForegroundColor Yellow
    $cargoBin = "$env:USERPROFILE\.cargo\bin\cargo.exe"
    if (Test-Path $cargoBin) {
        & $cargoBin install --git https://github.com/M1k3lee/ORDYN.git ordyn-cli ordyn-lsp --locked
        Write-Host "[OK] Installed ordyn-cli via user cargo toolchain!" -ForegroundColor Green
    } else {
        Write-Host "[!] Rust toolchain not detected. Please install Rust from https://rustup.rs before installing ORDYN." -ForegroundColor Red
        exit 1
    }
}

Write-Host ""
Write-Host "=== Installation Complete! ===" -ForegroundColor Cyan
Write-Host "Run 'ordyn --help' or 'ordyn new my-app' to initialize your first project." -ForegroundColor Green
