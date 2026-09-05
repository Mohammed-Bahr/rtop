$ErrorActionPreference = 'Stop'

$installDir = if ($env:RTOP_INSTALL_DIR) {
    $env:RTOP_INSTALL_DIR
} else {
    Join-Path $HOME '.local\bin'
}
$binary = Join-Path $installDir 'rtop.exe'

if (Test-Path $binary) {
    Remove-Item -Path $binary -Force
    Write-Host "Removed $binary"
} else {
    Write-Host "rtop is not installed in $installDir"
}
