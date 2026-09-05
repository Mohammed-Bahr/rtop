$ErrorActionPreference = 'Stop'

$repo = 'Mohammed-Bahr/rtop'
$app = 'rtop.exe'
$installDir = if ($env:RTOP_INSTALL_DIR) {
    $env:RTOP_INSTALL_DIR
} else {
    Join-Path $HOME '.local\bin'
}
$asset = "rtop-windows-x86_64.zip"
$baseUrl = "https://github.com/$repo/releases/latest/download"
$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("rtop-install-" + [guid]::NewGuid())

try {
    New-Item -ItemType Directory -Path $tempDir | Out-Null
    $archive = Join-Path $tempDir $asset
    $checksums = Join-Path $tempDir 'checksums.txt'

    Write-Host "Downloading $asset latest release..."
    Invoke-WebRequest -Uri "$baseUrl/$asset" -OutFile $archive
    Invoke-WebRequest -Uri "$baseUrl/checksums.txt" -OutFile $checksums

    $escapedAsset = [regex]::Escape($asset)
    $checksumLine = Get-Content $checksums |
        Where-Object { $_ -match "\s+\*?$escapedAsset\s*$" } |
        Select-Object -First 1
    if (-not $checksumLine) {
        throw "checksums.txt has no entry for $asset"
    }
    if ($checksumLine -notmatch "^\s*([0-9a-fA-F]{64})\s+\*?$escapedAsset\s*$") {
        throw "invalid checksum entry for $asset"
    }

    $expected = $Matches[1].ToLowerInvariant()
    $actual = (Get-FileHash -Path $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($expected -ne $actual) {
        throw 'checksum verification failed; refusing to install'
    }

    $extractDir = Join-Path $tempDir 'extracted'
    Expand-Archive -Path $archive -DestinationPath $extractDir
    $binary = Get-ChildItem -Path $extractDir -Filter $app -File -Recurse |
        Select-Object -First 1
    if (-not $binary) {
        throw "release archive does not contain expected $app"
    }

    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    Copy-Item -Path $binary.FullName -Destination (Join-Path $installDir $app) -Force
    Write-Host "Installed rtop to $installDir"

    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    $pathEntries = @($userPath -split ';' | Where-Object { $_ })
    if ($pathEntries -notcontains $installDir) {
        [Environment]::SetEnvironmentVariable('Path', (($pathEntries + $installDir) -join ';'), 'User')
        Write-Host "Added $installDir to your user PATH. Open a new PowerShell window to use rtop."
    } else {
        Write-Host 'Run: rtop --version'
    }
} finally {
    if (Test-Path $tempDir) {
        Remove-Item -Path $tempDir -Recurse -Force
    }
}
