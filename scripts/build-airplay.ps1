param([string]$MsysRoot = '', [switch]$SkipCompile)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if ((git -C $root branch --show-current) -notin @('test_3','test_4')) { throw 'Build only from test_3 or test_4.' }
if (-not $MsysRoot) { $MsysRoot = Join-Path $root '.toolchains/msys64' }
$MsysRoot = (Resolve-Path -LiteralPath $MsysRoot).Path
$bash = Join-Path $MsysRoot 'usr/bin/bash.exe'
$prefix = Join-Path $MsysRoot 'ucrt64'
$packageLock = Join-Path $PSScriptRoot 'airplay/toolchain-packages.lock.json'
$installed = @{}
& (Join-Path $MsysRoot 'usr/bin/pacman.exe') -Q | ForEach-Object { $parts=$_ -split ' ',2; $installed[$parts[0]]=$parts[1] }
if ($LASTEXITCODE -ne 0) { throw 'Package inspection failed.' }
$lock = Get-Content -LiteralPath $packageLock -Raw | ConvertFrom-Json
foreach ($package in $lock) {
    if ($installed[$package.name] -ne $package.version) { throw "AirPlay package lock mismatch for $($package.name). Run scripts/setup-airplay-toolchain.ps1." }
}
$source = Join-Path $root '.toolchains/uxplay'
$revision = '15a6b87172eca38b0df4e615fbf1bba108734dde'
if (-not (Test-Path -LiteralPath $source)) {
    try {
        $ErrorActionPreference = 'Continue'
        & git clone --no-checkout https://github.com/FDH2/UxPlay.git $source
        if ($LASTEXITCODE -ne 0) { throw 'UxPlay clone failed.' }
        & git -C $source checkout --detach $revision
        if ($LASTEXITCODE -ne 0) { throw 'UxPlay checkout failed.' }
    } finally { $ErrorActionPreference = 'Stop' }
}
if ((git -C $source rev-parse HEAD) -ne $revision) { throw 'Unexpected UxPlay revision; preserve that checkout and provide the pinned source.' }
$patch = Join-Path $PSScriptRoot 'airplay/uxplay-mirrormesh.patch'
$previousPreference = $ErrorActionPreference
try {
    $ErrorActionPreference = 'Continue'
    & git -C $source apply --reverse --check $patch 2>$null
    $reverseExit = $LASTEXITCODE
} finally { $ErrorActionPreference = $previousPreference }
if ($reverseExit -ne 0) {
    & git -C $source apply --check $patch
    if ($LASTEXITCODE -ne 0) { throw 'UxPlay source differs from the reviewed patch; no changes discarded.' }
    & git -C $source apply $patch
    if ($LASTEXITCODE -ne 0) { throw 'UxPlay patch failed.' }
}
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'airplay/mirrormesh_bridge.cpp'),(Join-Path $PSScriptRoot 'airplay/mirrormesh_bridge.h') -Destination $source
$env:MSYSTEM = 'UCRT64'
$env:CHERE_INVOKING = '1'
Push-Location $root
try {
    if (-not $SkipCompile) {
        try {
            $ErrorActionPreference = 'Continue'
            & $bash -lc 'cmake -S .toolchains/uxplay -B .toolchains/uxplay/build -G Ninja -DCMAKE_BUILD_TYPE=Release -DNO_MARCH_NATIVE=ON -DCMAKE_C_FLAGS_RELEASE=-O2 -DCMAKE_CXX_FLAGS_RELEASE=-O2 && cmake --build .toolchains/uxplay/build -j 4'
        } finally { $ErrorActionPreference = 'Stop' }
        if ($LASTEXITCODE -ne 0) { throw 'AirPlay helper compilation failed.' }
    }
} finally { Pop-Location }

$output = [IO.Path]::GetFullPath((Join-Path $root '.toolchains/airplay-runtime'))
$expected = [IO.Path]::GetFullPath((Join-Path $root '.toolchains/airplay-runtime'))
if ($output -ne $expected -or -not $output.StartsWith([IO.Path]::GetFullPath($root) + [IO.Path]::DirectorySeparatorChar)) { throw 'Invalid runtime output path.' }
if (Test-Path -LiteralPath $output) { Remove-Item -LiteralPath $output -Recurse -Force }
foreach ($folder in @('bin','lib/gstreamer-1.0','libexec/gstreamer-1.0','licenses','source')) {
    New-Item -ItemType Directory -Path (Join-Path $output $folder) -Force | Out-Null
}
$queue = [Collections.Generic.Queue[string]]::new()
$queue.Enqueue((Join-Path $source 'build/uxplay.exe'))
$queue.Enqueue((Join-Path $prefix 'libexec/gstreamer-1.0/gst-plugin-scanner.exe'))
# Mirror-mode H.264 with hardware decoding and software fallback; AAC/ALAC + WASAPI.
# Exclude unrelated device capture, network services, AI models and other plugin families.
# The pinned helper checks for autodetect during gstreamer_init even with fixed sinks.
$plugins = @('app','coreelements','playback','autodetect','typefindfunctions','videoparsersbad',
    'videoconvertscale','videofilter','d3d11','libav','audioparsers','audioconvert',
    'audioresample','volume','level','wasapi')
foreach ($plugin in $plugins) { $queue.Enqueue((Join-Path $prefix "lib/gstreamer-1.0/libgst$plugin.dll")) }
$copied = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
$owners = [Collections.Generic.HashSet[string]]::new()
$packageFiles = [Collections.Generic.List[string]]::new()
$objdump = Join-Path $prefix 'bin/objdump.exe'
$pacman = Join-Path $MsysRoot 'usr/bin/pacman.exe'
while ($queue.Count -gt 0) {
    $file = [IO.Path]::GetFullPath($queue.Dequeue())
    if (-not (Test-Path -LiteralPath $file)) { throw "Missing runtime input: $file" }
    $name = Split-Path -Leaf $file
    if (-not $copied.Add($name)) { continue }
    if ($copied.Count % 25 -eq 0) { Write-Output "Collected $($copied.Count) native runtime files..." }
    # Match the source DIRECTORY, never the DLL name: libgstreamer-1.0-0.dll is
    # a loader dependency in bin, not a GStreamer plugin.
    $pluginDirectory = [IO.Path]::GetFullPath((Join-Path $prefix 'lib/gstreamer-1.0'))
    $relative = if ([IO.Path]::GetDirectoryName($file).Equals($pluginDirectory, [StringComparison]::OrdinalIgnoreCase) -and $name -like '*.dll') { "lib/gstreamer-1.0/$name" }
        elseif ($name -eq 'gst-plugin-scanner.exe') { "libexec/gstreamer-1.0/$name" } else { "bin/$name" }
    Copy-Item -LiteralPath $file -Destination (Join-Path $output $relative)
    if ($file.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) {
        $posix = '/ucrt64/' + $file.Substring($prefix.Length + 1).Replace('\','/')
        $packageFiles.Add($posix)
    }
    $imports = & $objdump -p $file
    if ($LASTEXITCODE -ne 0) { throw "Cannot inspect native imports for $name" }
    foreach ($line in $imports) {
        if ($line -match 'DLL Name:\s*(\S+)') {
            $dependency = $Matches[1]
            $local = Join-Path $prefix "bin/$dependency"
            if (Test-Path -LiteralPath $local) { $queue.Enqueue($local) }
            elseif ($dependency -notmatch '^(api|ext)-ms-' -and -not (Test-Path -LiteralPath (Join-Path $env:SystemRoot "System32/$dependency"))) {
                throw "Unresolved native dependency: $dependency (from $name)"
            }
        }
    }
}
# Inspect the packaged layout, not just the toolchain's available dependencies.
# Every ordinary imported DLL must be beside the helper/on its bin search path,
# or supplied by Windows. A copy in a plugin folder cannot satisfy that import.
foreach ($binary in (Get-ChildItem -LiteralPath $output -Recurse -File | Where-Object Extension -In '.exe','.dll')) {
    $imports = & $objdump -p $binary.FullName
    if ($LASTEXITCODE -ne 0) { throw "Cannot inspect packaged imports for $($binary.Name)" }
    foreach ($line in $imports) {
        if ($line -match 'DLL Name:\s*(\S+)') {
            $dependency = $Matches[1]
            if ($dependency -match '^(api|ext)-ms-') { continue }
            if (Test-Path -LiteralPath (Join-Path $output "bin/$dependency")) { continue }
            if (Test-Path -LiteralPath (Join-Path $env:SystemRoot "System32/$dependency")) { continue }
            throw "Packaged dependency is missing from bin: $dependency (from $($binary.Name))"
        }
    }
}
# A help invocation exercises the actual Windows loader without opening a receiver.
# Restrict PATH to the bundle and Windows so an SDK cannot hide a packaging defect.
$launch = New-Object System.Diagnostics.ProcessStartInfo
$launch.FileName = Join-Path $output 'bin/uxplay.exe'
$launch.Arguments = '-h'
$launch.WorkingDirectory = $output
$launch.UseShellExecute = $false
$launch.CreateNoWindow = $true
$launch.RedirectStandardOutput = $true
$launch.RedirectStandardError = $true
$launch.EnvironmentVariables['PATH'] = (Join-Path $output 'bin') + ";$env:SystemRoot/System32;$env:SystemRoot"
$loader = New-Object System.Diagnostics.Process
$loader.StartInfo = $launch
try {
    if (-not $loader.Start()) { throw 'Could not start the packaged AirPlay helper.' }
    $stdout = $loader.StandardOutput.ReadToEndAsync()
    $stderr = $loader.StandardError.ReadToEndAsync()
    if (-not $loader.WaitForExit(10000)) {
        $loader.Kill()
        $loader.WaitForExit()
        throw 'Packaged AirPlay helper did not finish its help invocation.'
    }
    if ($loader.ExitCode -ne 0 -or $stdout.Result -notmatch 'UxPlay .*AirPlay mirroring server') {
        throw ('Packaged AirPlay loader failed (exit {0}); run scripts/diagnose-airplay.ps1.' -f $loader.ExitCode)
    }
    [void]$stderr.Result
    Write-Output 'Packaged AirPlay loader: exit 0 (no receiver started).'
} finally { $loader.Dispose() }
& $pacman -Qqo @packageFiles | ForEach-Object { [void]$owners.Add($_.Trim()) }
if ($LASTEXITCODE -ne 0) { throw 'Package provenance lookup failed.' }
$versions = @{}
& $pacman -Q @owners | ForEach-Object { $parts = $_ -split ' ',2; $versions[$parts[0]] = $parts[1] }
if ($LASTEXITCODE -ne 0) { throw 'Package version lookup failed.' }
$inventory = foreach ($owner in ($owners | Sort-Object)) {
    $short = $owner -replace '^mingw-w64-ucrt-x86_64-', ''
    $license = Join-Path $prefix "share/licenses/$short"
    if (Test-Path -LiteralPath $license) { Copy-Item -LiteralPath $license -Destination (Join-Path $output 'licenses') -Recurse -Force }
    [pscustomobject]@{ package=$owner; version=$versions[$owner]; recipe="https://github.com/msys2/MINGW-packages/tree/1b7ab74a48dccaa86fd49a169b00fa25e0dd6027/mingw-w64-$short" }
}
$inventory | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath (Join-Path $output 'source/runtime-packages.json') -Encoding UTF8
Copy-Item -LiteralPath (Join-Path $source 'LICENSE') -Destination (Join-Path $output 'licenses/UxPlay-GPL-3.0.txt')
Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'airplay/NOTICE.md') -Destination (Join-Path $output 'NOTICE.md')
Copy-Item -LiteralPath $patch,(Join-Path $PSScriptRoot 'airplay/mirrormesh_bridge.cpp'),(Join-Path $PSScriptRoot 'airplay/mirrormesh_bridge.h'),$PSCommandPath -Destination (Join-Path $output 'source')
Copy-Item -LiteralPath $packageLock,(Join-Path $PSScriptRoot 'setup-airplay-toolchain.ps1') -Destination (Join-Path $output 'source')
$archive = Join-Path $output 'source/UxPlay-corresponding-source.zip'
& git -C $source archive --format=zip "--output=$archive" HEAD
if ($LASTEXITCODE -ne 0) { throw 'UxPlay source archive failed.' }
Add-Type -AssemblyName System.IO.Compression.FileSystem
Add-Type -AssemblyName System.IO.Compression
$zip = [IO.Compression.ZipFile]::Open($archive, [IO.Compression.ZipArchiveMode]::Update)
try {
    # The local source payload needs no upstream CI/workflow configuration.
    $ciEntries = @($zip.Entries | Where-Object { $_.FullName.StartsWith('.github/') })
    foreach ($entry in $ciEntries) { $entry.Delete() }
    foreach ($entry in @('CMakeLists.txt','uxplay.cpp','renderers/video_renderer.c','lib/raop_handlers.h','lib/pairing.c','mirrormesh_bridge.cpp','mirrormesh_bridge.h')) {
        $old = $zip.GetEntry($entry)
        if ($old) { $old.Delete() }
        [void][IO.Compression.ZipFileExtensions]::CreateEntryFromFile($zip,(Join-Path $source $entry),$entry)
    }
} finally { $zip.Dispose() }
$rows = Get-ChildItem -LiteralPath $output -File -Recurse | Sort-Object FullName | ForEach-Object {
    '{0}  {1}' -f (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant(), $_.FullName.Substring($output.Length+1).Replace('\','/')
}
$rows | Set-Content -LiteralPath (Join-Path $output 'SHA256SUMS.txt') -Encoding ascii
Write-Output "Built native AirPlay preview runtime: $output ($($copied.Count) binaries)."
Write-Output 'No receiver was started. Physical iOS verification and redistribution review remain pending.'
