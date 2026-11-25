Add-Type -Namespace MH -Name W -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr p);
public delegate bool EnumProc(IntPtr h, IntPtr p);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern int GetWindowTextW(IntPtr h, System.Text.StringBuilder s, int n);
public struct RECT { public int L, T, R, B; }
'@
$proc = Get-Process rplayer -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $proc) { "0 0 0 0 0"; exit }
$tp = [uint32]$proc.Id
$script:best = [IntPtr]::Zero
$script:area = 0
[MH.W]::EnumWindows({
    param($h, $x)
    $q = 0
    [void][MH.W]::GetWindowThreadProcessId($h, [ref]$q)
    if ($q -eq $tp) {
        [MH.W+RECT]$r = New-Object MH.W+RECT
        [void][MH.W]::GetWindowRect($h, [ref]$r)
        $area = ($r.R - $r.L) * ($r.B - $r.T)
        if ($area -gt $script:area) { $script:area = $area; $script:best = $h }
    }
    return $true
}, [IntPtr]::Zero) | Out-Null
[MH.W+RECT]$r = New-Object MH.W+RECT
[void][MH.W]::GetWindowRect($script:best, [ref]$r)
"$($script:best) $($r.L) $($r.T) $($r.R) $($r.B)"
