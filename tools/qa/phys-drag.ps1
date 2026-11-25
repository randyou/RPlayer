param([int]$X,[int]$Y,[int]$DX=0,[int]$DY=0,[string]$Title="RPlayer 迷你播放器",[string]$Out="tools/qa/out/drag")

Add-Type -AssemblyName System.Drawing
Add-Type -Namespace DRG -Name W -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
[DllImport("user32.dll")] public static extern void mouse_event(uint flags, int dx, int dy, uint data, System.UIntPtr extra);
public struct RECT { public int L,T,R,B; }
'@

function Find-Title([string]$want) {
  $script:target = [IntPtr]::Zero
  $cb = [DRG.W+EnumProc]{
    param($h, $l)
    $sb = New-Object System.Text.StringBuilder 512
    [void][DRG.W]::GetWindowTextW($h, $sb, 512)
    if ($sb.ToString() -eq $want) { $script:target = $h; return $false }
    return $true
  }
  [void][DRG.W]::EnumWindows($cb, [IntPtr]::Zero)
  return $script:target
}

$hwnd = Find-Title $Title
if ($hwnd -eq [IntPtr]::Zero) { Write-Output "WINDOW_NOT_FOUND"; exit 1 }

[void][DRG.W]::SetCursorPos($X, $Y)
Start-Sleep -Milliseconds 150
[DRG.W]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)   # left down
Start-Sleep -Milliseconds 120

$steps = 10
for ($i = 1; $i -le $steps; $i++) {
  [void][DRG.W]::SetCursorPos($X + [int]($DX * $i / $steps), $Y + [int]($DY * $i / $steps))
  Start-Sleep -Milliseconds 45
}
Start-Sleep -Milliseconds 250

$r = New-Object DRG.W+RECT
[void][DRG.W]::GetWindowRect($hwnd, [ref]$r)
$w = $r.R - $r.L; $h = $r.B - $r.T
Write-Output "hwnd=$hwnd rect=$($r.L),$($r.T),$w,$h"

$bmp = New-Object System.Drawing.Bitmap $w, $h
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size $w, $h))
$g.Dispose()
$bmp.Save("$Out-screen.png", [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Output "screen -> $Out-screen.png"

$bmp2 = New-Object System.Drawing.Bitmap $w, $h
$g2 = [System.Drawing.Graphics]::FromImage($bmp2)
$hdc = $g2.GetHdc()
$ok = [DRG.W]::PrintWindow($hwnd, $hdc, 2)
$g2.ReleaseHdc($hdc)
$g2.Dispose()
$bmp2.Save("$Out-print.png", [System.Drawing.Imaging.ImageFormat]::Png)
$bmp2.Dispose()
Write-Output "printwindow=$ok -> $Out-print.png"

[DRG.W]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)   # left up
Start-Sleep -Milliseconds 200
[void][DRG.W]::SetCursorPos(1600, 1000)
Write-Output "drag done"
