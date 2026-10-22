param([switch]$SkipToolchainBuild)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if ((git -C $root branch --show-current) -ne 'test_6') { throw 'Build only from test_6.' }
$tools = Join-Path $root '.toolchains'
New-Item -ItemType Directory -Force -Path $tools | Out-Null
if (-not $SkipToolchainBuild) {
    try {
        $ErrorActionPreference = 'Continue'
        & docker build --progress=plain -f (Join-Path $PSScriptRoot 'linux.Dockerfile') -t mirrormesh-linux-toolchain:3.47.7 $PSScriptRoot
        $buildExit = $LASTEXITCODE
    } finally { $ErrorActionPreference = 'Stop' }
    if ($buildExit -ne 0) { throw "Linux toolchain build failed (exit $buildExit)." }
}
# Explicit source roots exclude user releases, keys, .git, Windows SDK/cache and build output.
$files = & git -C $root -c core.quotepath=false ls-files --cached --others --exclude-standard -- .gitattributes AGENTS.md README.md Cargo.toml Cargo.lock rust-toolchain.toml apps crates scripts docs
if ($LASTEXITCODE -ne 0) { throw 'Source inventory failed.' }
$files = $files | Where-Object { Test-Path -LiteralPath (Join-Path $root $_) -PathType Leaf }
$list = Join-Path $tools 'linux-source-files.txt'
[IO.File]::WriteAllLines($list, $files, [Text.UTF8Encoding]::new($false))
$snapshot = Join-Path $tools 'linux-source.tar'
& tar.exe -cf $snapshot -C $root -T $list
if ($LASTEXITCODE -ne 0) { throw 'Source snapshot failed.' }
$output = Join-Path $root 'releases'
New-Item -ItemType Directory -Force -Path $output | Out-Null
# Linux compiles in its own filesystem, never over Windows Flutter/Cargo outputs.
try {
    $ErrorActionPreference = 'Continue'
    & docker run --rm --mount "type=bind,source=$snapshot,target=/input/source.tar,readonly" --mount "type=bind,source=$output,target=/output" --mount 'type=volume,source=mirrormesh-linux-target,target=/work/MirrorMesh/target' --mount 'type=volume,source=mirrormesh-linux-registry,target=/opt/cargo/registry' --mount 'type=volume,source=mirrormesh-linux-pub-cache,target=/opt/pub-cache' -e MM_SOURCE_SNAPSHOT_BRANCH=test_6 -e MM_RELEASE_OUTPUT=/output mirrormesh-linux-toolchain:3.47.7 sh -c 'mkdir -p /work/MirrorMesh && tar -xf /input/source.tar -C /work/MirrorMesh && sh /work/MirrorMesh/scripts/build-linux.sh'
    $buildExit = $LASTEXITCODE
} finally { $ErrorActionPreference = 'Stop' }
if ($buildExit -ne 0) { throw "Linux release build failed (exit $buildExit)." }
