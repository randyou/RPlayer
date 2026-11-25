param([double]$Scale = 1.0, [int]$Width = 1440, [int]$Height = 900, [string]$Out = 'D:\Workspace\_dpi')
Add-Type -Namespace DP -Name W -MemberDefinition '[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr l); public delegate bool EnumWindowsProc(IntPtr h, IntPtr l); [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder s, int n); [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid); [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h); [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);'
function Get-MainWindow([int]$procId) {
  $script:found = [IntPtr]::Zero
  $cb = [DP.W+EnumWindowsProc]{ param($h,$l) $pp=0; [void][DP.W]::GetWindowThreadProcessId($h,[ref]$pp); if($pp -eq $procId -and [DP.W]::IsWindowVisible($h)){ $sb=New-Object System.Text.StringBuilder 512; [void][DP.W]::GetWindowTextW($h,$sb,512); if($sb.ToString() -match ' — '){ $script:found = $h; return $false } }; return $true }
  [void][DP.W]::EnumWindows($cb,[IntPtr]::Zero)
  return $script:found
}
Get-Process rplayer -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 600
$env:RPLAYER_UI_SCALE = $Scale
$env:RPLAYER_FRAME_LOG = 'D:\Workspace\_frames.csv'
$proc = Start-Process 'D:\Workspace\RPlayer\target\release\rplayer.exe' -WorkingDirectory 'D:\Workspace\RPlayer' -PassThru
$h = [IntPtr]::Zero
for ($i = 0; $i -lt 100; $i++) { $h = Get-MainWindow $proc.Id; if ($h -ne [IntPtr]::Zero) { break }; Start-Sleep -Milliseconds 100 }
Start-Sleep -Seconds 3
[void][DP.W]::SetWindowPos($h, [IntPtr]::Zero, 0, 0, $Width, $Height, 0x0004)
Start-Sleep -Milliseconds 900
& 'D:\Workspace\_shot.ps1' -Hwnd $h.ToInt64() -Out $Out | Select-Object -Last 1
$frames = Get-Content 'D:\Workspace\_frames.csv' | Where-Object { $_ -match '^\d' } | ForEach-Object { [double]$_ } | Sort-Object
"scale=$Scale size=${Width}x${Height} frames=$($frames.Count) p50=$([math]::Round($frames[[int]($frames.Count*0.5)],1))ms p95=$([math]::Round($frames[[int]($frames.Count*0.95)],1))ms ws=$([math]::Round((Get-Process -Id $proc.Id).WorkingSet64/1MB,1))MB"
