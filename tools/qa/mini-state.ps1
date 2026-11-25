param(
  [string]$Current = "",
  [string]$Theme = "paper",
  [string]$Style = "classic",
  [string]$Out = "tools/qa/out/mini",
  [switch]$Hover
)

# Drive the mini player into a deterministic state and capture it.
# Usage: .\tools\qa\mini-state.ps1 -Current 7038494800498988877 -Theme paper -Style classic -Out docs/qa/v0.2/v220-S2-mini-classic-paper-art

$ErrorActionPreference = "Stop"
Get-Process rplayer -ErrorAction SilentlyContinue | Stop-Process -Force
Start-Sleep -Milliseconds 700

$session = "$env:APPDATA\RPlayer\session.json"
$s = Get-Content $session -Raw | ConvertFrom-Json
if ($Current -ne "") {
  $s.current = [uint64]$Current
  $s.position = 5.0
} else {
  $s.current = $null
  $s.position = 0.0
}
($s | ConvertTo-Json -Depth 10) | Set-Content $session -Encoding UTF8

$lib = "$env:APPDATA\RPlayer\library.json"
$o = Get-Content $lib -Raw | ConvertFrom-Json
$o.theme_preset = $Theme
$o.light_theme = ($Theme -eq "paper")
$o.mini_style = $Style
($o | ConvertTo-Json -Depth 20) | Set-Content $lib -Encoding UTF8

Start-Process 'D:\Workspace\RPlayer\target\release\rplayer.exe' -WorkingDirectory 'D:\Workspace\RPlayer'
Start-Sleep -Seconds 3

Add-Type -TypeDefinition 'using System;using System.Runtime.InteropServices;public class MS{[DllImport("user32.dll")]public static extern bool SetCursorPos(int x,int y);[DllImport("user32.dll")]public static extern void mouse_event(uint f,int dx,int dy,uint d,System.UIntPtr e);}'

# The player-bar mini chip sits at a fixed offset from the main window origin.
$w = Get-Process rplayer | Where-Object { $_.MainWindowHandle -ne 0 } | Select-Object -First 1
$rect = New-Object -TypeName psobject
Add-Type -TypeDefinition 'using System;using System.Runtime.InteropServices;public class WR{[DllImport("user32.dll")]public static extern bool GetWindowRect(IntPtr h,out R r);[DllImport("user32.dll")]public static extern bool SetWindowPos(IntPtr h,IntPtr after,int x,int y,int cx,int cy,uint flags);public struct R{public int L,T,Rr,B;}}'

# Launch cascades the window; pin it back to a known origin before clicking.
[void][WR]::SetWindowPos($w.MainWindowHandle, [IntPtr]::Zero, 78, 78, 1440, 900, 0x0004)
Start-Sleep -Milliseconds 500

$r = New-Object WR+R
[void][WR]::GetWindowRect($w.MainWindowHandle, [ref]$r)
$cx = $r.L + 1261
$cy = $r.T + 853
Write-Output "main rect=$($r.L),$($r.T) chip=$cx,$cy"

[void][MS]::SetCursorPos($cx, $cy)
Start-Sleep -Milliseconds 120
[MS]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 60
[MS]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
Start-Sleep -Milliseconds 1100

Add-Type -TypeDefinition 'using System;using System.Runtime.InteropServices;public class MF{[DllImport("user32.dll")]public static extern bool EnumWindows(EnumProc cb, IntPtr l);public delegate bool EnumProc(IntPtr h, IntPtr l);[DllImport("user32.dll", CharSet=CharSet.Unicode)]public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder s, int n);[DllImport("user32.dll")]public static extern bool GetWindowRect(IntPtr h, out R r);public struct R{public int L,T,Rr,B;}}'
$script:mini = [IntPtr]::Zero
$cb = [MF+EnumProc]{
  param($h, $l)
  $sb = New-Object System.Text.StringBuilder 512
  [void][MF]::GetWindowTextW($h, $sb, 512)
  if ($sb.ToString() -eq "RPlayer 迷你播放器") { $script:mini = $h; return $false }
  return $true
}
[void][MF]::EnumWindows($cb, [IntPtr]::Zero)
$mr = New-Object MF+R
[void][MF]::GetWindowRect($script:mini, [ref]$mr)

if ($Hover) {
  # Park the pointer just inside the card so the 55% action strip ramps to 100%.
  [void][MS]::SetCursorPos($mr.L + 200, $mr.T + 100)
  Start-Sleep -Milliseconds 420
} else {
  [void][MS]::SetCursorPos(200, 1000)
  Start-Sleep -Milliseconds 450
}

& (Join-Path $PSScriptRoot "capture.ps1") -Title "RPlayer 迷你播放器" -Out $Out
