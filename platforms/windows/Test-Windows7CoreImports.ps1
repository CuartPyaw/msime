[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$LiteralPath,
    [string]$DependencyDirectory = ''
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# 使用 MSVC 自带解析器同时列出普通与延迟导入，不执行待检查的 DLL。
$lines = @(& dumpbin /nologo /imports $LiteralPath)
if ($LASTEXITCODE -ne 0) { throw "Cannot read PE imports: $LiteralPath" }
$delayed = $false
$library = ''
$imports = [Collections.Generic.List[object]]::new()
$libraries = [Collections.Generic.List[object]]::new()
foreach ($line in $lines) {
    if ($line -match 'Section contains the following delay load imports:') {
        $delayed = $true
        $library = ''
    } elseif ($line -match '^    ([A-Za-z0-9_.-]+\.dll)\s*$') {
        $library = $Matches[1].ToLowerInvariant()
        $libraries.Add([pscustomobject]@{ Name = $library; Delayed = $delayed })
    } elseif ($library -ne '') {
        $pattern = if ($delayed) { '^\s+[0-9A-Fa-f]+\s+[0-9A-Fa-f]+\s+([A-Za-z_?][A-Za-z0-9_?@$]*)\s*$' }
                   else { '^\s+[0-9A-Fa-f]+\s+([A-Za-z_?][A-Za-z0-9_?@$]*)\s*$' }
        if ($line -match $pattern) {
            $imports.Add([pscustomobject]@{ Library = $library; Name = $Matches[1]; Delayed = $delayed; Ordinal = $null })
        } elseif ($line -match '^\s+Ordinal\s+([0-9]+)\s*$') {
            $ordinal = [int]$Matches[1]
            # Windows SDK 的 Winsock 导入库保留固定序号；只接受 Win7 已有的初始化与清理接口。
            $winsockOrdinals = @{ 115 = 'WSAStartup'; 116 = 'WSACleanup' }
            if ($library -eq 'ws2_32.dll' -and -not $delayed -and $winsockOrdinals.ContainsKey($ordinal)) {
                $imports.Add([pscustomobject]@{ Library = $library; Name = $winsockOrdinals[$ordinal]; Delayed = $false; Ordinal = $ordinal })
                continue
            }
            # sherpa 的上游导入库以序号链接这个 C ABI；必须核对实际相邻 DLL 的导出，不能略过。
            if ($library -ne 'onnxruntime.dll' -or $ordinal -ne 1 -or $delayed -or $DependencyDirectory -eq '') {
                throw "Unverified ordinal import: $library!$ordinal"
            }
            $dependency = Join-Path $DependencyDirectory $library
            if (-not (Test-Path -LiteralPath $dependency -PathType Leaf)) { throw "Missing ordinal dependency: $library" }
            $exports = @(& dumpbin /nologo /exports $dependency)
            if ($LASTEXITCODE -ne 0 -or -not ($exports -match '^\s+1\s+[0-9A-Fa-f]+\s+[0-9A-Fa-f]+\s+OrtGetApiBase\s*$')) {
                throw "Unexpected ONNX ordinal 1 export: $dependency"
            }
            $imports.Add([pscustomobject]@{ Library = $library; Name = 'OrtGetApiBase'; Delayed = $false; Ordinal = $ordinal })
        }
    }
}
if ($imports.Count -eq 0) { throw "PE has no recognizable imports: $LiteralPath" }
foreach ($module in $libraries) {
    if (@($imports | Where-Object { $_.Library -eq $module.Name -and $_.Delayed -eq $module.Delayed }).Count -eq 0) {
        throw "PE library has no recognizable imports: $($module.Name)"
    }
}

$allowedDelayed = @{
    'combase.dll' = @('CoTaskMemFree', 'CoCreateFreeThreadedMarshaler')
    'api-ms-win-core-winrt-error-l1-1-0.dll' = @('RoOriginateErrorW')
    'mmdevapi.dll' = @('ActivateAudioInterfaceAsync')
}
$unsupported = @(
    'GetDpiForWindow', 'GetDpiForSystem', 'SetThreadDpiAwarenessContext',
    'SetProcessDpiAwarenessContext', 'GetWindowDpiAwarenessContext', 'GetAwarenessFromDpiAwarenessContext',
    'GetThreadDpiAwarenessContext', 'AreDpiAwarenessContextsEqual',
    'GetSystemTimePreciseAsFileTime', 'WaitOnAddress', 'WakeByAddressSingle', 'WakeByAddressAll',
    'GetTempPath2W', 'GetTempPath2A', 'CreateFile2', 'SetThreadDescription', 'GetThreadDescription',
    'ActivateAudioInterfaceAsync', 'RoOriginateErrorW', 'GetProcessMitigationPolicy',
    'PathCchRemoveFileSpec', 'PathCchRemoveBackslash'
)
foreach ($entry in $imports) {
    if ($entry.Delayed) {
        if (-not $allowedDelayed.ContainsKey($entry.Library) -or $entry.Name -notin $allowedDelayed[$entry.Library]) {
            throw "Win7 hook does not cover delayed import: $($entry.Library)!$($entry.Name)"
        }
    } elseif ($entry.Library -in @('combase.dll', 'dcomp.dll', 'shcore.dll', 'api-ms-win-core-path-l1-1-0.dll') -or
              $entry.Library.StartsWith('api-ms-win-core-winrt-') -or $entry.Name -in $unsupported) {
        throw "Win7 core has an incompatible static import: $($entry.Library)!$($entry.Name)"
    }
}

# 此清单核对已发现的装载障碍；系统全部导出和第三方 DLL 仍需在 Win7 验收。
[pscustomobject]@{
    File = [IO.Path]::GetFileName($LiteralPath)
    Imports = @($imports.ToArray() | Sort-Object Library, Name, Delayed -Unique)
}
