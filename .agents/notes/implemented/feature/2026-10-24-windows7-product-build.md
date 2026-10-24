# Agent Note: Windows 7 完整原生产品构建与安装布局

Status: implemented

## Problem

兼容核心、原生设置和面板分别编译与测试后，仍不能从常规托盘入口启动它们；普通安装包依赖 WinUI、WebView2 和现代 VC 系统运行库，也只有 x64 Server。手工复制测试目录无法保证安装、升级与双架构应用的 TSF 注册正确。

## Decision

同仓增加明确的 Win7 产品构建配置，编译相同 Server、TSF 与共享业务，只替换设置和面板承载。`MSIME_WINDOWS7_CLIENT` 仅控制 Server 选择已打包的 `msime-client-settings-win7.exe` 与 `msime-client-panels-win7.exe`；默认关闭，现代产品继续原有文件名与行为。两种布局使用同一套路由、状态目录、配置文档与子进程环境协议，不能用目录残留文件猜测产品类型。

专用脚本复用已验证的同源 Win7 Rust 与语音产物，编译两个原生架构并收集实际 PE 依赖闭包。正式产品不携带测试 EXE 或 PDB；运行库从许可允许的 SDK/VC Redist 目录收集，不从开发机系统目录捞取。完整安装继续复用现有 Inno 生命周期、版本隔离 TSF 目录、数据所有权、升级保留配置与卸载策略。Win7 包显式声明 SP1 基线，原生窗口与随应用 CRT 无需 WebView2 或系统 VC 安装提示。构建与现代入口同样接受 `-ServerUiAccess`：未签名开发构建明确关闭，受信任签名安装包可明确开启，不因系统兼容而删除跨完整性级别能力。

### 包名与更新契约

两个架构分别输出 `MetasequoiaIME-<Edition>_Setup_v<版本>_win7_<x64|x86>.exe`，关于页与共享更新服务使用完全相同的实际包名。最初更新切片假定一个包承载双架构，优点是分发链接简单；实际产品的 x64／x86 依赖闭包与安装布局已分开，为避免一个用户下载两套完整语音运行库并扩大安装条件分支，更新检查改为携带真实架构，只选对应唯一资产。见[专用更新通道](2026-10-24-windows7-update-channel.md)。

### 真实系统入口验收

安装验收脚本只在全新、无账号状态的可丢弃系统上运行，使用构建机从共享版本表生成的 XML，不复制产品标识。检查安装、双架构 COM 激活、TSF 注册、任务、数据目录 ACL、升级保留合成数据、嵌套目录迁移与显式卸载清理。

`windows7-tsf-desktop --desktop` 仅由可见交互桌面显式调用，不能加入普通 CTest。它启动本架构系统目录下的独立记事本，只读写并关闭自己创建的窗口，以 `SendInput` 虚拟键输入合成拼音，要求已安装 Server 显示候选，再按空格并核对中文结果；不使用 Unicode 直输或直接写编辑框冒充 TSF 验收。激活和恢复遵循 [ITfInputProcessorProfileMgr::ActivateProfile](https://learn.microsoft.com/en-us/windows/win32/api/msctf/nf-msctf-itfinputprocessorprofilemgr-activateprofile) 的会话范围，不修改默认输入法。探针显式打开自己创建的编辑器的临时中文状态：产品默认 `default_ime_mode=english` 保持不变，不能把英文模式下透传拼音判作兼容故障。通过系统默认 IME 窗口的 `WM_IME_CONTROL/IMC_SETOPENSTATUS` 选择中文，仍必须由真实虚拟键得到候选并提交中文。

### 既有笔记审计

- [完整 Win7 支持](2026-10-23-windows7-support.md)：部分重叠，本篇只持有完整构建、启动入口与安装布局，完整功能验收由总笔记持有。
- [核心构建](../../implemented/feature/2026-10-23-windows7-rust-core-build.md)、[运行库闭包](../../implemented/feature/2026-10-24-windows7-crt-forwarding-audit.md)：部分重叠，产品构建调用现有工具，不另外实现核心编译或依赖规则。
- [原生设置基础](../../implemented/feature/2026-10-23-windows7-native-settings-foundation.md)：部分重叠，页面和存储仍归现有原生承载。

## Alternatives considered

- **把兼容面板改名成 `MSIME.exe` 并覆盖原文件。** 启动器可完全不改，但文件名掩盖承载差异，设置内跳转仍指向原生名称，而且暂存目录混用时难以发现误装。因此由构建配置明确指定，并逐项验证文件存在。
- **按操作系统版本在普通安装包中选择两套界面。** 用户只下载一个包，但两套 Rust 目标与语音运行库、WinUI/WebView2 依赖和双架构安装都混在一起，当前普通包回归面会扩大。先提供明确 Win7 构建入口，继续共享源码和安装生命周期。

## Verification

- 两种启动布局的路由、显式设置路径、相对路径拒绝和环境传播测试通过。
- 两个架构产品编译、依赖闭包审计与 Win7 实际装载通过。
- Win7 完整安装、升级保留配置、卸载、TSF 注册和真实 32/64 位应用输入通过。
- 现代 Windows Server／TSF 编译和相关原生回归通过；普通完整安装包未重新实装。

安装生命周期已在 Win7 SP1 x64 通过：安装、升级保留合成配置和用户文件、嵌套目录迁移、双架构 COM、TSF、看门狗、目录 ACL、可信签名及显式卸载。测试基线包含 KB4474419、KB4490628 和 KB2921916，不将其写成未更新的 SP1。最终双架构产品均经过依赖闭包审计和受信任测试签名；最终安装路径下的原生设置系统文件对话框打开／取消、x64 与 x86 系统记事本的真实拼音键、可见候选与“你好”提交均通过，Server 持续运行，英文默认值保持。

## Consequences

32 位 Win7 系统仍需独立系统环境，WOW64 进程通过不能替代它。签名信任和 Win7 的 SHA-2 补丁基线必须按真实安装结果记录，不能将未签名手工启动等同于正式分发验收。
