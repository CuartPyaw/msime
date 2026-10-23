# Agent Note: Windows 7 手写识别使用系统 Tablet PC COM 接口

Status: implemented

## Problem

`host-windows::ink` 目前只使用 WinRT `InkRecognizerContainer`，Win7 没有这个运行时。即使核心 DLL 能装载，手写仍不能调用系统已安装的中文识别器；把缺少 WinRT 当作没有中文语言包也不能满足完整支持。

## Decision

仅 `target_vendor = "win7"` 的 Windows 构建使用 `IInkRecognizers`、`IInkDisp` 和 `IInkRecognizerContext`。普通 Windows 继续使用现有 WinRT 路径。两个后端复用中文识别器排序、输入上限、中文候选优先和去重规则，不改变偏好或引擎模型回退策略。

每次操作使用独立 STA 线程，所有 COM 接口在该线程创建和释放，最后平衡 `CoUninitialize`；不改变调用者的 apartment。点坐标转换为固定 HIMETRIC 尺度，拒绝非有限值造成的无效笔划，限制笔划与点数，确保两轴跨度不超过 COM 的有符号 32 位范围。`SAFEARRAY` 的创建、锁定和释放具有明确所有权，错误路径同样回收。

识别同步运行在该工作线程，候选最多保留共享面板允许的数量。没有中文识别器时保留 `NoChineseRecognizer`，没有有效笔划时返回 `EmptyInput`，COM 或识别错误返回 `Unavailable`；不把拉丁语识别器冒充中文。

原生面板通过 Windows 专用 C ABI `msime_client_windows_handwriting` 查询识别器与识别笔划，采用共享 `HandwritingRecognitionRequest` 的输入校验与候选契约，系统识别没有结果时保留可信模型回退。接口不依赖 Tauri，按安装包 edition 拒绝未提供手写的版本，不访问会话、不联网。界面、候选上屏与包内模型定位继续由原生面板承载。

`panels/win7` 的独立 Win32 窗口提供 420 DIP 画布、撤销、清空和至多十二项候选。识别与模型查询由串行工作线程处理，新请求替换尚未开始的请求；结果带画布代次，清空、继续书写和撤销后不接受旧结果。关闭先等待线程退出，再销毁窗口，不把堆指针放进 Windows 消息。窗口使用 `WS_EX_NOACTIVATE`，沿用 `NativeTextInputTarget` 的原编辑器校验，只在它仍为前台窗口时注入候选，切换到其他应用时明确拒绝。模型按运行时选项、环境变量、随包路径、共享资源包发布状态查找，不自行认定未发布模型可用。全部面板完成前不修改普通启动器和安装包。

成功注入的候选带共享的面板 `SendInput` 标记，避免 TSF 的直通统计重复记数，再由工作线程调用现有聚合统计接口按手写来源记录；关闭时完成已受理统计。文字只在内存里用于分类，不写诊断或统计正文，失败不重复上屏。

### 既有笔记审计

- [完整 Win7 支持](../../proposed/feature/2026-10-23-windows7-support.md)：部分落地的运行能力切片，原生手写面板、安装和 Win7 实机验收仍在完整范围内。
- [Windows 剪贴板设置](../../implemented/feature/2026-10-11-windows-settings-clipboard.md)：其中提到共享面板，决定归属不同；不修改剪贴板或面板入口。
- 按 TabletPC、Windows Ink 和 InkRecognizer 检索活跃笔记，没有同一兼容识别后端的既有决定需要替换。

## Alternatives considered

- **仅下载 Engine 手写模型。** 可以复用跨平台模型路径，但丢失系统中文识别器已有的能力，并额外依赖下载。COM 接口直接使用系统语言包，模型仍是缺少系统识别能力时的既有回退。
- **让所有 Windows 都改用 Tablet PC COM。** 可以只维护一个后端，但会改变现代系统上既有识别行为。兼容实现只进入 Win7 专用目标；普通目标的测试额外编译它以覆盖 COM 和内存边界。

## Testing

在 Windows 11 上，Win7 x64、Win7 x86 和普通 Windows 目标的 `cargo test -p msime-host-windows --lib ink::` 各九项通过；合成十字笔划经实际已安装的中文识别器得到「十」，普通目标的公共入口仍实际调用原 WinRT 后端。Win7 x64/x86 的 `cargo test -p msime-host-api --lib windows_handwriting::` 各三项通过，包含真实 C ABI 识别、缓冲区边界、非法字段与模型路径拒绝。对应后端 clippy 与 Win7 x64 Host API clippy 通过；现有 `read_clipboard_text` 的 `Result<_, ()>` 使用显式 `-A clippy::result_unit_err`，其他警告仍为错误。

x64 与 x86 原生窗口使用同源 Win7 核心构建并通过 CTest：实际窗口鼠标消息调用真实识别器，验证撤销、清空、过时代次、提交回调拒绝后保留候选、成功后清空和只记录一次手写来源统计，以及带待处理识别时关闭。静态导入检查通过。Win7 x64/x86 核心构建、兼容钩子测试和 DLL 实际装载检查通过，固定 Winsock 序号的解析与未知序号拒绝测试通过。

日文 edition 的 x64 构建与测试通过：C++ 不创建手写窗口，Rust C ABI 按程序目录的包声明返回 `handwriting_not_offered`。CI 同时为五笔与日文编译两种原生宿主并跑测试，包身份与状态目录身份分别覆盖。

桌面验收模式 `--desktop` 使用自行创建的原生编辑器和另一测试窗口验证真实 `SendInput` 及前台边界。远端当前只有 Session 0，没有已登录的交互用户，实际运行在取得前台窗口时失败；修正测试环境预检后，该模式在无用户桌面时明确返回 `77`，不能计作通过。实际桌面与 Win7 SP1 验收仍有缺口。

## Consequences

Win7 构建能使用系统已安装的中文手写识别器，并有独立原生画布通过同一个 C ABI 处理笔划、模型回退与候选。代价是维护 COM 和 WinRT 两个后端、原生面板及其生命周期测试；普通 Windows 的运行后端不变。Tablet PC 组件和中文识别器属于系统可选能力，不同 edition 的组件组合仍需实测。

当前目标仅承载手写，尚不提供模型下载界面，也未接入普通启动器和完整安装包。其他面板、完整设置、Win7 SP1 组件与运行库、交互桌面候选上屏和安装仍由[完整支持提案](../../proposed/feature/2026-10-23-windows7-support.md)跟进；此切片不能作为完整 Win7 支持交付。
