# Package committed build inputs; never copy a running app or SDK/cache directory.
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not (Test-Path -LiteralPath (Join-Path $root '.git'))) { throw 'Create source archives from a Git checkout.' }
& git -C $root diff --quiet HEAD
if ($LASTEXITCODE -ne 0) { throw 'Commit tracked changes before creating a source archive.' }
$versionLine = Get-Content -LiteralPath (Join-Path $root 'apps/mirrormesh_flutter/pubspec.yaml') | Where-Object { $_ -match '^version:' }
if ($versionLine -notmatch '^version:\s*(\d+\.\d+\.\d+)\+') { throw 'Could not read product version.' }
$version = $Matches[1]
$output = Join-Path $root 'releases'
New-Item -ItemType Directory -Force -Path $output | Out-Null
$archive = Join-Path $output "MirrorMesh-$version-source.zip"
$staging = Join-Path $root '.toolchains'
New-Item -ItemType Directory -Force -Path $staging | Out-Null
$candidate = Join-Path $staging ("source-package-{0}.zip" -f [guid]::NewGuid().ToString('N'))
& git -C $root archive --format=zip --prefix=MirrorMesh/ "--output=$candidate" HEAD
if ($LASTEXITCODE -ne 0) { throw 'Source archive failed.' }
Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [IO.Compression.ZipFile]::OpenRead($candidate)
try {
    foreach ($entry in $zip.Entries) {
        if ($entry.FullName -match '(^|/)(\.env($|\.)|releases/|\.git/|\.toolchains/|target/)|\.(pem|key|p12|jks|keystore|mobileprovision|sqlite[^/]*)$') {
            throw "Private/generated path in source archive: $($entry.FullName)"
        }
        if ($entry.Length -eq 0 -or $entry.Length -gt 3MB) { continue }
        $reader = [IO.StreamReader]::new($entry.Open())
        try { $text = $reader.ReadToEnd() } finally { $reader.Dispose() }
        if ($text -match '-----BEGIN (?:[A-Z]+ )?PRIVATE KEY-----|\b(?:ghp_[A-Za-z0-9]{25,}|github_pat_[A-Za-z0-9_]{30,}|AKIA[A-Z0-9]{16})\b') {
            throw "Possible credential material in source archive: $($entry.FullName)"
        }
    }
} finally { $zip.Dispose() }
# Publish only after validation. A rejected candidate stays in the ignored staging
# directory for inspection; it is never placed among shareable release artifacts.
Move-Item -LiteralPath $candidate -Destination $archive -Force
$hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
"$hash  $(Split-Path -Leaf $archive)" | Set-Content -LiteralPath "$archive.sha256" -Encoding ascii
Write-Output "Built committed source archive: $archive"
Write-Output 'Untracked files, ignored SDKs, private state and generated releases are not included.'
