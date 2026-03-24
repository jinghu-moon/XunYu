param(
    [string]$DllPath = (Join-Path (Resolve-Path (Join-Path $PSScriptRoot '..')) 'build/xunbak-7z-plugin/Debug/xunbak.dll')
)

$ErrorActionPreference = 'Stop'

if (!(Test-Path $DllPath)) {
    throw "DLL not found: $DllPath"
}

Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public static class NativeMethods {
    [DllImport("kernel32.dll", CharSet = CharSet.Unicode, SetLastError = true)]
    public static extern IntPtr LoadLibraryW(string lpFileName);

    [DllImport("kernel32.dll", CharSet = CharSet.Ansi, SetLastError = true)]
    public static extern IntPtr GetProcAddress(IntPtr hModule, string lpProcName);

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern bool FreeLibrary(IntPtr hModule);
}
"@

$module = [NativeMethods]::LoadLibraryW($DllPath)
if ($module -eq [IntPtr]::Zero) {
    throw "LoadLibrary failed: $DllPath"
}

try {
    $exports = @(
        'CreateObject',
        'GetNumberOfFormats',
        'GetHandlerProperty2',
        'GetHandlerProperty',
        'GetIsArc'
    )

    foreach ($name in $exports) {
        $ptr = [NativeMethods]::GetProcAddress($module, $name)
        if ($ptr -eq [IntPtr]::Zero) {
            throw "Missing export: $name"
        }
        Write-Host "$name => $ptr"
    }
}
finally {
    [void][NativeMethods]::FreeLibrary($module)
}

Write-Host "DLL smoke check passed: $DllPath"
