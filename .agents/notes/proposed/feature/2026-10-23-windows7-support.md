# Agent Note: 在同一源码中提供完整 Windows 7 兼容构建

Status: proposed

## Problem

用户要求完整支持 Windows 7，并且不影响现有功能。当前原生宿主直接导入 Windows 10 的 DPI API，`msimeui` 静态导入 Windows 8 起才有的 `dcomp.dll`；这些依赖会在运行时回退执行前阻止程序加载。候选绘制已经存在 Direct2D 1.0 与分层窗口回退，但尚不足以证明 Win7 可用。原生设置使用 Windows App SDK 1.7，Rust 发布脚本使用普通 Windows MSVC 目标，二者也没有 Win7 运行保证。

## Proposal

保留同一引擎、配置、业务逻辑和 TSF/Server 协议，增加可重复的 Win7 构建与运行验证。现代 Windows 保持现有功能、默认值和设置入口；兼容适配按系统实际能力选择，不能靠隐藏功能或跳过验证宣称完成。

实施顺序是先消除装载阶段的不兼容，再适配完整 UI 与依赖，最后进行安装和真实输入法验收。最先落地的 DPI 与 DirectComposition 适配属于基础切片，不能单独作为 Win7 支持交付。

### 系统接口与绘制

新 DPI API 统一按系统导出探测：存在时原参数、原返回值透传；缺少时窗口 DPI 使用 GDI 查询，进程采用系统 DPI 感知，线程级上下文保持原状。禁止在缺少线程 API 时反复修改进程 DPI。无效窗口仍返回查询失败，不伪造成功的窗口 DPI。DirectComposition 动态装载失败时使用已有分层窗口；现代合成器仍走原绘制路径。

### 构建、设置与完整能力

Win7 Rust 构建需要独立的 `*-win7-windows-msvc` 目标、标准库和依赖导入审计。现有 Win10/11 的工具链与发布行为保持原状。Win7 设置需要原生兼容承载，覆盖当前设置页面与所有可达操作；WinUI 3 不能作为 Win7 设置入口。共享面板和语音运行库必须逐项验证，任何未适配能力都属于尚未完成，而非默认移除。

导入审计只解析有明确含义的序号：Windows SDK 的 Winsock `115/116` 分别映射到 `WSAStartup/WSACleanup`；sherpa 的 ONNX Runtime 序号 `1` 还必须与相邻 DLL 的 `OrtGetApiBase` 导出核对。未知库、未知序号与未经兼容钩子覆盖的延迟序号继续失败，不能整体放开序号导入。核心构建在编译前检查 MSVC 的 `dumpbin` 是否可用。

兼容设置采用 Win32 标准控件，复用 `settings/` 中不依赖 WinRT 的页面模型和同一个 Host API，不引入另一套业务规则或偏好默认值。读写在串行工作线程执行，按已读取的修订号比较并交换；保存失败后重新读取实际状态，不自动覆盖并发更新。开发中的兼容目标单独构建，全部页面和操作完成前不改普通设置启动器或安装包。先接通快捷键页的真实控件、持久化和冲突处理，再逐页补齐完整范围。

首轮 Rust Win7 目标构建成功，但实际 DLL 仍导入 `combase.dll` 的两个旧 COM 函数、WinRT 错误 API，以及 `mmdevapi.dll` 的 `ActivateAudioInterfaceAsync`。后者在 Win7 不存在，新版 cpal 的默认麦克风和扬声器也会使用它。Win7 核心构建使用 MSVC 延迟加载及专用通知钩子处理这三个已确认的依赖；原有现代构建不加此钩子。系统模块只从系统目录加载，存在的函数仍调用原地址；旧 COM 函数使用 `ole32.dll` 的同一函数，WinRT 错误消息使用传统 `IErrorInfo` 保存。默认录音和播放的虚拟 GUID 用 Win7 的 `IMMDeviceEnumerator` 与 `IMMDevice::Activate` 激活，由 MTA 工作线程按原接口回调完成结果；不能仅返回“不支持”而使现有语音和音效失效。不为未知导入伪造成功或通用空实现。该切片仍需普通麦克风、语音播放和 Win7 实机验证。

### 本地识别运行库

锁定的 sherpa-onnx 1.13.8 Windows x64/x86 包已下载并通过现有 SHA-256 校验。实际 ONNX Runtime 版本为 1.28.2，静态导入 `CreateFile2`、`GetProcessMitigationPolicy`、`GetSystemTimePreciseAsFileTime` 与 Windows 8 的路径接口；当前包不能作为 Win7 运行库。普通 Windows 的锁文件不改。兼容运行库从 ONNX Runtime 同版本固定提交 `33ca9628233dc8f002435e868d4c2e9f82766ca1` 构建，针对已发现的系统接口使用真实旧 API 或能力探测，保持模型格式与运算支持，不靠整体退到旧 ONNX Runtime 避开问题。构建源码与产物仍在隔离的 `target/tooling` 和 `target/windows7-voice/<arch>` 内，最终导入和实际模型推理必须重新验证。

[语音运行库的独立构建](../../implemented/feature/2026-10-23-windows7-voice-runtime.md)已落地 x64 与 x86 的同版本源码构建、导入检查、真实合成 Add 与优化卷积推理、sherpa C ABI 与相邻依赖验证；Win7 实机仍需继续验证。该切片不代表整机语音能力已经验收。

[原生手写切片](../../implemented/feature/2026-10-23-windows7-handwriting.md)接通 Tablet PC COM、Windows 专用 C ABI 与独立 Win32 画布；x64/x86 构建与实际系统识别测试已执行，模型状态、下载取消、离线导入与移除经共享资源包接口接通，并已在 Windows 11 上验证真实下载安装；候选窗口设置页补充了原生 owner-draw 固定样例预览；用户交互桌面上屏、Win7 SP1 组件与安装集成仍未完成。

### 既有笔记审计

- [原生兼容设置基础](../../implemented/feature/2026-10-23-windows7-native-settings-foundation.md)：部分落地，快捷键页、工具栏设置控件与存储边界已有真实测试，完整页面和操作仍需继续适配。
- [设置界面归属](../../implemented/architecture/2026-10-15-settings-ui-ownership-by-platform.md)：部分重叠。Windows 的原生设置归属保持，新增兼容承载不改变其他平台的选择；Win10/11 继续使用 WinUI 3。
- [候选渲染边界](../../implemented/architecture/2026-10-13-cross-platform-candidate-rendering-boundaries.md)：部分重叠。候选继续归 Server，TSF DLL 不承载绘制，兼容处理只在宿主绘制层。
- 按 `win7`、`windows 7` 检索 `proposed`、`implemented`、`rejected`，没有既有 Win7 支持方案；无需替换或归档旧决定。

## Alternatives considered

- **单独 fork 长期维护。** 它可以独立冻结旧系统依赖，主线不需要处理兼容逻辑。但引擎、配置与业务修复需要长期双向移植，用户已经选择同仓支持，因此使用独立构建配置与明确适配模块。
- **全局降级工具链与界面。** 一份低版本实现能够避免两个 UI 承载，但会扩大对现有 Windows 用户及其功能的影响，违背本任务的回归约束。
- **冻结旧 WebView2 承载全部界面。** 复用共享 React 页能减少界面重写，但 Win7 最后支持的 WebView2 109 已停止维护，新 SDK 和前端能力也没有兼容保证，不能作为未经验证的完整解决方案。

## Acceptance criteria

- Win7 SP1 上安装、启动、设置、候选、正常上屏及 32/64 位应用输入均实际通过。
- 当前 Windows 设置及共享面板中的能力均有可用入口和行为验证，语音、账号、词库、插件等逐项记录结果。
- Win7 包内全部 PE 文件的直接与延迟导入经过兼容审计，不能仅检查主程序。
- Win10/11 的编译、原生测试、安装升级和现有功能回归通过；其他平台共享层改动运行对应门禁。
- 本地 quick、笔记校验及差异检查通过；明确区分交叉编译、运行测试与真实输入验收。

## Risks

Rust Win7 目标属于 Tier 3，必须维护可复现标准库构建并审计第三方依赖。现有 WinUI 设置与面板需要完整兼容实现，不能仅补启动器。Win7 测试环境和签名补丁基线必须实际可用。本机磁盘首次检查仅剩 12 GB，不在本机新增完整构建树；远程 Windows 构建使用隔离目录，不能复用其他会话的未提交源码或新旧协议产物。
