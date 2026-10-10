# Agent Note: Windows 中英文切换体验对齐 macOS：切换提示、应用例外、Alt+Shift+H

Status: implemented

## Problem

macOS 的中英文切换有三样 Windows 没有的东西。切换后光标旁会闪一下「中」或「英」的徽标（`input_mode_hud`），Windows 只有任务栏的语言栏图标，盯着光标打字的人看不到模式变了。「应用例外」可以给指定应用固定起始模式，比如终端和代码编辑器总从英文开始，但规则表存在 macOS 的 NSUserDefaults（`MSIMEClientAppInputModeRules`）里，共享设置页和 Windows 都读不到。Option+Shift+H 切换全半角由共享开关 `keybindings.toggle_fullwidth_option_shift_h` 控制，Windows 不认这个组合键，能力位也把开关藏了起来。审计还列了 Shift+空格切换中英文，那一项这次没有做，原因见 Alternatives considered。

## Decision

**应用例外是共享偏好。** `Preferences::app_input_mode_rules`（`crates/client-core/src/preferences.rs`）是应用标识到 `chinese`/`english` 的表。macOS 用 bundle id 作键，Windows 用进程的可执行文件基名作键（设置页写入前转成小写），两种键放在同一张表里，彼此不会相撞。表为空时不写进文档（`skip_serializing_if`），没有这个键的旧版本照样能读。`validate` 要求最多 32 条；每个标识非空、不超过 64 字节、没有控制字符和 `\` `/`、两侧没有空白；整张表按 ASCII 不分大小写地不重复，否则拒绝并报 `InvalidAppInputModeRules`，Tauri 映射为 `app_input_mode_rules_invalid`。规则不参与云端同步，因为 bundle id 和进程名只对本机有意义。

**macOS 改读共享文档，NSUserDefaults 只作回退。** `AppearancePreferences.mm` 的 `-applicationInputModeRules` 有文档值就用文档值，没有就读本地旧键，-englishMode 的「规则 > 记忆 > 默认」顺序不变。`-applySharedInputPreferences:` 读到文档里有这个键、而且旧键里的规则文档都已经有了时，迁移就完成了，本地旧键随即删除，否则以后文档清空规则时，旧规则会经回退重新生效。旧键里还有文档没有的规则，说明升级后还没保存过，共享设置页就先写了规则表；这时把旧规则并进来、旧键留着，等下一次保存一起写进文档，不让升级前的规则被悄悄丢掉。缺这个键就是没有规则。原生窗口改规则时照旧先写本地键，`-sharedPreferencesByMerging:` 每次都把整张表写进文档，但只写偏好库收得下的部分（`PublishableInputModeRules`：标识合法、不分大小写不重复、最多 32 条）。偏好库对不合法的表拒绝整份保存，升级前留下的第 33 条规则或一个超长的 bundle id 就会让本输入法此后的每次保存都失败。原生窗口的「添加应用…」也按同样的规则当场拒绝并说明原因。`PreferenceSnapshotMerge.h` 对 `app_input_mode_rules` 整体替换、不逐项合并，移除的规则才会真的从文档里消失。

**Windows 由 Server 执行规则，TIP 只定激活时的起点。** `ModeAuthority.h` 的 `mode_authority_step` 多收焦点客户端的进程号和它的规则：

- 焦点从别的进程进来，就是一次新的停留，上次的手动让位作废。
- 有规则且本次停留没有让位时，推规则里的模式。两种作用域下都推，全局状态不动。
- 同一会话里模式变了，又不是 Server 自己刚推的（`pushed` 记着推了什么），就是用户切换了：规则在这次停留里让位，全局作用域下这个模式成为新的全局状态。
- 同一进程里换输入框不算新的停留。

Server 换客户端时用 `process_image_base_name`（`src/system/ProcessImageName.h`，`PROCESS_QUERY_LIMITED_INFORMATION` 加 `QueryFullProcessImageNameW`）问出进程名，再到 `common/AppInputModeRules.h` 里不分 ASCII 大小写地查。TIP 激活时在 `InitializeMetasequoiaIMECompartment` 里先查本进程的规则，查不到再用 `default_ime_mode`，这样第一份状态快照就是规则的模式，不会先闪一下默认模式。规则在偏好发布时更新，下一次进入应用时生效，和 macOS「激活时生效」一致。

**切换提示只在用户切换时出现。** `mode_authority_step` 的 `user_changed` 只在同一会话里出现、非 Server 推送的模式变化时为真，焦点切换、规则推送和全局状态推送都不会让它为真。Server 收到它、`input_mode_hud` 开着、前台不是全屏呈现时，用 `InputModeHudWindow`（`src/candidate/InputModeHudWindow.cpp`）显示徽标。窗口不激活、鼠标穿透、置顶，按每显示器 DPI v2 摆放。配色和悬浮工具栏同一份调色板。尺寸取工具栏的 `font_size` 和 `scale_percent`（`InputModeHudLayout.h`）：与工具栏同高，左边 logo，右边一个字，0.6 秒后隐藏。位置按顺序取：

- 前台线程的系统光标，只认每显示器 DPI 感知的窗口；
- 这个会话最近一次组字的锚点；
- 都没有时，屏幕水平居中、下方三分之一处。

摆放规则和 macOS 的 `MSIMEInputModeHUDFrame` 相同：光标下方放得下就放下方，放不下放上方，夹在工作区里。能力位 `input_mode_hud` 加上 Windows。WinUI 设置的「输入 › 中英文」组和共享设置页都有这个开关。

**Alt+Shift+H 在 Windows 上同样切换全半角。** `_MatchChordInputHotkey` 在中文模式下、`toggle_fullwidth_option_shift_h` 开着、不带 Ctrl（AltGr 也报成 Ctrl+Alt）和 Win 时，把 Alt+Shift+H 当作全半角热键（`tsf/Global/FullwidthChordPolicy.h`），英文模式下原样交给应用，和 macOS 一样。Ctrl+Shift+Space 不受这个开关影响，macOS 也是这样。能力位 `fullwidth_chord` 加上 Windows，共享快捷键页按平台显示「Alt+Shift+H 切换全半角」。WinUI 快捷键页加同一个开关，「恢复默认快捷键」本来就整体恢复 `keybindings`，也会把它恢复。

**设置界面。** 新能力位 `app_input_mode_rules`（macOS、Windows）控制共享设置页「中英文」组里的规则表（`app-input-mode-rules-section.tsx`）：每条规则一行，可以改模式、可以移除；末尾一行填标识后添加，新规则从中文开始。Windows 上填的是程序文件名，粘贴完整路径时只取文件名。WinUI 设置的同一组有同样的规则表，写入前的校验在 `settings/AppInputModeRuleList.h`。两处校验和偏好库一致，另外要求 Windows 上以 `.exe` 结尾。

## Alternatives considered

- **规则只放在 Windows 本地（注册表或 Server 自己的配置）。** 改动最小，macOS 和共享偏好都不用动。但这样会出现第二份「应用例外」，共享设置页仍然没有地方编辑它，两个平台说法不一。用户要的是一个规则表，所以把它提升成共享偏好，并迁移 macOS。
- **规则只在 TIP 激活时生效，Server 不管。** TIP 天然知道自己的进程名，不用开进程句柄。但 TIP 只在输入法激活时运行这段代码，普通的应用切换不会重新激活，而 macOS 的语义是每次进入应用都回到规则。全局作用域下 Server 还会把全局状态推到有规则的应用上，盖掉规则。所以由 Server 按焦点进程执行规则，TIP 只负责激活时不闪。
- **macOS 的整张表经现有的逐项深合并写进文档。** 不用改 `MSIMEMergePreferenceSnapshot`。但深合并不会删键，原生窗口里移除的规则会永远留在文档里，也会经文档回到 Windows 设置页。
- **切换提示从 TIP 的 IMESwitch 事件触发，而不是从 Server 的模式观察里推断。** 事件更直接。但 Server 推送的模式也会产生同样的事件，要分清「用户切的」还是「Server 推的」，还得另加标记。`mode_authority_step` 本来就跟踪每个会话的模式和自己的推送，在那里判断最少。
- **Shift+空格按 macOS 的做法与单按 Shift 共用 `switch_language_shift`。** 这样和 macOS 完全一致，Android 也是这样做的。没有做的原因：Windows 上 Shift+空格历来是微软拼音的全半角键，来源宿主的资源字符串也写着它切全半角；Excel 用它选整行，浏览器用它向上翻页；而 `switch_language_shift` 默认开，跟着它一起开会让 Windows 用户手速快时「大写字母后接空格」被吞掉并切到中文。要做只能另设一个默认关的键，那又和 macOS 共用的那个开关分了家。这一项留给产品决定。

## Consequences

- **收益**：Windows 用户切换中英文后在光标旁看得到模式，指定应用可以固定起始模式，Alt+Shift+H 切全半角，开关都能在 Windows 的两套设置界面和共享设置页里找到。macOS 的规则表进入共享文档，能在共享设置页里编辑。
- **代价与已知上限**：
  - 徽标靠系统光标或组字锚点定位。没有系统光标、又是 DPI 感知不是每显示器的窗口、还没组过字的输入框，只能退回屏幕下方居中。
  - 徽标不淡出，到时直接隐藏。macOS 淡出 0.18 秒；淡出需要按帧重画，带透明度的画刷会让 `DeviceResources` 的画刷缓存无限增长，所以没做。
  - Alt+Shift+H 依赖 TSF 把带 Alt 的按键交给按键接收器，也依赖 Windows 的 Alt+Shift 换语言热键在中间按了别的键时不触发。这两点都要在真机上确认。
  - 规则表让偏好文档变大，32 条、每条 64 字节时最多约 2.6KB。Windows Server 的偏好快照和 iOS 桥接都把整份文档限在 16KiB，规则表和游戏程序列表一样只限制自己这部分，整份能否留在上限以内还取决于其他字段。
  - 规则表的 32 条上限对两个平台合计生效。
- **何时重访**：产品决定 Windows 也要 Shift+空格时；有人反馈某类应用里徽标位置不对时，比如 UWP、游戏或远程桌面；macOS 的原生设置窗口下线时，本地旧键的回退可以一起删掉。

## Verification

- Rust：`cargo test -p msime-client-core --lib -- app_input_mode host_surface` 覆盖文档读写、校验和能力位。
- 共享设置页：`apps/desktop/tests/settings/app-input-mode-rules.test.tsx` 覆盖规范化、校验、Windows 上编辑并保存规则表、切换提示与 Alt+Shift+H 开关。
- macOS：`tests/settings/PreferenceSnapshotMergeTest.mm` 覆盖整表替换；`tests/input/ShortcutTest.mm` 的 `TestApplicationInputModeRulesFollowSharedDocument` 覆盖迁移、回退、未发布旧规则的合并、发布过滤和清空。
- Windows 宿主：`tests/input/mode_authority.cpp`（规则、让位、推送落地、`user_changed`）、`tests/input/app_input_mode_rules.cpp`、`tests/ui/input_mode_hud_layout.cpp`、`tests/ui/app_input_mode_rule_list.cpp` 和 `tsf/tests/input/fullwidth_chord_policy.cpp`。

和 macOS 按布局字母认 Option+Shift+H 的取舍见 [macOS 字母快捷键跟随键盘布局](../bug-fix/2026-10-09-macos-letter-shortcuts-follow-keyboard-layout.md)；悬浮工具栏一侧的对齐见 [Windows 悬浮工具栏对齐 macOS](2026-10-10-windows-floating-toolbar-mac-parity.md)。
