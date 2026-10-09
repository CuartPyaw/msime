# Agent Note: iOS 新增「原生」键盘皮肤，只画系统键盘的样子

Status: implemented

## Problem

#6116 的用户觉得 iOS 键盘的皮肤都不像 iOS 自带键盘，希望有一套极简、接近原生、能配 iOS 26 Liquid Glass 的皮肤。当时没有任何一个选项能画出这个样子：

- 「跟随系统」`system` 在触屏端按应用主题当前季节取色（见 [触屏键盘默认跟随系统](2026-10-08-touch-keyboard-default-follows-app-theme.md)），秋天是米色加赭红。应用主题总能解析出来（默认 `siji`），所以 `NativeKeyboardTokens` 和透明背景只在应用主题解析失败时才用得上，实际走不到。
- 其余内置主题都是品牌配色；自定义设计带渐变和圆角。
- 按键圆角固定 5pt，带 1pt 底边阴影，是 iOS 26 之前的样子。

## Decision

新增独立的全局主题 `native`，标题「原生」，排在「跟随系统」之后。「跟随系统」= 季节色的决定不变，新装默认值仍是 `system`。

- **契约**：`GlobalTheme::Native` 进 `ALL`（8 项）。它和 `system` 一样不带调色板，`resolve` 返回 `source: system`、两个调色板都是 `null`，但 `id` 原样是 `native`，宿主据此区分。`is_base()` 对它为假：自定义主题的底和皮肤包清单的 `base` 要在每个宿主上都画得出来，所以不能写 `native`。`CustomTheme::validate` 因此改成拒绝所有非底主题；从 `native` 开始自定义（Rust 的键盘试用、`packages/ui` 的 `customThemeBase`、iOS 的 `GlobalThemePreference.customizing`）时，底退回 `system`。
- **只在 iOS 提供**：`GlobalTheme::platforms()` 对 `native` 返回 `[ios]`，目录项多了 `platforms` 字段。`theme::catalog()` 仍列全部主题（`packages/ui` 的 `theme-catalog.json` 是它的副本）；`theme::catalog_for(platform)` 只列该平台提供的。C 接口 `msime_client_theme_catalog` 不带参数，按库编译到的目标平台调用 `catalog_for`，所以 Android、鸿蒙、macOS 的原生选择器不用改就看不到它。共享设置页用 `offeredThemeCatalog(host.platform)` 过滤轮播和鸿蒙网格；不知道宿主时不列它。
- **其他宿主收到 `native` 时**：偏好文档照常接受（serde 认识它），`resolve` 给出与 `system` 相同的空调色板，宿主画平台 token。共享设置页的 `settingsThemePreferences` 把宿主不提供的主题读成 `system`，选择器标「跟随系统」为选中。实际上这些宿主收不到它：同步键按平台分开（`platform.ios.global_theme` 只有 iOS 读写）。
- **iOS 渲染**（`KeyboardTheme`、`SystemKeyboardTokens`）：`native` 不填季节配色，`drawsNativeBackground` 为真，背景透明，露出 `KeyboardInputView`（`UIInputView` 的 `.keyboard` 样式）自带的系统键盘底板——iOS 26 起这块底板就是系统的 Liquid Glass，更早的系统是系统模糊材质。按键：
  - iOS 26 起照系统键盘画：字母键和功能键同色（浅色 `#FFFFFF`、深色 `#464646`），没有底边阴影，iPhone 圆角 6pt。这些值是在 iOS 27 模拟器上对系统键盘截图量出来的；同一张图上浅色底板 `#E2E3E8`、深色 `#222223`，只给没有系统底板的预览用。iPad 没有量，圆角按经典取值的比例取 8pt。
  - iOS 17–25 沿用 `NativeKeyboardTokens` 的经典 UIKit 取值，底边阴影换成中性黑色（透明度不变），不用品牌绿。
  - 回车键和选中候选用系统蓝，取 `systemBlue` 的增强对比度值：浅色 `#0040DD`，深色 `#409CFF`。回车键上的文字浅色下用白色，深色下用黑色。
- **云同步**：`platform.ios.global_theme` 是 `native` 时两条上传路径（原生 `IOSCloudSettings` 和 Tauri 的 `local_account_preferences`）都不上传这个键，云端保留原值。理由与「触感强度跟随系统」先不上传相同：旧版 iOS 的 `IOSPreferencePlan` 和 Rust 的 `from_cloud` 都按主题 id 白名单校验，收到不认识的 id 会拒收整份设置文档。新版下载时认识 `native`。Android、鸿蒙用各自命名空间的键（`platform.android.global_theme` 经 `GlobalTheme::from_id`，`platform.harmony.global_theme` 经 `GLOBAL_THEMES`），iOS 不写这两个键。服务端字段表不在本仓库，有没有枚举约束这里查不到；上面的客户端校验是本仓库内唯一的约束。
- **旧的皮肤文件夹名**：`native` 现在是保留 id，新导入的皮肤包不能叫这个名字，叫这个名字的已有文件夹也不再被扫描。`is_selectable_id` 仍接受它，与 msime-windows 内置外观名的处理一样：偏好里存着 `custom_theme.candidate_skin = "native"` 的旧文档仍能读，只是找不到这个包，自定义主题照常画它的底。

## Alternatives considered

- **把 iOS 的「跟随系统」改回 Rust 契约原本的含义（纯系统 token，不跟季节）**：不用加 id，跨平台契约不动，`theme.rs` 的文档注释也重新成立。但它推翻 #6029 刚定下的「键盘与应用同色」，而且只在 iOS 上翻转，同一个 `system` 在 Android、鸿蒙和 iOS 上会是两种意思。用户已拍板保留季节色。
- **每个按键放一个 `UIGlassEffect` 的 `UIVisualEffectView`**：最字面地「用 Liquid Glass」。但模拟器上量出来的 iOS 26 系统键盘按键是扁平的不透明色块，玻璃只在底板上；给每个键叠玻璃画出来反而不像系统键盘。键盘扩展有内存上限，三四十个效果视图也偏重，按下态、重新着色和回车键的状态切换都要为它另写一套。
- **目录在所有平台都列 `native`，由各原生宿主自己过滤**：C 接口不用知道平台。但 Android（Java）、鸿蒙（ArkTS）、macOS（Objective-C++）的选择器都要各改一处，漏改一个就在那个平台上多出一个画不出来的选项。按编译目标在 Rust 里过滤，只改一处。
- **上传 `native`**：同一账号的 iPhone 和 iPad 之间能同步这个选择。但同账号下还没升级的 iOS 设备会因此拒收整份设置（方案、简繁、按键音一起同步失败）。等旧版本淘汰后再放开。
- **系统蓝用常规值 `#007AFF`**：最像系统。但它写在浅色键盘底板上只有约 1.7:1，选中候选几乎看不清；深色下 `#0A84FF` 上的白字约 3.6:1，低于本仓库按键文字 4.5:1 的门槛。增强对比度值仍是系统自己的蓝。

## Consequences

- **收益**：iOS 用户多了一个照系统键盘画的皮肤，iOS 26 起透出系统的 Liquid Glass 底板；「跟随系统」和默认值都不变；其他平台的选择器不用改代码。
- **代价**：契约里多了一个只在一个平台提供的主题，宿主要么用 `catalog_for`，要么读 `platforms`；`CustomTheme.base` 的取值范围比主题 id 窄了一项。`native` 在 iOS 设备之间暂不同步。深色下回车键是黑字，与系统按钮的白字不同。
- **验证**：Studio 上 `cargo test -p msime-client-core`（1150 项）、`msime-host-api` 的主题用例、`msime-tauri-mobile-platform`、`msime-desktop` 的 iOS 账号与主题解析用例通过，fmt 与 clippy 零警告；`swift test --package-path shared/backend` 通过；本机 vitest 跑皮肤页、鸿蒙网格、主题偏好、主题选择更新和自定义主题对拍，`tsc --noEmit` 通过。iOS 单测在 Studio 上 `build-for-testing`、拉回本机在 iOS 27 模拟器上跑完整个 `MSIMEKeyboardTests`（624 项，0 失败），新增用例覆盖原生皮肤的配色、圆角、阴影、透明背景、底和上传；用例附带的真实键盘渲染图与同一模拟器的系统键盘截图并排对过。没有在真机上看过：键盘扩展里透明背景露出的底板以 c69ac2734 的真机观察为依据，iOS 17–25 上的经典按键和 iPad 圆角都没有对照系统键盘。
