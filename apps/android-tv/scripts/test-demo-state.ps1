$ErrorActionPreference = 'Stop'

$appRoot = Split-Path $PSScriptRoot -Parent
$statePath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/PairingDemoState.java'
$testPath = Join-Path $appRoot 'app/src/test/java/com/mirrormesh/tv/PairingDemoStateTest.java'
$outputPath = Join-Path $env:TEMP 'MirrorMeshAndroidTvStateTests'
$javac = if ($env:JAVA_HOME) { Join-Path $env:JAVA_HOME 'bin/javac.exe' } else { 'javac' }
$java = if ($env:JAVA_HOME) { Join-Path $env:JAVA_HOME 'bin/java.exe' } else { 'java' }

if (!(Test-Path -LiteralPath $statePath)) {
    throw "State implementation not found: $statePath"
}
if (!(Test-Path -LiteralPath $testPath)) {
    throw "State test not found: $testPath"
}

New-Item -ItemType Directory -Path $outputPath -Force | Out-Null
& $javac --release 17 -d $outputPath $statePath $testPath
if ($LASTEXITCODE -ne 0) {
    throw "javac failed with exit code $LASTEXITCODE"
}

& $java -cp $outputPath com.mirrormesh.tv.PairingDemoStateTest
if ($LASTEXITCODE -ne 0) {
    throw "Java state test failed with exit code $LASTEXITCODE"
}
