param([long]$Hwnd,[int]$X,[int]$Y,[int]$Repeat=1,[int]$DelayMs=250)
Add-Type -Namespace PS -Name M -MemberDefinition '[DllImport("user32.dll")] public static extern IntPtr SendMessageW(IntPtr h, uint m, IntPtr w, IntPtr l); [DllImport("user32.dll")] public static extern bool PostMessageW(IntPtr h, uint m, IntPtr w, IntPtr l);'
$lp = [IntPtr](($Y -shl 16) -bor ($X -band 0xFFFF))
for($i=0;$i -lt $Repeat;$i++){
  [void][PS.M]::PostMessageW([IntPtr]$Hwnd, 0x0200, [IntPtr]0, $lp)
  Start-Sleep -Milliseconds 30
  [void][PS.M]::PostMessageW([IntPtr]$Hwnd, 0x0201, [IntPtr]1, $lp)
  Start-Sleep -Milliseconds 40
  [void][PS.M]::PostMessageW([IntPtr]$Hwnd, 0x0202, [IntPtr]0, $lp)
  Start-Sleep -Milliseconds $DelayMs
}
"posted $Repeat click(s) at $X,$Y to $Hwnd"
