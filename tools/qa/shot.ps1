param([long]$Hwnd,[string]$Out="D:\Workspace\_shot")
Add-Type -AssemblyName System.Drawing
Add-Type -Namespace S -Name W -MemberDefinition '[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r); [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags); [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h); public struct RECT { public int Left; public int Top; public int Right; public int Bottom; }'
$h = [IntPtr]$Hwnd
$r = New-Object S.W+RECT
[void][S.W]::GetWindowRect($h, [ref]$r)
$w = $r.Right - $r.Left; $ht = $r.Bottom - $r.Top
"hwnd=$Hwnd rect=$($r.Left),$($r.Top),$($r.Right),$($r.Bottom) visible=$([S.W]::IsWindowVisible($h)) size=${w}x${ht}"
$bmp = New-Object System.Drawing.Bitmap $w, $ht
$g = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $g.GetHdc()
$ok = [S.W]::PrintWindow($h, $hdc, 2)
$g.ReleaseHdc($hdc); $g.Dispose()
$bmp.Save("$Out.png", [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
"printwindow=$ok -> $Out.png"
