[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# 模拟 dumpbin 的文本边界，验证固定序号可解析，未知序号与新系统静态接口仍会被拒绝。
$audit = Join-Path $PSScriptRoot '../../Test-Windows7CoreImports.ps1'
function Check-Import {
    param([string]$Library, [string]$Import, [string]$Expected = '', [switch]$Delayed)
    $importLines = @(
        $(if ($Delayed) { 'Section contains the following delay load imports:' } else { 'Section contains the following imports:' }),
        ('    ' + $Library), $Import, '  Summary', '        1000 _RDATA'
    )
    Set-Item function:dumpbin -Value {
        $global:LASTEXITCODE = 0
        return $importLines
    }.GetNewClosure()
    if ($Expected -eq '') {
        $rejected = $false
        try { & $audit -LiteralPath 'synthetic.dll' | Out-Null } catch { $rejected = $true }
        if (-not $rejected) { throw "Unexpectedly accepted: $Library $Import" }
    } else {
        $result = & $audit -LiteralPath 'synthetic.dll'
        if ($result.Imports.Count -ne 1 -or $result.Imports[0].Name -cne $Expected) {
            throw "Unexpected import mapping: $Library $Import"
        }
    }
}
Check-Import 'WS2_32.dll' '                             Ordinal   115' 'WSAStartup'
Check-Import 'ws2_32.dll' '                             Ordinal   116' 'WSACleanup'
Check-Import 'WS2_32.dll' '                             Ordinal   117'
Check-Import 'unverified.dll' '                             Ordinal   115'
Check-Import 'WS2_32.dll' '                             Ordinal   115' -Delayed
Check-Import 'kernel32.dll' '                            A6 GetDpiForWindow'
Check-Import 'dxgi.dll' '                             5 CreateDXGIFactory2'
Check-Import 'kernel32.dll' '                            A6 GetTickCount64' 'GetTickCount64'
Check-Import 'onnxruntime.dll' '                             Ordinal   1'
Write-Output 'Windows 7 导入检查：固定 Winsock 序号、未知序号与系统接口边界通过。'
