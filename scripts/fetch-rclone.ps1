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

$Work = Join-Path ([System.IO.Path]::GetTempPath()) ("orbit-rclone-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $Work | Out-Null
try {
    Write-Host "Fetching rclone $Version (windows/amd64)"
    Invoke-WebRequest -Uri "$Base/$Zip" -OutFile (Join-Path $Work $Zip) -UseBasicParsing
    Invoke-WebRequest -Uri "$Base/SHA256SUMS" -OutFile (Join-Path $Work "SHA256SUMS") -UseBasicParsing

    $line = Get-Content (Join-Path $Work "SHA256SUMS") | Where-Object { $_ -match [regex]::Escape($Zip) + '$' } | Select-Object -First 1
    if (-not $line) { throw "No checksum for $Zip in SHA256SUMS." }
    $expected = ($line -split '\s+')[0].ToLowerInvariant()
    $actual = (Get-FileHash -Algorithm SHA256 -Path (Join-Path $Work $Zip)).Hash.ToLowerInvariant()
    if ($expected -ne $actual) { throw "SHA256 mismatch for $Zip." }

    Expand-Archive -Path (Join-Path $Work $Zip) -DestinationPath $Work -Force
    $exe = Get-ChildItem -Path $Work -Recurse -Filter "rclone.exe" | Select-Object -First 1
    if (-not $exe) { throw "rclone.exe was not found in $Zip." }

    New-Item -ItemType Directory -Path $OutDir -Force | Out-Null
    Copy-Item -Path $exe.FullName -Destination (Join-Path $OutDir "rclone-$Triple.exe") -Force
    Write-Host "Installed $(Join-Path $OutDir "rclone-$Triple.exe")"
}
finally {
    Remove-Item -Recurse -Force $Work -ErrorAction SilentlyContinue
}
