# Workspace-local build prerequisites. Never changes Defender or firewall settings.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if ((git -C $root branch --show-current) -ne 'test_5') { throw 'Use test_5 for this setup.' }
$tools = Join-Path $root '.toolchains'
$msys = Join-Path $tools 'msys64'
$downloads = Join-Path $tools 'downloads'
New-Item -ItemType Directory -Path $downloads -Force | Out-Null
if (-not (Test-Path -LiteralPath (Join-Path $msys 'usr/bin/bash.exe'))) {
    $archive = Join-Path $downloads 'msys2-base-x86_64-20260927.tar.xz'
    if (-not (Test-Path -LiteralPath $archive)) {
        & curl.exe --fail --location --silent --show-error --retry 4 --output $archive 'https://github.com/msys2/msys2-installer/releases/download/2026-09-27/msys2-base-x86_64-20260927.tar.xz'
        if ($LASTEXITCODE -ne 0) { throw 'MSYS2 bootstrap download failed.' }
    }
    if ((Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant() -ne 'ea2f31a0b6ade63914ce441ffb022f0f6aa96982bfefa2326460a26d5fb01322') { throw 'MSYS2 bootstrap checksum mismatch.' }
    & tar.exe -xf $archive -C $tools
    if ($LASTEXITCODE -ne 0) { throw 'MSYS2 extraction failed.' }
}
$env:MSYSTEM = 'UCRT64'
$env:CHERE_INVOKING = '1'
$bash = Join-Path $msys 'usr/bin/bash.exe'
try { $ErrorActionPreference = 'Continue'; & $bash -lc 'true' }
finally { $ErrorActionPreference = 'Stop' }
if ($LASTEXITCODE -ne 0) { throw 'MSYS2 initialization failed.' }
$pacman = Join-Path $msys 'usr/bin/pacman.exe'
$actual = @{}
& $pacman -Q | ForEach-Object { $parts=$_ -split ' ',2; $actual[$parts[0]]=$parts[1] }
if ($LASTEXITCODE -ne 0) { throw 'Installed-package inspection failed.' }
$lock = Get-Content -LiteralPath (Join-Path $PSScriptRoot 'airplay/toolchain-packages.lock.json') -Raw | ConvertFrom-Json
$needed = @($lock | Where-Object { $actual[$_.name] -ne $_.version })
$cache = Join-Path $msys 'var/cache/pacman/pkg'
foreach ($package in $needed) {
    $destination = Join-Path $cache $package.file
    if (-not (Test-Path -LiteralPath $destination)) {
        & curl.exe --fail --location --silent --show-error --retry 5 --retry-delay 3 --output $destination $package.url
        if ($LASTEXITCODE -ne 0) { throw "Download failed: $($package.name)" }
    }
    if ((Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant() -ne $package.sha256) { throw "Checksum mismatch: $($package.name)" }
    if (-not (Test-Path -LiteralPath "$destination.sig")) {
        & curl.exe --fail --location --silent --show-error --retry 5 --retry-delay 3 --output "$destination.sig" "$($package.url).sig"
        if ($LASTEXITCODE -ne 0) { throw "Signature download failed: $($package.name)" }
    }
}
foreach ($kind in @('core','ucrt')) {
    $group = @($needed | Where-Object { ($_.name -like 'mingw-*') -eq ($kind -eq 'ucrt') })
    if ($group.Count -eq 0) { continue }
    $files = @($group | ForEach-Object { Join-Path $cache $_.file })
    # Pacman still requires trusted signatures, in addition to our exact hash lock.
    try { $ErrorActionPreference = 'Continue'; & $pacman -U --noconfirm @files }
    finally { $ErrorActionPreference = 'Stop' }
    if ($LASTEXITCODE -ne 0) { throw "Locked $kind package installation failed. Restart this script after a core runtime upgrade." }
}
Write-Output 'Workspace-local AirPlay build toolchain matches the package lock.'
