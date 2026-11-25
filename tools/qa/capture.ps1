param([string]$Title = "RPlayer 桌面歌词", [string]$Out = "D:\Workspace\_overlay")

Add-Type -AssemblyName System.Drawing
Add-Type -Namespace CAP -Name W -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder s, int n);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
public struct RECT { public int L,T,R,B; }
'@

$target = [IntPtr]::Zero
$cb = [CAP.W+EnumProc]{
    param($h, $l)
    $sb = New-Object System.Text.StringBuilder 512
    [void][CAP.W]::GetWindowTextW($h, $sb, 512)
    if ($sb.ToString() -eq $Title) { $script:target = $h; return $false }
    return $true
}
[void][CAP.W]::EnumWindows($cb, [IntPtr]::Zero)
if ($target -eq [IntPtr]::Zero) { Write-Output "WINDOW_NOT_FOUND"; exit 1 }

$r = New-Object CAP.W+RECT
[void][CAP.W]::GetWindowRect($target, [ref]$r)
$w = $r.R - $r.L
$hgt = $r.B - $r.T
Write-Output "hwnd=$target rect=$($r.L),$($r.T),$w,$hgt visible=$([CAP.W]::IsWindowVisible($target))"

# 1) window-content capture (excludes whatever is behind)
$bmp = New-Object System.Drawing.Bitmap $w, $hgt
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc()
$ok = [CAP.W]::PrintWindow($target, $hdc, 2)
$g.ReleaseHdc($hdc)
$g.Dispose()
$bmp.Save("$Out-print.png", [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Output "printwindow=$ok -> $Out-print.png"

# 2) plain screen capture of the same rect
$bmp2 = New-Object System.Drawing.Bitmap $w, $hgt
$g2 = [System.Drawing.Graphics]::FromImage($bmp2)
$g2.CopyFromScreen($r.L, $r.T, 0, 0, (New-Object System.Drawing.Size $w, $hgt))
$g2.Dispose()
$bmp2.Save("$Out-screen.png", [System.Drawing.Imaging.ImageFormat]::Png)
$bmp2.Dispose()
Write-Output "screen -> $Out-screen.png"
