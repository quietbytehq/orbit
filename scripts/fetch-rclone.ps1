# Downloads the pinned rclone build for Windows (x64) into src-tauri\binaries\, named the way
# Tauri expects sidecars: rclone-<target-triple>.exe. The archive is checked against rclone's
# published SHA256SUMS before anything is installed.
$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $PSScriptRoot
$Version = (Get-Content (Join-Path $Root "scripts\rclone-version.txt") -Raw).Trim()
$OutDir = Join-Path $Root "src-tauri\binaries"
$Triple = "x86_64-pc-windows-msvc"
$Zip = "rclone-$Version-windows-amd64.zip"
$Base = "https://downloads.rclone.org/$Version"

# The hash and the unzip go through .NET rather than Get-FileHash and Expand-Archive: those
# cmdlets live in modules that Windows PowerShell 5.1 cannot always autoload (for example when it
# is launched from npm on a GitHub runner), while the .NET types are always there.
[System.Net.ServicePointManager]::SecurityProtocol = [System.Net.SecurityProtocolType]::Tls12
Add-Type -AssemblyName System.IO.Compression.FileSystem

function Get-Sha256Hex([string]$Path) {
    $sha = [System.Security.Cryptography.SHA256]::Create()
    $stream = [System.IO.File]::OpenRead($Path)
    try { $hash = $sha.ComputeHash($stream) }
    finally { $stream.Dispose(); $sha.Dispose() }
    return ([System.BitConverter]::ToString($hash) -replace '-', '').ToLowerInvariant()
}

$Work = Join-Path ([System.IO.Path]::GetTempPath()) ("orbit-rclone-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $Work | Out-Null
try {
    Write-Host "Fetching rclone $Version (windows/amd64)"
    $web = New-Object System.Net.WebClient
    $web.DownloadFile("$Base/$Zip", (Join-Path $Work $Zip))
    $web.DownloadFile("$Base/SHA256SUMS", (Join-Path $Work "SHA256SUMS"))

    $line = Get-Content (Join-Path $Work "SHA256SUMS") | Where-Object { $_ -match [regex]::Escape($Zip) + '$' } | Select-Object -First 1
    if (-not $line) { throw "No checksum for $Zip in SHA256SUMS." }
    $expected = ($line -split '\s+')[0].ToLowerInvariant()
    $actual = Get-Sha256Hex (Join-Path $Work $Zip)
    if ($expected -ne $actual) { throw "SHA256 mismatch for $Zip." }

    $Unzipped = Join-Path $Work "unzipped"
    [System.IO.Compression.ZipFile]::ExtractToDirectory((Join-Path $Work $Zip), $Unzipped)
    $exe = Get-ChildItem -Path $Unzipped -Recurse -Filter "rclone.exe" | Select-Object -First 1
    if (-not $exe) { throw "rclone.exe was not found in $Zip." }

    New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
    Copy-Item -Path $exe.FullName -Destination (Join-Path $OutDir "rclone-$Triple.exe") -Force
    Write-Host "Installed $(Join-Path $OutDir "rclone-$Triple.exe")"
}
finally {
    Remove-Item -Recurse -Force $Work -ErrorAction SilentlyContinue
}
