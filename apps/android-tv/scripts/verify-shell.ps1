$ErrorActionPreference = 'Stop'

$appRoot = Split-Path $PSScriptRoot -Parent
$manifestPath = Join-Path $appRoot 'app/src/main/AndroidManifest.xml'
$layoutPath = Join-Path $appRoot 'app/src/main/res/layout/activity_main.xml'
$videoLayoutPath = Join-Path $appRoot 'app/src/main/res/layout/activity_tv_video.xml'
$stringsPath = Join-Path $appRoot 'app/src/main/res/values/strings.xml'
$javaPath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/MainActivity.java'
$appGradlePath = Join-Path $appRoot 'app/build.gradle.kts'
$nativeBridgePath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/NativePairingBridge.java'
$repoRoot = Split-Path (Split-Path $appRoot -Parent) -Parent
$nativePairingPath = Join-Path $repoRoot 'crates/mm-ffi/src/android_tv_pairing.rs'
$nativeContextPath = Join-Path $repoRoot 'crates/mm-ffi/src/android_context.rs'
$approvedChannelPath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/RustApprovedSignalingChannel.java'
$sessionStartNoticePath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/SessionStartNotice.java'
$pairingStatePath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/TvPairingState.java'
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
if ($permissions | Where-Object { $_ -match 'WRITE_EXTERNAL_STORAGE|READ_MEDIA_IMAGES|MANAGE_EXTERNAL_STORAGE' }) {
    throw 'Saving a user-requested TV screenshot must not require broad storage permissions.'
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
$videoLayout = [xml](Get-Content -LiteralPath $videoLayoutPath -Raw)
$buttons = $layout.SelectNodes("//*[local-name()='Button']")
if ($buttons.Count -lt 5) {
    throw 'TV screen must include its focusable pairing action buttons.'
}
foreach ($button in $buttons) {
    if ($button.GetAttribute('focusable', $androidNamespace) -ne 'true' -or
        $button.GetAttribute('focusableInTouchMode', $androidNamespace) -ne 'true') {
        throw 'Every TV action button must support D-pad focus.'
    }
}
$screenshotButton = $videoLayout.SelectNodes("//*[local-name()='Button']") |
    Where-Object { $_.GetAttribute('id', $androidNamespace) -eq '@+id/video_screenshot_button' } |
    Select-Object -First 1
if ($null -eq $screenshotButton -or
    $screenshotButton.GetAttribute('focusable', $androidNamespace) -ne 'true' -or
    $screenshotButton.GetAttribute('focusableInTouchMode', $androidNamespace) -ne 'true' -or
    $screenshotButton.GetAttribute('enabled', $androidNamespace) -ne 'false') {
    throw 'The screenshot action must be D-pad accessible and disabled until a frame is rendered.'
}
$grantControls = $layout.SelectNodes("//*[local-name()='CheckBox']")
if ($grantControls.Count -ne 3) {
    throw 'The TV pairing screen must expose the three independently scoped grant controls.'
}
foreach ($grantControl in $grantControls) {
    if ($grantControl.GetAttribute('focusable', $androidNamespace) -ne 'true' -or
        $grantControl.GetAttribute('focusableInTouchMode', $androidNamespace) -ne 'true') {
        throw 'Every TV grant control must support D-pad focus.'
    }
}

$helpText = $layout.SelectNodes("//*[local-name()='TextView']") |
    Where-Object { $_.GetAttribute('id', $androidNamespace) -eq '@+id/help_text' } |
    Select-Object -First 1
if ($null -eq $helpText -or
    $helpText.GetAttribute('accessibilityLiveRegion', $androidNamespace) -ne 'polite') {
    throw 'Dynamic TV approval help text must announce politely to accessibility services.'
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
if ($receiverSource -notmatch 'requestScreenshotCapture\(' -or
    $receiverSource -notmatch 'TvScreenshotFrame\.toNv21\(' -or
    $receiverSource -notmatch 'YuvImage' -or
    $receiverSource -notmatch 'screenshotExecutor\.shutdownNow\(') {
    throw 'The receiver must capture one bounded decoded frame on request and close the snapshot worker.'
}
$videoActivityPath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/TvVideoActivity.java'
$videoActivitySource = Get-Content -LiteralPath $videoActivityPath -Raw
if ($videoActivitySource -notmatch 'onScreenshotCaptured\(' -or
    $videoActivitySource -notmatch 'MediaStore\.Images\.Media\.EXTERNAL_CONTENT_URI' -or
    $videoActivitySource -notmatch 'getExternalFilesDir\(' -or
    $videoActivitySource -notmatch 'requestScreenshotCapture\(') {
    throw 'The fullscreen receiver must save an explicitly requested frame to a bounded user-visible destination.'
}
if ($receiverSource -match 'MediaProjection|ScreenCapturerAndroid|Camera2Enumerator|Camera1Enumerator') {
    throw 'The TV receiver must not start local screen or camera capture.'
}
$appGradle = Get-Content -LiteralPath $appGradlePath -Raw
if ($appGradle -notmatch 'io\.github\.webrtc-sdk:android:150\.7871\.01') {
    throw 'The Android TV app must pin the reviewed native WebRTC artifact version.'
}
if ($appGradle -notmatch 'com\.google\.zxing:core:3\.5\.4') {
    throw 'The Android TV QR encoder dependency must use the reviewed pinned ZXing core version.'
}
$bridge = Get-Content -LiteralPath $nativeBridgePath -Raw
foreach ($method in @('initializeRustAndroidContext', 'startPairing', 'refreshPairingInvitation', 'cancelPairingWait', 'waitForPairingRequest', 'approvePairing', 'rejectPairing', 'stopPairing', 'pollControlMessage', 'sendSessionAnswer', 'sendSessionIce', 'closeControlChannel')) {
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
foreach ($jniName in @('startPairing', 'refreshPairingInvitation', 'cancelPairingWait', 'waitForPairingRequest', 'approvePairing', 'rejectPairing', 'stopPairing', 'pollControlMessage', 'sendSessionAnswer', 'sendSessionIce', 'closeControlChannel')) {
    if ($nativePairing -notmatch [regex]::Escape("NativePairingBridge_$jniName")) {
        throw "Rust JNI implementation is missing $jniName."
    }
}
$mainActivity = Get-Content -LiteralPath $javaPath -Raw
foreach ($requiredSource in @('NativePairingBridge.startPairing', 'NativePairingBridge.waitForPairingRequest', 'NativePairingBridge.approvePairing', 'NativePairingBridge.rejectPairing', 'NativePairingBridge.stopPairing', 'QRCodeWriter', 'NetworkInterface.getNetworkInterfaces', 'onDestroy()')) {
    if ($mainActivity -notmatch [regex]::Escape($requiredSource)) {
        throw "Android TV activity is missing the real pairing flow element: $requiredSource."
    }
}
if ($mainActivity -notmatch 'RustApprovedSignalingChannel' -or
    $mainActivity -notmatch 'TvApprovedSessionLauncher\.open' -or
    $mainActivity -notmatch 'signaling\.start\(') {
    throw 'The approved TV pairing flow must route an authenticated sender offer into the native WebRTC receiver.'
}
if (!(Test-Path -LiteralPath $approvedChannelPath)) {
    throw 'The Rust-backed approved signaling channel adapter is missing.'
}
$approvedChannel = Get-Content -LiteralPath $approvedChannelPath -Raw
foreach ($requiredSignal in @('NativePairingBridge.pollControlMessage', 'NativePairingBridge.sendSessionAnswer', 'NativePairingBridge.sendSessionIce', 'MAX_BUFFERED_ICE_CANDIDATES', 'setRemoteIceCandidateListener', 'setRemoteStartListener', 'case "start"', 'source_id')) {
    if ($approvedChannel -notmatch [regex]::Escape($requiredSignal)) {
        throw "The approved WebRTC signaling adapter is missing $requiredSignal."
    }
}
$sessionStartNotice = Get-Content -LiteralPath $sessionStartNoticePath -Raw
if ($sessionStartNotice -notmatch 'validateSourceId\(' -or
    $approvedChannel -notmatch 'SessionStartNotice\.validateSourceId\(') {
    throw 'Session-start metadata must be validated before reaching the TV receiver.'
}
if ($nativePairing -notmatch 'try_receive_control_message' -or
    $nativePairing -notmatch 'MessagePayload::SessionOffer' -or
    $nativePairing -notmatch 'MessagePayload::SessionIce' -or
    $nativePairing -notmatch 'MessagePayload::SessionStart' -or
    $nativePairing -notmatch 'PeerGrant::View') {
    throw 'Rust must validate the approved view grant and typed session offer/ICE before crossing JNI.'
}
if ($mainActivity -notmatch 'grantView\.setChecked\(false\)' -or
    $mainActivity -notmatch 'grantSend\.setChecked\(false\)' -or
    $mainActivity -notmatch 'grantAnnotate\.setChecked\(false\)' -or
    $mainActivity -notmatch 'if \(!view && !send && !annotate\)') {
    throw 'Grant controls must start unchecked and approval must require an explicit non-empty selection.'
}
if ($mainActivity -match 'PairingDemoState') {
    throw 'The Android TV activity must not use local-only demo pairing state.'
}
if (!(Test-Path -LiteralPath $pairingStatePath)) {
    throw 'The Android TV explicit pairing state is missing.'
}
$pairingState = Get-Content -LiteralPath $pairingStatePath -Raw
if ($pairingState -notmatch 'REQUEST_PENDING' -or $pairingState -notmatch 'requestApproved') {
    throw 'Android TV pairing state must keep request arrival separate from approval.'
}
$nativeContext = Get-Content -LiteralPath $nativeContextPath -Raw
if ($nativeContext -notmatch 'Java_com_mirrormesh_tv_NativePairingBridge_initializeRustAndroidContext') {
    throw 'Rust must initialize the Android context before opening Android Keystore or private storage.'
}

$strings = [xml](Get-Content -LiteralPath $stringsPath -Raw)
$pairingMediaStatus = $strings.SelectSingleNode("//*[local-name()='string' and @name='pairing_paired']")
if ($null -eq $pairingMediaStatus -or $pairingMediaStatus.InnerText -notmatch 'video is not connected') {
    throw 'The pairing UI must state that an approved control channel does not imply a video stream.'
}

Write-Output "Android TV source checks passed: $($resourceFiles.Count) XML resources, $($buttons.Count) D-pad buttons, native pairing/signaling bridge, QR invitation, explicit grant approval, H.264 receiver handoff, INTERNET only, no local capture permissions."
