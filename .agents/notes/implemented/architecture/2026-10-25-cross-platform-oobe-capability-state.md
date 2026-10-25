# Agent Note: 全平台 OOBE 与系统能力状态分离

Status: implemented

## Problem

六个平台都有首次使用流程，但它们处理的对象不同：macOS 的安装页负责注册输入源和运行配置，Windows 安装器与 Server 分别保存安装偏好和运行时选项，Linux 在缺少配置或词库过期时进入准备页，Android 用原生引导和原子词库准备，iOS 用 SwiftUI 引导，鸿蒙按系统输入法实时状态推进状态机，Web engine 则把首次使用交给宿主。各平台没有统一的 OOBE 状态契约，最容易产生的误判是把“用户看过引导”当成“输入法已在系统启用”。

## Decision

- 各平台继续由原生宿主持有 OOBE 流程，暂不把平台特有的系统设置动作强行抽进共享 React 或 Rust。Web engine 没有系统 OOBE，由宿主负责。
- 统一检查和设计讨论使用四类状态：资源是否准备好、系统输入法是否已启用或为当前输入法、应用偏好是否已保存、用户是否离开过 OOBE。权限和账号是能力前置条件，不能由 OOBE 离开标记推导。
- iOS 没有公开 API 判断键盘是否添加到系统列表或是否获得完全访问；私有的 AppleKeyboards 读取和键盘写入标记都不作为状态来源。因此 `hasSeenOnboarding` 只记录“已看过/已离开首次引导”，并兼容迁移旧的 `hasCompletedOnboarding` 偏好。设置首页的 `KeyboardStatusCard` 长期保留两步操作，并明确告知用户 iOS 无法自动确认状态，不显示伪造的成功或失败图标。
- iOS 的“稍后再说”、跳过和 iPad 卡片关闭仍然可以离开首次流程；离开后用户可从设置页重新打开引导，系统能力由用户在系统设置中确认。

六个平台的现状核对如下：

| 平台 | OOBE 入口 | 离开后仍需确认的系统能力 |
| --- | --- | --- |
| macOS | 独立安装页 | 输入源注册、词库运行配置，以及部分系统版本需要用户在系统设置或重新登录后生效的输入模式 |
| Windows | 安装器询问数据目录和云候选；Server 首次启动写 `runtime-options.json` | 安装后没有统一的输入法可用性引导 |
| Linux | 缺配置或词库过期时进入设置页，校验、下载、准备并注册 IBus/Fcitx5 | 服务启用和当前输入法由桌面环境决定 |
| Android | Splash 后的原生四步引导，`Bootstrap.prepare` 原子准备词库 | 系统输入法启用与当前输入法选择 |
| iOS | SwiftUI 四步引导和长期状态卡 | 键盘列表、完全访问和当前输入法均需用户在系统界面确认 |
| HarmonyOS | 按“未启用 / 已启用但非当前 / 已完成”实时推进 | 麦克风和剪贴板权限仍在功能触发时处理 |

## Alternatives considered

- **用私有系统设置读取在 iOS 画出 ✓ / !**：可以让界面看起来像 Android，但有审核风险，且读取结果不稳定；不采用。
- **把“允许完全访问”写入 App Group 后当成长期真值**：实现简单，但用户撤销权限后旧标记仍在；不采用。
- **马上抽象成所有平台共用的 OOBE 状态机**：可以统一名称，却无法表达 macOS、Linux 和移动系统不同的权限及系统设置边界；先保留原生流程，统一状态语义和验收口径。

## Consequences

iOS 老用户的旧完成标记会迁移到 `hasSeenOnboarding`，不会再次被强制拉回引导；UI 测试使用的旧启动参数仍兼容。用户离开引导后，首页会继续显示两项系统设置动作和无法自动确认的说明，避免把“看过引导”误解为“键盘已经可用”。代价是 iOS 不能在首页给出自动化的完成勾选，系统能力验收仍需真实系统设置或输入框验证。其他平台暂不改变运行时行为，后续可按四类状态分别补齐资源、启用、偏好与权限的验收。

## Verification

- `git diff --check` 通过。
- iOS UI 测试 `testKeyboardHomePrioritizesTryoutAndQuickAdjustments` 增加了状态卡说明的存在性和文案断言；现有首次引导测试继续使用 `-hasCompletedOnboarding YES`，覆盖旧标记迁移兼容路径。
- 未启动完整 Xcode 构建：本机磁盘空间约 4.2 GB，不足以安全创建新的完整构建产物。

## Related decisions

- [iOS 与鸿蒙宿主采用全平台设计稿](../feature/2026-10-07-ios-harmony-all-platform-design.md) 记录了 iOS 无法检测系统键盘和完全访问的既有平台边界。
- [iOS 系统键盘验收显式准备状态](../testing/2026-10-20-ios-system-keyboard-acceptance-state.md) 记录了真实系统键盘验收为何必须显式准备环境。
