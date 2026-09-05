# Set strict error handling
$ErrorActionPreference = "Stop"

Write-Host "--- Checking Prerequisites ---" -ForegroundColor Cyan

# Function to check if a command exists
function Test-CommandExists {
    param ([string]$Command)
    return [bool](Get-Command $Command -ErrorAction SilentlyContinue)
}

# 1. Check and Install Git
if (Test-CommandExists "git") {
    Write-Host "[✓] Git is already installed." -ForegroundColor Green
} else {
    Write-Host "[!] Git is not installed. Installing via winget..." -ForegroundColor Yellow
    winget install --id Git.Git -e --source winget
}

# 2. Check and Install Rust / Cargo
if (Test-CommandExists "cargo") {
    Write-Host "[✓] Rust (Cargo) is already installed." -ForegroundColor Green
} else {
    Write-Host "[!] Rust is not installed. Installing via winget..." -ForegroundColor Yellow
    winget install --id Rustlang.Rustup -e --source winget

    # Run rustup default setup if needed
    Write-Host "[*] Initializing rustup toolchain..." -ForegroundColor Yellow
    & "$env:USERPROFILE\.cargo\bin\rustup.exe" default stable
}

# 3. Refresh PATH Environment Variables for current session
Write-Host "`n--- Refreshing Environment Variables ---" -ForegroundColor Cyan
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

# Ensure Cargo bin path is explicitly added to the current session path
$cargoBin = "$env:USERPROFILE\.cargo\bin"
if ((Test-Path $cargoBin) -and ($env:Path -notlike "*$cargoBin*")) {
    $env:Path += ";$cargoBin"
}

# Verify installation before building
if (-not (Test-CommandExists "cargo") -or -not (Test-CommandExists "git")) {
    Write-Host "`n[X] Error: Prerequisites could not be detected in PATH. Please restart your terminal and rerun the script." -ForegroundColor Red
    exit 1
}

# 4. Install rtop via Cargo
Write-Host "`n--- Installing rtop from GitHub ---" -ForegroundColor Cyan
cargo install --git https://github.com/Mohammed-Bahr/rtop.git

Write-Host "`n[✓] Setup completed successfully! You can now run 'rtop' from your terminal." -ForegroundColor Green
