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
$op = $mgrType::RequestAsync()
$mgr = Await $op $mgrType
if (-not $mgr) { "no manager"; exit 1 }
$sessions = $mgr.GetSessions()
"sessions: $($sessions.Count)"
foreach ($s in $sessions) {
    "  app=$($s.SourceAppUserModelId) status=$($s.GetPlaybackInfo().PlaybackStatus)"
    $propsOp = $s.TryGetMediaPropertiesAsync()
    $propsType = [Windows.Media.Control.GlobalSystemMediaTransportControlsSessionMediaProperties, Windows.Media.Control, ContentType = WindowsRuntime]
    $props = Await $propsOp $propsType
    if ($props) { "    title=$($props.Title) artist=$($props.Artist) album=$($props.AlbumTitle)" }
}
