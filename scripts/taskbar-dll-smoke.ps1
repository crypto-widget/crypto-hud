param(
    [Parameter(Mandatory = $true)]
    [string]$DllPath
)

$ErrorActionPreference = "Stop"
$resolvedPath = (Resolve-Path -LiteralPath $DllPath -ErrorAction Stop).Path
if (-not (Test-Path -LiteralPath $resolvedPath -PathType Leaf)) {
    throw "Taskbar extension DLL does not exist: $resolvedPath"
}

$bytes = [System.IO.File]::ReadAllBytes($resolvedPath)
if ($bytes.Length -lt 0x40 -or $bytes[0] -ne 0x4D -or $bytes[1] -ne 0x5A) {
    throw "Taskbar extension is not a valid DOS/PE image: $resolvedPath"
}
$peOffset = [BitConverter]::ToInt32($bytes, 0x3C)
if ($peOffset -lt 0 -or $peOffset + 6 -gt $bytes.Length -or
    $bytes[$peOffset] -ne 0x50 -or $bytes[$peOffset + 1] -ne 0x45 -or
    $bytes[$peOffset + 2] -ne 0 -or $bytes[$peOffset + 3] -ne 0) {
    throw "Taskbar extension has an invalid PE header: $resolvedPath"
}
$machine = [BitConverter]::ToUInt16($bytes, $peOffset + 4)
if ($machine -ne 0x8664) {
    throw ("Taskbar extension must be x64 (machine 0x8664), found 0x{0:X4}" -f $machine)
}

$imageText = [Text.Encoding]::ASCII.GetString($bytes)
foreach ($runtimeDependency in @("MSVCP140.dll", "VCRUNTIME140.dll", "VCRUNTIME140_1.dll")) {
    if ($imageText.IndexOf($runtimeDependency, [StringComparison]::OrdinalIgnoreCase) -ge 0) {
        throw "Taskbar extension must use the static MSVC runtime; found dependency $runtimeDependency"
    }
}

if (-not ("CryptoHudTaskbarDllSmokeNative" -as [type])) {
    Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public static class CryptoHudTaskbarDllSmokeNative
{
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr LoadLibraryExW(string fileName, IntPtr file, uint flags);

    [DllImport("kernel32.dll", CharSet = CharSet.Ansi, SetLastError = true)]
    public static extern IntPtr GetProcAddress(IntPtr module, string procedureName);

    [DllImport("kernel32.dll", SetLastError = true)]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool FreeLibrary(IntPtr module);
}
"@
}

$loadLibrarySearchDllLoadDir = 0x00000100
$loadLibrarySearchSystem32 = 0x00000800
$module = [CryptoHudTaskbarDllSmokeNative]::LoadLibraryExW(
    $resolvedPath,
    [IntPtr]::Zero,
    $loadLibrarySearchDllLoadDir -bor $loadLibrarySearchSystem32
)
if ($module -eq [IntPtr]::Zero) {
    $errorCode = [Runtime.InteropServices.Marshal]::GetLastWin32Error()
    throw "Taskbar extension failed to load (Win32 error $errorCode): $resolvedPath"
}

try {
    foreach ($export in @("DllGetClassObject", "DllCanUnloadNow", "CryptoHudTaskbarHook")) {
        if ([CryptoHudTaskbarDllSmokeNative]::GetProcAddress($module, $export) -eq [IntPtr]::Zero) {
            throw "Taskbar extension is missing required export: $export"
        }
    }
} finally {
    [void][CryptoHudTaskbarDllSmokeNative]::FreeLibrary($module)
}

Write-Host "Taskbar extension DLL smoke passed: $resolvedPath"
