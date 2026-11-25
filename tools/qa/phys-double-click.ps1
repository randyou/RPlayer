param([int]$X, [int]$Y, [int]$GapMs = 90)
Add-Type -Namespace PD -Name M -MemberDefinition '[DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y); [DllImport("user32.dll")] public static extern void mouse_event(uint f, int dx, int dy, uint d, System.UIntPtr e);'
[void][PD.M]::SetCursorPos($X, $Y)
Start-Sleep -Milliseconds 120
for ($i = 0; $i -lt 2; $i++) {
    [PD.M]::mouse_event(0x0002, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 40
    [PD.M]::mouse_event(0x0004, 0, 0, 0, [UIntPtr]::Zero)
    if ($i -eq 0) { Start-Sleep -Milliseconds $GapMs }
}
"double-click at $X,$Y"
