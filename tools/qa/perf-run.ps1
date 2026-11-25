param([int]$TimeoutSec = 900, [long]$Hwnd = 0)
Add-Type -Namespace PF -Name W -MemberDefinition '[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern bool EnumWindows(EnumWindowsProc cb, IntPtr l); public delegate bool EnumWindowsProc(IntPtr h, IntPtr l); [DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder s, int n); [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid); [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);'
function Get-RPlayerWindow([int]$procId) {
  $script:found = [IntPtr]::Zero
  $cb = [PF.W+EnumWindowsProc]{ param($h,$l) $pp=0; [void][PF.W]::GetWindowThreadProcessId($h,[ref]$pp); if($pp -eq $procId -and [PF.W]::IsWindowVisible($h)){ $sb=New-Object System.Text.StringBuilder 512; [void][PF.W]::GetWindowTextW($h,$sb,512); if($sb.ToString() -match ' — '){ $script:found = $h; return $false } }; return $true }
  [void][PF.W]::EnumWindows($cb,[IntPtr]::Zero)
  return $script:found
}
function Find-Node($n,[string]$pat){ if($n.name -and $n.name -match $pat){ return $n.name }; foreach($c in $n.children){ $r=Find-Node $c $pat; if($r){ return $r } }; return $null }

$sw = [Diagnostics.Stopwatch]::StartNew()
if ($Hwnd -eq 0) {
  $proc = Start-Process 'D:\Workspace\RPlayer\target\release\rplayer.exe' -WorkingDirectory 'D:\Workspace\RPlayer' -PassThru
  $h = [IntPtr]::Zero
  while ($sw.Elapsed.TotalSeconds -lt $TimeoutSec) {
    $h = Get-RPlayerWindow $proc.Id
    if ($h -ne [IntPtr]::Zero) { break }
    Start-Sleep -Milliseconds 40
  }
  "first-interactive-window {0:N2}s hwnd={1} pid={2}" -f $sw.Elapsed.TotalSeconds, $h, $proc.Id
} else {
  $h = [IntPtr]$Hwnd
  $proc = Get-Process rplayer | Select-Object -First 1
  "attach hwnd=$h pid=$($proc.Id)"
}
Start-Sleep -Seconds 2
$count = $null
while ($sw.Elapsed.TotalSeconds -lt $TimeoutSec) {
  $out = agent-desktop snapshot --window-id ("w-" + $h) 2>&1 | Out-String
  try { $j = $out | ConvertFrom-Json } catch { $j = $null }
  if ($j -and $j.ok) {
    $count = Find-Node $j.data.tree '首歌曲'
    if ($count) {
      $n = 0
      if ($count -match '([\d,]+)\s*首歌曲') { $n = [int](($matches[1]) -replace ',', '') }
      "  {0,6:N1}s  {1}" -f $sw.Elapsed.TotalSeconds, $count
      if ($n -ge 10000) { break }
    }
  }
  Start-Sleep -Seconds 5
}
$p = Get-Process -Id $proc.Id
"scan-done {0:N1}s ws={1:N1}MB threads={2} handles={3} count=[{4}]" -f $sw.Elapsed.TotalSeconds, ($p.WorkingSet64/1MB), $p.Threads.Count, $p.HandleCount, $count
