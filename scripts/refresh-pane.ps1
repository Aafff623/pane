Add-Type -AssemblyName System.Windows.Forms
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public class Win32 {
    [DllImport("user32.dll")] public static extern int GetWindowText(IntPtr h, StringBuilder sb, int count);
    [DllImport("user32.dll")] public static extern int GetWindowTextLength(IntPtr h);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
    [DllImport("user32.dll")] public static extern IntPtr FindWindow(string lpClass, string lpWindowName);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr h, uint Msg, IntPtr w, IntPtr l);
    public const uint WM_LBUTTONDOWN = 0x0201;
    public const uint WM_LBUTTONUP = 0x0202;
}
"@

# Try to find Pane tray window - it may not have a traditional title
$p = Get-Process pane -ErrorAction SilentlyContinue
if ($p -eq $null) { Write-Host "Pane not running"; exit }

$mainHwnd = $p.MainWindowHandle
Write-Host "Pane main HWND: $mainHwnd"

if ($mainHwnd -ne 0) {
    $len = [Win32]::GetWindowTextLength($mainHwnd)
    $sb = New-Object System.Text.StringBuilder($len + 1)
    [void][Win32]::GetWindowText($mainHwnd, $sb, $sb.Capacity)
    $title = $sb.ToString()
    Write-Host "Title: $title"
    Write-Host "Visible: $([Win32]::IsWindowVisible($mainHwnd))"
}

# Try to activate Pane window
if ($mainHwnd -ne 0) {
    [void][Win32]::SetForegroundWindow($mainHwnd)
    Start-Sleep -Milliseconds 200
    # Send F5 to refresh
    [void][Win32]::SendMessage($mainHwnd, 0x0102, [IntPtr]0x74, [IntPtr]0x00150001)  # F5
    Write-Host "Refresh signal sent"
}

Write-Host "Done"
