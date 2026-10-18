$ErrorActionPreference = 'Stop'

$appRoot = Split-Path $PSScriptRoot -Parent
$manifestPath = Join-Path $appRoot 'app/src/main/AndroidManifest.xml'
$layoutPath = Join-Path $appRoot 'app/src/main/res/layout/activity_main.xml'
$stringsPath = Join-Path $appRoot 'app/src/main/res/values/strings.xml'
$javaPath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/MainActivity.java'
$statePath = Join-Path $appRoot 'app/src/main/java/com/mirrormesh/tv/PairingDemoState.java'
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
if ($manifest.SelectNodes("//*[local-name()='uses-permission']").Count -ne 0) {
    throw 'Standby shell must not request network, capture, camera, or microphone permissions.'
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
if ($activity -match 'android\.net|android\.media|MediaProjection|MediaCodec|WifiManager') {
    throw 'The local UI demonstration must not access network, capture, or media APIs.'
}

$strings = [xml](Get-Content -LiteralPath $stringsPath -Raw)
$demoStatus = $strings.SelectSingleNode("//*[local-name()='string' and @name='local_demo_status']")
if ($null -eq $demoStatus -or $demoStatus.InnerText -notmatch 'No network or video') {
    throw 'The UI must disclose that its controls are local-only and do not stream media.'
}

Write-Output "Android TV source shell checks passed: $($resourceFiles.Count) XML resources, $($buttons.Count) D-pad buttons, explicit local approval flow, no permissions or media APIs."
