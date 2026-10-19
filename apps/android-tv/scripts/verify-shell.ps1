$ErrorActionPreference = 'Stop'

$appRoot = Split-Path $PSScriptRoot -Parent
$manifestPath = Join-Path $appRoot 'app/src/main/AndroidManifest.xml'
$layoutPath = Join-Path $appRoot 'app/src/main/res/layout/activity_main.xml'
$stringsPath = Join-Path $appRoot 'app/src/main/res/values/strings.xml'
$javaPath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/MainActivity.java'
$statePath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/PairingDemoState.java'
$appGradlePath = Join-Path $appRoot 'app/build.gradle.kts'
$nativeBridgePath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/NativePairingBridge.java'
$repoRoot = Split-Path (Split-Path $appRoot -Parent) -Parent
$nativePairingPath = Join-Path $repoRoot 'crates/mm-ffi/src/android_tv_pairing.rs'
$nativeContextPath = Join-Path $repoRoot 'crates/mm-ffi/src/android_context.rs'
$androidNamespace = 'http://schemas.android.com/apk/res/android'

$manifest = [xml](Get-Content -LiteralPath $manifestPath -Raw)
$featureNames = @(
    $manifest.SelectNodes("//*[local-name()='uses-feature']") |
        ForEach-Object { $_.GetAttribute('name', $androidNamespace) }
)
if ('android.software.leanback' -notin $featureNames) {
    throw 'Manifest must require the Android TV Leanback feature.'
}
if ('android.hardware.touchscreen' -notin $featureNames) {
    throw 'Manifest must declare touchscreen as optional.'
}
$permissions = @(
    $manifest.SelectNodes("//*[local-name()='uses-permission']") |
        ForEach-Object { $_.GetAttribute('name', $androidNamespace) }
)
if ('android.permission.INTERNET' -notin $permissions) {
    throw 'WebRTC requires the ordinary Android INTERNET permission.'
}
if ($permissions | Where-Object { $_ -match 'CAMERA|RECORD_AUDIO|MediaProjection|FOREGROUND_SERVICE' }) {
    throw 'The receive-only TV target must not request capture, camera, or microphone permissions.'
}
if ($manifest.SelectSingleNode("//*[local-name()='application']").GetAttribute('usesCleartextTraffic', $androidNamespace) -ne 'false') {
    throw 'TV signaling must use TLS; cleartext traffic must remain disabled.'
}

$categories = @(
    $manifest.SelectNodes("//*[local-name()='category']") |
        ForEach-Object { $_.GetAttribute('name', $androidNamespace) }
)
if ('android.intent.category.LEANBACK_LAUNCHER' -notin $categories) {
    throw 'Manifest must register a Leanback launcher activity.'
}

$resourceFiles = Get-ChildItem -LiteralPath (Join-Path $appRoot 'app/src/main/res') -Filter '*.xml' -Recurse
foreach ($file in $resourceFiles) {
    [xml](Get-Content -LiteralPath $file.FullName -Raw) | Out-Null
}

$layout = [xml](Get-Content -LiteralPath $layoutPath -Raw)
$buttons = $layout.SelectNodes("//*[local-name()='Button']")
if ($buttons.Count -lt 5) {
    throw 'TV screen must include its five focusable action buttons.'
}
foreach ($button in $buttons) {
    if ($button.GetAttribute('focusable', $androidNamespace) -ne 'true' -or
        $button.GetAttribute('focusableInTouchMode', $androidNamespace) -ne 'true') {
        throw 'Every TV action button must support D-pad focus.'
    }
}

$helpText = $layout.SelectNodes("//*[local-name()='TextView']") |
    Where-Object { $_.GetAttribute('id', $androidNamespace) -eq '@+id/help_text' } |
    Select-Object -First 1
if ($null -eq $helpText -or
    $helpText.GetAttribute('accessibilityLiveRegion', $androidNamespace) -ne 'polite') {
    throw 'Dynamic TV approval help text must announce politely to accessibility services.'
}

$activity = Get-Content -LiteralPath $javaPath -Raw
if (!(Test-Path -LiteralPath $statePath)) {
    throw 'TV approval controls must use the tested local state model.'
}
foreach ($action in @('showDemoRequest', 'accept', 'reject', 'disconnect')) {
    if ($activity -notmatch [regex]::Escape("demoState.$action()")) {
        throw "TV activity must wire the $action action to an explicit user control."
    }
}
$receiverPath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/TvWebRtcReceiver.java'
$receiverSource = Get-Content -LiteralPath $receiverPath -Raw
if ($receiverSource -notmatch 'HardwareVideoDecoderFactory' -or
    $receiverSource -notmatch 'SurfaceViewRenderer' -or
    $receiverSource -notmatch 'H264SdpPolicy\.requireH264VideoCodec' -or
    $receiverSource -notmatch 'setCodecPreferences\(' -or
    $receiverSource -notmatch 'RtpTransceiverDirection\.RECV_ONLY' -or
    $receiverSource -notmatch 'H264SdpPolicy\.requireOnlyH264VideoCodec' -or
    $receiverSource -notmatch 'createPeerConnection\(' -or
    $receiverSource -notmatch 'setRemoteDescription\(' -or
    $receiverSource -notmatch '\.dispose\(') {
    throw 'The native receive adapter must require H.264, use hardware decoding, and render to a native surface.'
}
if ($receiverSource -match 'MediaProjection|ScreenCapturerAndroid|Camera2Enumerator|Camera1Enumerator') {
    throw 'The TV receiver must not start local screen or camera capture.'
}
$appGradle = Get-Content -LiteralPath $appGradlePath -Raw
if ($appGradle -notmatch 'io\.github\.webrtc-sdk:android:150\.7871\.01') {
    throw 'The Android TV app must pin the reviewed native WebRTC artifact version.'
}
$bridge = Get-Content -LiteralPath $nativeBridgePath -Raw
foreach ($method in @('initializeRustAndroidContext', 'startPairing', 'waitForPairingRequest', 'approvePairing', 'rejectPairing', 'stopPairing', 'closeControlChannel')) {
    $methodPattern = "native\s+\w+\s+$([regex]::Escape($method))\s*\("
    if ($bridge -notmatch $methodPattern) {
        throw "Native TV pairing bridge is missing $method."
    }
}
if ($bridge -notmatch 'System\.loadLibrary\("mm_ffi"\)') {
    throw 'The TV native bridge must load the packaged Rust library.'
}
if ($appGradle -notmatch 'aarch64-linux-android' -or $appGradle -notmatch 'libmm_ffi\.so') {
    throw 'The Android TV Gradle build must package the Rust ARM64 library.'
}
$nativePairing = Get-Content -LiteralPath $nativePairingPath -Raw
foreach ($jniName in @('startPairing', 'waitForPairingRequest', 'approvePairing', 'rejectPairing', 'stopPairing', 'closeControlChannel')) {
    if ($nativePairing -notmatch [regex]::Escape("NativePairingBridge_$jniName")) {
        throw "Rust JNI implementation is missing $jniName."
    }
}
$nativeContext = Get-Content -LiteralPath $nativeContextPath -Raw
if ($nativeContext -notmatch 'Java_com_mirrormesh_tv_NativePairingBridge_initializeRustAndroidContext') {
    throw 'Rust must initialize the Android context before opening Android Keystore or private storage.'
}

$strings = [xml](Get-Content -LiteralPath $stringsPath -Raw)
$demoStatus = $strings.SelectSingleNode("//*[local-name()='string' and @name='local_demo_status']")
if ($null -eq $demoStatus -or $demoStatus.InnerText -notmatch 'No network or video') {
    throw 'The UI must disclose that its controls are local-only and do not stream media.'
}

Write-Output "Android TV source checks passed: $($resourceFiles.Count) XML resources, $($buttons.Count) D-pad buttons, explicit approval shell, INTERNET only, hardware H.264 receiver, no local capture permissions."
