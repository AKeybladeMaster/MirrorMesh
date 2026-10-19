$ErrorActionPreference = 'Stop'

$appRoot = Split-Path $PSScriptRoot -Parent
$statePath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/PairingDemoState.java'
$testPath = Join-Path $appRoot 'app/src/test/java/com/mirrormesh/tv/PairingDemoStateTest.java'
$h264PolicyPath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/H264SdpPolicy.java'
$h264PolicyTestPath = Join-Path $appRoot 'app/src/test/java/com/mirrormesh/tv/H264SdpPolicyTest.java'
$receiverStatePath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/TvReceiverState.java'
$reconnectWatchdogPath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/TvReconnectWatchdog.java'
$pairingStatePath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/TvPairingState.java'
$nativeBridgePath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/NativePairingBridge.java'
$receiverStateTestPath = Join-Path $appRoot 'app/src/test/java/com/mirrormesh/tv/TvReceiverStateTest.java'
$reconnectWatchdogTestPath = Join-Path $appRoot 'app/src/test/java/com/mirrormesh/tv/TvReconnectWatchdogTest.java'
$pairingStateTestPath = Join-Path $appRoot 'app/src/test/java/com/mirrormesh/tv/TvPairingStateTest.java'
$screenshotFramePath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/TvScreenshotFrame.java'
$screenshotFrameTestPath = Join-Path $appRoot 'app/src/test/java/com/mirrormesh/tv/TvScreenshotFrameTest.java'
$sessionStartNoticePath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/SessionStartNotice.java'
$sessionStartNoticeTestPath = Join-Path $appRoot 'app/src/test/java/com/mirrormesh/tv/SessionStartNoticeTest.java'
$outputPath = Join-Path $env:TEMP 'MirrorMeshAndroidTvStateTests'
$javac = if ($env:JAVA_HOME) { Join-Path $env:JAVA_HOME 'bin/javac.exe' } else { 'javac' }
$java = if ($env:JAVA_HOME) { Join-Path $env:JAVA_HOME 'bin/java.exe' } else { 'java' }

if (!(Test-Path -LiteralPath $statePath)) {
    throw "State implementation not found: $statePath"
}
if (!(Test-Path -LiteralPath $testPath)) {
    throw "State test not found: $testPath"
}
$requiredPaths = @(
    $h264PolicyPath,
    $h264PolicyTestPath,
    $receiverStatePath,
    $receiverStateTestPath,
    $reconnectWatchdogPath,
    $reconnectWatchdogTestPath,
    $pairingStatePath,
    $pairingStateTestPath,
    $nativeBridgePath,
    $screenshotFramePath,
    $screenshotFrameTestPath,
    $sessionStartNoticePath,
    $sessionStartNoticeTestPath
)
foreach ($requiredPath in $requiredPaths) {
    if (!(Test-Path -LiteralPath $requiredPath)) {
        throw "Android TV receiver test input not found: $requiredPath"
    }
}

New-Item -ItemType Directory -Path $outputPath -Force | Out-Null
$javacInputs = @(
    $statePath,
    $testPath,
    $h264PolicyPath,
    $h264PolicyTestPath,
    $receiverStatePath,
    $receiverStateTestPath,
    $reconnectWatchdogPath,
    $reconnectWatchdogTestPath,
    $pairingStatePath,
    $pairingStateTestPath,
    $nativeBridgePath,
    $screenshotFramePath,
    $screenshotFrameTestPath,
    $sessionStartNoticePath,
    $sessionStartNoticeTestPath
)
& $javac --release 17 -d $outputPath @javacInputs
if ($LASTEXITCODE -ne 0) {
    throw "javac failed with exit code $LASTEXITCODE"
}

foreach ($testClass in @(
    'PairingDemoStateTest',
    'H264SdpPolicyTest',
    'TvReceiverStateTest',
    'TvReconnectWatchdogTest',
    'TvPairingStateTest',
    'TvScreenshotFrameTest',
    'SessionStartNoticeTest'
)) {
    & $java -cp $outputPath "com.mirrormesh.tv.$testClass"
    if ($LASTEXITCODE -ne 0) {
        throw "Java test $testClass failed with exit code $LASTEXITCODE"
    }
}
