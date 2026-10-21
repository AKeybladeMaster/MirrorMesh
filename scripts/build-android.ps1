param(
    [ValidateSet('debug', 'release')][string]$Configuration = 'debug',
    [ValidateSet('arm64-v8a', 'x86_64')][string]$Abi = 'arm64-v8a',
    [string]$AndroidSdk = $env:ANDROID_SDK_ROOT
)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if ((git -C $root branch --show-current) -ne 'test_2') { throw 'Build this release from the test_2 branch.' }
if (-not $AndroidSdk) { $AndroidSdk = Join-Path $root '.toolchains/android-sdk' }
$env:ANDROID_SDK_ROOT = (Resolve-Path -LiteralPath $AndroidSdk).Path
if (-not $env:JAVA_HOME) { throw 'Set JAVA_HOME to a JDK 17 or newer installation.' }
$rustTarget = if ($Abi -eq 'arm64-v8a') { 'aarch64-linux-android' } else { 'x86_64-linux-android' }
try {
    # Windows PowerShell treats redirected native stderr as ErrorRecord objects.
    # Rust/Gradle emit informational messages there; their exit code is authoritative.
    $ErrorActionPreference = 'Continue'
    & rustup target add $rustTarget
} finally { $ErrorActionPreference = 'Stop' }
if ($LASTEXITCODE -ne 0) { throw 'Rust Android target installation failed.' }
$variant = (Get-Culture).TextInfo.ToTitleCase($Configuration)
Push-Location (Join-Path $root 'apps/android-tv')
try {
    $ErrorActionPreference = 'Continue'
    & ./gradlew.bat ":app:assemblePhone$variant" ":app:assembleTv$variant" "-PmmAndroidAbi=$Abi" --console=plain --no-daemon
    if ($LASTEXITCODE -ne 0) { throw 'Android APK build failed.' }
} finally { $ErrorActionPreference = 'Stop'; Pop-Location }

foreach ($flavor in @('phone', 'tv')) {
    $source = Join-Path $root "apps/android-tv/app/build/outputs/apk/$flavor/$Configuration"
    $metadata = Get-Content -LiteralPath (Join-Path $source 'output-metadata.json') -Raw | ConvertFrom-Json
    $name = $metadata.elements[0].outputFile
    $output = Join-Path $root "releases/android-$flavor"
    New-Item -ItemType Directory -Force -Path $output | Out-Null
    $suffix = if ($name -like '*unsigned*') { '-unsigned' } else { '' }
    $destination = Join-Path $output "MirrorMesh-$flavor-$Abi-$Configuration$suffix.apk"
    Copy-Item -LiteralPath (Join-Path $source $name) -Destination $destination -Force
    $hash = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $(Split-Path -Leaf $destination)" | Set-Content -LiteralPath "$destination.sha256" -Encoding ascii
    Write-Output "Built: $destination"
}
if ($Configuration -eq 'debug') {
    Write-Output 'Debug APKs are signed locally and installable for preview. They are not store releases.'
} elseif (-not $env:MM_ANDROID_KEYSTORE) {
    Write-Output 'Release APKs are unsigned. Supply your signing environment variables to produce distributable APKs.'
}
