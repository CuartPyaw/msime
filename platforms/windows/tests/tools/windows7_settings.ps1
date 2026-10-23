[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Program,
    [ValidateSet('full','pinyin','wubi','japanese','vietnamese','tibetan')][string]$Edition = 'full'
)

# 在交互桌面启动原生宿主，只检查本测试进程的系统对话框；所有状态均使用独立临时目录。
$ErrorActionPreference = 'Stop'
Add-Type -TypeDefinition @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class Win7SettingsSmoke {
    public delegate bool Visitor(IntPtr window, IntPtr data);
    [DllImport("user32.dll")] public static extern bool EnumWindows(Visitor visitor, IntPtr data);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
    [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetWindowText(IntPtr window, StringBuilder value, int size);
    [DllImport("user32.dll",CharSet=CharSet.Unicode)] public static extern int GetClassName(IntPtr window, StringBuilder value, int size);
    [DllImport("user32.dll")] public static extern IntPtr GetDlgItem(IntPtr window, int id);
    [DllImport("user32.dll")] public static extern IntPtr SendMessage(IntPtr window, uint message, IntPtr wparam, IntPtr lparam);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr window, uint message, IntPtr wparam, IntPtr lparam);
    [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] public static extern bool IsWindowEnabled(IntPtr window);
    [DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr window, int show);
    public static string Classes(uint process) {
        var classes = new System.Collections.Generic.List<string>();
        EnumWindows((window, data) => {
            uint owner; GetWindowThreadProcessId(window, out owner);
            if (owner == process) { var type = new StringBuilder(256); GetClassName(window, type, type.Capacity); classes.Add(type.ToString()); }
            return true;
        }, IntPtr.Zero);
        return string.Join(",", classes);
    }
    public static IntPtr Find(uint process, string className, string title) {
        IntPtr result = IntPtr.Zero;
        EnumWindows((window, data) => {
            uint owner; GetWindowThreadProcessId(window, out owner);
            if (owner != process || (className == "#32770" && !IsWindowVisible(window))) return true;
            var type = new StringBuilder(256); GetClassName(window, type, type.Capacity);
            if (type.ToString() != className) return true;
            var name = new StringBuilder(256); GetWindowText(window, name, name.Capacity);
            if (!string.IsNullOrEmpty(title) && name.ToString() != title) return true;
            result = window; return false;
        }, IntPtr.Zero);
        return result;
    }
}
'@
$directory = Join-Path $env:TEMP ('msime-win7-settings-smoke-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $directory | Out-Null
$previousDirectory = $env:MSIME_CLIENT_STATE_DIR
$program = (Resolve-Path -LiteralPath $Program).Path
if (-not (Test-Path -LiteralPath $program -PathType Leaf)) { throw 'Native settings program does not exist' }
$windowClass = 'MSIME.Windows7Settings.' + $Edition
function StateSnapshot {
    return (@(Get-ChildItem -LiteralPath $directory -Filter '*.json' -Recurse -File |
        Sort-Object FullName | ForEach-Object { $_.FullName + ':' + (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash }) -join "`n")
}
$app = $null
try {
    $env:MSIME_CLIENT_STATE_DIR = $directory
    $app = Start-Process -FilePath $program -WorkingDirectory $directory -WindowStyle Normal -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(15)
    do {
        $hostWindow = [Win7SettingsSmoke]::Find($app.Id, $windowClass, $null)
        if ($hostWindow -ne [IntPtr]::Zero) { break }
        if ([DateTime]::UtcNow -gt $deadline) { throw ('Settings host did not appear; exited=' + $app.HasExited + '; session=' + $app.SessionId + '; own classes=' + [Win7SettingsSmoke]::Classes($app.Id)) }
        Start-Sleep -Milliseconds 30
    } while ($true)
    [Win7SettingsSmoke]::ShowWindowAsync($hostWindow, 1) | Out-Null
    $body = [Win7SettingsSmoke]::GetDlgItem($hostWindow, 99)
    do {
        if ([Win7SettingsSmoke]::IsWindowEnabled([Win7SettingsSmoke]::GetDlgItem($body, 100))) { break }
        if ([DateTime]::UtcNow -gt $deadline) { throw 'Settings did not load' }
        Start-Sleep -Milliseconds 30
    } while ($true)
    $before = StateSnapshot
    $selector = [Win7SettingsSmoke]::GetDlgItem($hostWindow, 109)
    [Win7SettingsSmoke]::SendMessage($selector, 0x014e, [IntPtr]2, [IntPtr]::Zero) | Out-Null
    [Win7SettingsSmoke]::SendMessage($hostWindow, 0x0111, [IntPtr](109 -bor (1 -shl 16)), $selector) | Out-Null
    foreach ($entry in @(@(305, '导出设置'), @(306, '导入设置'))) {
        $button = [Win7SettingsSmoke]::GetDlgItem($body, $entry[0])
        if (-not [Win7SettingsSmoke]::IsWindowEnabled($button)) { throw 'Settings file action is disabled' }
        [Win7SettingsSmoke]::PostMessage($body, 0x0111, [IntPtr]$entry[0], $button) | Out-Null
        $deadline = [DateTime]::UtcNow.AddSeconds(15)
        do {
            $dialog = [Win7SettingsSmoke]::Find($app.Id, '#32770', $entry[1])
            if ($dialog -ne [IntPtr]::Zero) { break }
            if ([DateTime]::UtcNow -gt $deadline) { throw 'System file dialog did not appear' }
            Start-Sleep -Milliseconds 30
        } while ($true)
        [Win7SettingsSmoke]::PostMessage($dialog, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
        do {
            if ([Win7SettingsSmoke]::IsWindowEnabled($hostWindow)) { break }
            if ([DateTime]::UtcNow -gt $deadline) { throw 'Settings host did not recover after cancellation' }
            Start-Sleep -Milliseconds 30
        } while ($true)
    }
    if ((StateSnapshot) -cne $before) { throw 'Cancellation unexpectedly changed settings files' }
    [Win7SettingsSmoke]::PostMessage($hostWindow, 0x0010, [IntPtr]::Zero, [IntPtr]::Zero) | Out-Null
    if (-not $app.WaitForExit(10000) -or $app.ExitCode -ne 0) { throw 'Settings host did not close cleanly' }
    Write-Output ('Windows 7 settings desktop smoke passed: both native system file dialogs opened and cancelled; settings unchanged; edition=' + $Edition)
} finally {
    if ($app -and -not $app.HasExited) { $app.Kill(); $app.WaitForExit() }
    $env:MSIME_CLIENT_STATE_DIR = $previousDirectory
    Remove-Item -LiteralPath $directory -Recurse -Force
}
