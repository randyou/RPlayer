# Register a Start Menu shortcut carrying System.AppUserModel.ID so Windows
# media controls can show "RPlayer" instead of "Unknown app".
param(
    [string]$Target = (Join-Path $PSScriptRoot '..\RPlayer.exe'),
    [string]$AppId = 'RPlayer',
    [string]$DisplayName = 'RPlayer'
)

$ErrorActionPreference = 'Stop'
$target = (Resolve-Path $Target).Path
$startMenu = [Environment]::GetFolderPath('Programs')
$linkPath = Join-Path $startMenu "$DisplayName.lnk"

$wsh = New-Object -ComObject WScript.Shell
$shortcut = $wsh.CreateShortcut($linkPath)
$shortcut.TargetPath = $target
$shortcut.WorkingDirectory = Split-Path $target
$shortcut.IconLocation = "$target,0"
$shortcut.Description = 'RPlayer 本地音乐播放器'
$shortcut.Save()

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class LnkAumid
{
    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    public struct PROPERTYKEY { public Guid fmtid; public uint pid; }

    [StructLayout(LayoutKind.Explicit)]
    public struct PROPVARIANT
    {
        [FieldOffset(0)] public ushort vt;
        [FieldOffset(2)] public ushort r1;
        [FieldOffset(4)] public ushort r2;
        [FieldOffset(6)] public ushort r3;
        [FieldOffset(8)] public IntPtr p;
        [FieldOffset(16)] public IntPtr p2;
    }

    [ComImport]
    [Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99")]
    [InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    public interface IPropertyStore
    {
        uint GetCount();
        void GetAt(uint index, out PROPERTYKEY key);
        void GetValue(ref PROPERTYKEY key, out PROPVARIANT value);
        void SetValue(ref PROPERTYKEY key, ref PROPVARIANT value);
        void Commit();
    }

    [DllImport("shell32.dll", CharSet = CharSet.Unicode, PreserveSig = false)]
    private static extern void SHGetPropertyStoreFromParsingName(
        string path, IntPtr bindCtx, uint flags, ref Guid iid, out IPropertyStore store);

    [DllImport("ole32.dll")]
    private static extern int PropVariantClear(ref PROPVARIANT pv);

    public static void Set(string path, string aumid)
    {
        Guid iid = new Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99");
        IPropertyStore store;
        SHGetPropertyStoreFromParsingName(path, IntPtr.Zero, 2, ref iid, out store);
        PROPERTYKEY key = new PROPERTYKEY
        {
            fmtid = new Guid("9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3"),
            pid = 5
        };
        PROPVARIANT value = new PROPVARIANT();
        value.vt = 31; // VT_LPWSTR
        value.p = Marshal.StringToCoTaskMemUni(aumid);
        store.SetValue(ref key, ref value);
        store.Commit();
        PropVariantClear(ref value);
        Marshal.ReleaseComObject(store);
    }
}
'@

[LnkAumid]::Set($linkPath, $AppId)
Write-Output "Created $linkPath with AUMID $AppId"
