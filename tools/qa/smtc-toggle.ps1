Add-Type -AssemblyName System.Runtime.WindowsRuntime
$asTask = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
    $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and
    $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'
})[0]

function Await($op, $type) {
    $task = $asTask.MakeGenericMethod($type).Invoke($null, @($op))
    $task.Wait(8000) | Out-Null
    if ($task.IsCompleted) { $task.Result } else { $null }
}

$mgrType = [Windows.Media.Control.GlobalSystemMediaTransportControlsSessionManager, Windows.Media.Control, ContentType = WindowsRuntime]
$mgr = Await ($mgrType::RequestAsync()) $mgrType
$s = ($mgr.GetSessions() | Where-Object { $_.SourceAppUserModelId -eq 'RPlayer' })[0]
if (-not $s) { "no rplayer session"; exit 1 }
"before: $($s.GetPlaybackInfo().PlaybackStatus)"
$bool = [bool]
$op = $s.TryTogglePlayPauseAsync()
$task = $asTask.MakeGenericMethod($bool).Invoke($null, @($op))
$task.Wait(8000) | Out-Null
"toggle ok=$($task.Result)"
Start-Sleep -Milliseconds 1200
"after: $($mgr.GetSessions()[0].GetPlaybackInfo().PlaybackStatus)"
