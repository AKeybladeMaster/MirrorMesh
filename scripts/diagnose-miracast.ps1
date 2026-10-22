# Read-only capability/status report. Does not launch a receiver or change settings.
$ErrorActionPreference = 'Stop'
$package = Get-AppxPackage -Name Microsoft.PPIProjection -ErrorAction SilentlyContinue
Write-Output ('Windows Wireless Display installed: {0}' -f [bool]$package)
try {
    Add-Type -AssemblyName System.Runtime.WindowsRuntime
    $receiverType = [Windows.Media.Miracast.MiracastReceiver,Windows.Media.Miracast,ContentType=WindowsRuntime]
    $statusType = [Windows.Media.Miracast.MiracastReceiverStatus,Windows.Media.Miracast,ContentType=WindowsRuntime]
    $receiver = [Activator]::CreateInstance($receiverType)
    $operation = $receiver.GetStatusAsync()
    $asTask = [System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
        $_.Name -eq 'AsTask' -and $_.IsGenericMethodDefinition -and
        $_.GetParameters().Count -eq 1 -and
        $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
    } | Select-Object -First 1
    $task = $asTask.MakeGenericMethod($statusType).Invoke($null, @($operation))
    if (-not $task.Wait(7000)) { $operation.Cancel(); throw 'Windows status query timed out.' }
    Write-Output ('Wi-Fi capability: {0}' -f $task.Result.WiFiStatus)
    Write-Output ('Receiver status: {0}' -f $task.Result.ListeningStatus)
} catch {
    Write-Output ('Windows Miracast status unavailable (HRESULT 0x{0:X8}).' -f ($_.Exception.HResult -band 0xffffffffL))
}
$service = Get-Service -Name WlanSvc -ErrorAction SilentlyContinue
Write-Output ('Wi-Fi service: {0}' -f $(if ($service) { $service.Status } else { 'unavailable' }))
Write-Output 'If the PC appears but fails to connect, discovery works; this report does not prove media compatibility.'
Write-Output 'Keep Wireless Display open; note its exact error. Check Wi-Fi/display driver support using the manufacturers support pages.'
Write-Output 'Use Android Wireless display/Miracast, not Google Cast. MirrorMesh app sharing remains a separate option.'
Write-Output 'No driver, firewall, policy, network profile or approval setting was changed. No private addresses were collected.'
