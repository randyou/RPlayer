param([int]$WheelSteps = 120, [int]$StepDelayMs = 25)
Add-Type -Namespace SC -Name M -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y); [DllImport("user32.dll")] public static extern void mouse_event(uint flags, int dx, int dy, uint data, System.UIntPtr extra); [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);'
$log = 'D:\Workspace\_frames.csv'
if (Test-Path $log) { Remove-Item $log -Force }
$env:RPLAYER_FRAME_LOG = $log
$proc = Start-Process 'D:\Workspace\RPlayer\target\release\rplayer.exe' -WorkingDirectory 'D:\Workspace\RPlayer' -PassThru
Start-Sleep -Seconds 6
$h = (Get-Process -Id $proc.Id).MainWindowHandle
"pid=$($proc.Id) hwnd=$h frames-before-scroll=$((Get-Content $log -ErrorAction SilentlyContinue | Measure-Object).Count)"
# Put the pointer over the middle of the track table and focus the window.
[void][SC.M]::SetForegroundWindow([IntPtr]$h)
Start-Sleep -Milliseconds 400
[void][SC.M]::SetCursorPos(700, 600)
Start-Sleep -Milliseconds 300
$sw = [Diagnostics.Stopwatch]::StartNew()
for ($i = 0; $i -lt $WheelSteps; $i++) {
  [SC.M]::mouse_event(0x0800, 0, 0, [uint32]4294967176, [UIntPtr]::Zero)  # wheel down
  Start-Sleep -Milliseconds $StepDelayMs
}
for ($i = 0; $i -lt $WheelSteps; $i++) {
  [SC.M]::mouse_event(0x0800, 0, 0, 120, [UIntPtr]::Zero)                 # wheel up
  Start-Sleep -Milliseconds $StepDelayMs
}
$scrollSec = $sw.Elapsed.TotalSeconds
"scroll-seconds {0:N1}" -f $scrollSec
Start-Sleep -Milliseconds 800
$frames = Get-Content $log | Where-Object { $_ -match '^\d' } | ForEach-Object { [double](($_ -split "`t")[0]) }
# Drop the warm-up frames: the very first `update` pays for the font atlas and
# shader compilation (100-250ms) and would poison `max` and the P95 tail.
$frames = $frames | Select-Object -Skip 8
$sorted = $frames | Sort-Object
$p50 = $sorted[[int]($sorted.Count * 0.50)]
$p95 = $sorted[[int]($sorted.Count * 0.95)]
$max = ($sorted | Measure-Object -Maximum).Maximum
"frames={0} p50={1:N1}ms p95={2:N1}ms max={3:N1}ms" -f $frames.Count, $p50, $p95, $max
$p = Get-Process -Id $proc.Id
"ws={0:N1}MB cpu={1:N1}s" -f ($p.WorkingSet64/1MB), $p.TotalProcessorTime.TotalSeconds
Stop-Process -Id $proc.Id -Force
