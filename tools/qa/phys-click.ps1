param([int]$X,[int]$Y,[int]$Repeat=1)
Add-Type -Namespace PH -Name M -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y); [DllImport("user32.dll")] public static extern void mouse_event(uint flags, int dx, int dy, uint data, System.UIntPtr extra);'
for($i=0;$i -lt $Repeat;$i++){
  [void][PH.M]::SetCursorPos($X, $Y)
  Start-Sleep -Milliseconds 90
  [PH.M]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 60
  [PH.M]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
  Start-Sleep -Milliseconds 220
}
"physical click at $X,$Y x$Repeat"
