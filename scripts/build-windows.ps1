param(
    [string]$FlutterRoot = $env:FLUTTER_ROOT,
    [string]$VcRuntimeDirectory = '',
    [switch]$WithoutAirPlay
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if ((git -C $root branch --show-current) -notin @('test_3','test_4')) { throw 'Build this release from test_3 or test_4.' }
if (-not $FlutterRoot) {
    $installedFlutter = Join-Path $env:LOCALAPPDATA 'Programs/MirrorMeshFlutter/flutter'
    if (Test-Path -LiteralPath $installedFlutter) { $FlutterRoot = $installedFlutter }
    else { throw 'Set FLUTTER_ROOT or pass -FlutterRoot with the Flutter SDK directory.' }
}
$FlutterRoot = (Resolve-Path -LiteralPath $FlutterRoot).Path
$flutter = Join-Path $FlutterRoot 'bin/flutter.bat'
$env:CI = 'true'
$env:FLUTTER_SUPPRESS_ANALYTICS = 'true'
# A process-local SDK exception; no global Git configuration is changed.
$env:GIT_CONFIG_COUNT = '1'
$env:GIT_CONFIG_KEY_0 = 'safe.directory'
$env:GIT_CONFIG_VALUE_0 = $FlutterRoot.Replace('\', '/')
if (-not $WithoutAirPlay) {
    & (Join-Path $PSScriptRoot 'build-airplay.ps1')
    if ($LASTEXITCODE -ne 0) { throw 'AirPlay runtime build failed.' }
}
Push-Location (Join-Path $root 'apps/mirrormesh_flutter')
try {
    & $flutter pub get --enforce-lockfile
    if ($LASTEXITCODE -ne 0) { throw 'Flutter dependency resolution failed.' }
    & $flutter build windows --release
    if ($LASTEXITCODE -ne 0) { throw 'Windows release build failed.' }
} finally { Pop-Location }

$source = Join-Path $root 'apps/mirrormesh_flutter/build/windows/x64/runner/Release'
$output = Join-Path $root 'releases/windows-x64'
New-Item -ItemType Directory -Force -Path $output | Out-Null
Copy-Item -LiteralPath (Join-Path $source 'MirrorMesh.exe') -Destination $output
Get-ChildItem -LiteralPath $source -Filter '*.dll' | Copy-Item -Destination $output
Copy-Item -LiteralPath (Join-Path $source 'data') -Destination $output -Recurse -Force
$airplayOutput = [IO.Path]::GetFullPath((Join-Path $output 'airplay'))
if ($airplayOutput -ne [IO.Path]::GetFullPath((Join-Path $root 'releases/windows-x64/airplay'))) { throw 'Invalid AirPlay output path.' }
if (Test-Path -LiteralPath $airplayOutput) { Remove-Item -LiteralPath $airplayOutput -Recurse -Force }
if (-not $WithoutAirPlay) {
    Copy-Item -LiteralPath (Join-Path $root '.toolchains/airplay-runtime') -Destination $airplayOutput -Recurse
}

if (-not $VcRuntimeDirectory) {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    if (Test-Path -LiteralPath $vswhere) {
        $vsPath = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
        $redist = Join-Path $vsPath 'VC/Redist/MSVC'
        $runtime = Get-ChildItem -LiteralPath $redist -Directory |
            Where-Object Name -Match '^\d+\.' | Sort-Object Name -Descending | Select-Object -First 1
        if ($runtime) { $VcRuntimeDirectory = Join-Path $runtime.FullName 'x64/Microsoft.VC143.CRT' }
    }
}
if ($VcRuntimeDirectory -and (Test-Path -LiteralPath $VcRuntimeDirectory)) {
    Get-ChildItem -LiteralPath $VcRuntimeDirectory -Filter '*.dll' | Copy-Item -Destination $output
} else { Write-Warning 'VC++ runtime not bundled. Install the Microsoft VC++ x64 redistributable on the destination PC.' }

foreach ($document in @('docs/USER_GUIDE.md', 'docs/connections.md', 'docs/build-release.md', 'docs/airplay.md', 'docs/THIRD-PARTY-NOTICES-rust.txt', 'docs/THIRD-PARTY-NOTICES-flutter.txt')) {
    Copy-Item -LiteralPath (Join-Path $root $document) -Destination $output
}
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'diagnose-airplay.ps1') -Destination $output
$files = Get-ChildItem -LiteralPath $output -File -Recurse | Where-Object Name -ne 'SHA256SUMS.txt'
$checksums = $files | Sort-Object FullName | ForEach-Object {
    '{0}  {1}' -f (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $_.FullName.Substring($output.Length + 1).Replace('\', '/')
}
$checksums | Set-Content -LiteralPath (Join-Path $output 'SHA256SUMS.txt') -Encoding ascii
Compress-Archive -Path (Join-Path $output '*') -DestinationPath (Join-Path $root 'releases/MirrorMesh-windows-x64.zip') -Force
Write-Output "Built portable bundle: $output\MirrorMesh.exe"
Write-Output 'Keep the executable, DLLs and data folder together. This build is unsigned.'
