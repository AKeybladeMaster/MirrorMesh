param([string]$BundleDirectory = '', [switch]$CheckStartup)
$ErrorActionPreference = 'Stop'
if (-not $BundleDirectory) {
    $BundleDirectory = if (Test-Path -LiteralPath (Join-Path $PSScriptRoot 'MirrorMesh.exe')) {
        $PSScriptRoot
    } else { Join-Path (Split-Path -Parent $PSScriptRoot) 'releases/windows-x64' }
}
$BundleDirectory = (Resolve-Path -LiteralPath $BundleDirectory).Path
$runtime = Join-Path $BundleDirectory 'airplay'
foreach ($relative in @('bin/uxplay.exe','bin/libgstreamer-1.0-0.dll','libexec/gstreamer-1.0/gst-plugin-scanner.exe')) {
    if (-not (Test-Path -LiteralPath (Join-Path $runtime $relative))) {
        throw "Missing AirPlay runtime file: $relative. Extract the complete Windows ZIP into a new folder."
    }
}
function Invoke-AirPlayDiagnostic([string]$ExecutablePath, [string]$Arguments, [int]$TimeoutMs) {
    $launch = New-Object System.Diagnostics.ProcessStartInfo
    $launch.FileName = $ExecutablePath
    $launch.Arguments = $Arguments
    $launch.WorkingDirectory = $BundleDirectory
    $launch.UseShellExecute = $false
    $launch.CreateNoWindow = $true
    $launch.RedirectStandardOutput = $true
    $launch.RedirectStandardError = $true
    $launch.EnvironmentVariables['PATH'] = (Join-Path $runtime 'bin') + ";$env:SystemRoot/System32;$env:SystemRoot"
    $process = New-Object System.Diagnostics.Process
    $process.StartInfo = $launch
    try {
        if (-not $process.Start()) { throw 'Could not launch the local AirPlay diagnostic.' }
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        if (-not $process.WaitForExit($TimeoutMs)) {
            $process.Kill()
            $process.WaitForExit()
            throw 'AirPlay diagnostic timed out. Its owned receiver process is stopped on exit.'
        }
        [void]$stderr.Result
        return [pscustomobject]@{ code = $process.ExitCode; text = $stdout.Result }
    } finally { $process.Dispose() }
}
$helper = Invoke-AirPlayDiagnostic (Join-Path $runtime 'bin/uxplay.exe') '-h' 10000
Write-Output ('Packaged helper loader: exit {0} (0x{1:X8}).' -f $helper.code, ($helper.code -band 0xffffffffL))
if ($helper.code -ne 0 -or $helper.text -notmatch 'UxPlay .*AirPlay mirroring server') {
    throw 'AirPlay helper could not load. Re-extract the complete Windows x64 bundle; do not mix DLLs from another release.'
}
if ($CheckStartup) {
    Write-Output 'Starting the receiver briefly on this LAN. No client will be approved; reception stops after the readiness check.'
    $startup = Invoke-AirPlayDiagnostic (Join-Path $BundleDirectory 'MirrorMesh.exe') '--diagnose-airplay' 25000
    # Only app-generated metadata is printed, never arbitrary native logs or PINs.
    $startup.text -split "`n" | Where-Object { $_ -match '^AirPlay startup: ' } | ForEach-Object { Write-Output $_.Trim() }
    Write-Output "Native receiver startup: exit $($startup.code)."
    if ($startup.code -ne 0 -or $startup.text -notmatch 'AirPlay startup: READY') { throw 'Native AirPlay receiver startup failed.' }
} else {
    Write-Output 'No receiver started. Use -CheckStartup to diagnose native initialization and discovery registration.'
}
