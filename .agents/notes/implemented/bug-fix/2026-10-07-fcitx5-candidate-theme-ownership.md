# Agent Note: Fcitx5 候选主题只按用户主动选择接管，无覆盖时退出并恢复

Status: implemented

## Problem

PR #4371#discussion_r4203158939 / r4203158947 指出，宿主的接管判据与生命周期都不对：

- `applyCandidatePanelTheme` 用「当前 classicui 主题是否可替换」（`fcitx_theme_replaceable`）决定接管，`refreshPreferences` 则用 `preferences.value("global_theme", "system") != "system"` 判断「用户选了主题」。启动、焦点进入、runtime options 刷新、系统明暗变化都走同一条路径，于是用户在 fcitx5-configtool 里选过第三方主题后，任意一次后台同步都可能把 `Theme` 强制写回 `msime`。
- 缓存 `theme == candidate_theme_applied_` 直接 `return`，跳过了所有权判断：classicui 被改回自带主题后，同一份配色不会重新接管。合并 develop 的每拍缓存后以 `chosen` 与落盘选择为准：输入没变而用户只是又选了一次水杉主题时仍接管，其余情况不追回用户自己改的值。
- 把全局主题切回「系统」后，水杉仍占着 classicui 的 `Theme`/`DarkTheme`，既不退出也不恢复。

PR #4371#discussion_r4203158943 指出解析边界：`candidate_dark_theme`、`surface_dark_theme`、`candidate_layout_id` 对 `preferences` 调 `.value()`，而 `preferences` 为 `null`、数组或数字时抛出 `nlohmann::json::type_error`；这三个 helper 由 IBus 宿主（`ClientEngine.cpp`）与 Fcitx5 宿主（`resolveThemeInMode`）共用，只修 `global_theme` 一行不够。

## Decision

- **接管依据是共享层解析出的实际候选覆盖**，不是 classicui 当前主题。`CandidateTheme::covers_candidates` / `candidate_theme_covers(resolved)`（`CandidateColors.h`）为真仅两种：共享层的 `candidate` 是非 null 对象（内置主题，或 custom 的皮肤槽位/取色器给了颜色），或 `candidate_skin` 非空（皮肤清单声明了当前布局与明暗）。`system`、基底 system 且无皮肤无颜色的 custom、未知/退役 id、解析失败落回的空文档都判为「没有覆盖」。它在 `candidate_theme_colors` 里**先于**空槽位被原生 palette 补齐前计算。
- **只有「用户主动选择」才允许从第三方主题手里接管。** `FcitxState::syncCandidatePanelTheme(bool chosen)` 的 `chosen` 只有两处为真：主题菜单 `setThemeChoice()`，以及偏好文件监视里 `candidate_theme_selection(preferences)`（`global_theme` + 整个 `custom_theme` 两个字段的可比较文档）发生变化时。`ensure()`（启动/焦点）、`refreshProviderSockets()`（runtime options 刷新）、`setSystemDark()`、以及指纹不变的偏好同步都传 `false`。
- **无覆盖时退出接管并恢复。** `restore_classicui_theme(fcitx::AddonInstance&)`（`FcitxEngine.cpp`）在 `!resolved.covers_candidates` 时调用：只把当前值仍等于水杉自己主题名的 `Theme`/`DarkTheme` 放进 `held`，再用 `panel_restore_values(record, "fcitx5", held, kClassicuiStockThemes)`（`PanelRestoreRecord.h`）求恢复值——记录里的 `prior`，记录没留下可用 prior 时用自带主题 `default`/`default-dark`。恢复记录缺失或损坏时仍用空记录调用 `panel_restore_values`，不能跳过自带主题兜底。用户自己改过的项不在 `held` 里，一项都不动。
- **恢复不是一次新的接管**：走 `classicui.setConfig()`，不经 `set_classicui_config()`（后者会 `record_classicui_takeover`），否则会把用户自己的值换成被恢复的值；只写 `Theme`/`DarkTheme`，`Font`/`WheelForPaging` 等其他项不动。
- **缓存只决定是否重写主题文件与是否重读 classicui 的完整配置**，不影响所有权判断、退出接管或 `publishCandidatePanelStatus`。合并 develop 的每拍缓存（#5988，输入没变就直接返回）后，接管写好且不是新的一次主动选择、落盘的主题选择与提示配置（跟随深色、字体、Wayland 字体 DPI）也没变时才跳过：用户自己改 classicui.conf 或卸载还原的主题不会被改回去，输入没变时也不再重新接管；用户在主题菜单里重新选择水杉主题（`chosen`）仍然立即重新接管。退出接管时同时清空 `candidate_theme_applied_`，否则选回同一皮肤会命中旧 stamp，却留下已清零的提示宽度。
- **非对象 preferences 按空文档处理**：`candidate_dark_theme`/`surface_dark_theme` 跟随系统明暗，`candidate_layout_id` 取默认纵向，`candidate_theme_request` 用 `find()` 本就返回 `system`。入口 helper 覆盖两个宿主。
- 自带主题名唯一副本 `kClassicuiStockThemes{{"Theme","default"},{"DarkTheme","default-dark"}}`，与 `msime-linux-setup --unregister` 的 `FCITX5_STOCK_THEMES` 同一规则。

## Alternatives considered

- **沿用 `global_theme != "system"` 作为接管触发** — 改动最小，但把「重读同一份偏好」也当成了主动选择。PR #4371 的测试预期「外部切主题后再次强制写回 msime」正是这个判据的直接后果，被本轮改掉。
- **新建一份「上次是否水杉接管」的状态文件** — 能精确记录所有权，但和 `panel-restore.json`（`prior`/`written`）职责重叠，且退出接管与卸载恢复会分成两套逻辑；复用既有记录 + `written` 一致性检查更小。
- **把恢复也写进接管记录** — 会让记录里的值在退出接管后变成用户的值，下一次接管/卸载的语义全错。恢复只写 addon 配置，不碰记录。

## Consequences

- **收益**：用户选的第三方主题不再被后台同步夺回；切回「系统」或无覆盖 custom 时真的退出并把仍属于水杉的项原地放回；classicui 被改回自带主题后同一份配色会重新接管；畸形偏好不再让宿主抛异常。
- **代价与已知上限**：「退出接管」无法区分「用户在 fcitx5-configtool 里主动选了水杉主题（`Theme=msime`）而偏好是系统」和「上次残留」，前者也会被改回 prior/自带主题——与卸载脚本同一规则。同理，后台同步不读 `getConfig()` 的代价是：用户在 fcitx5-configtool 改主题后不做焦点变化就切中英，会用到上一次同步的判据（见模式提示笔记）。`panel_restore_values` 的 `written` 一致性检查在当前调用点上不会触发，保留是为了与 `--unregister` 的 `restorable` 同构。

## Verification

`platforms/linux/tests/candidate/candidate_palette.cpp`（非对象文档、覆盖信号、`candidate_theme_selection` 边界）、`panel_restore_record.cpp`（`panel_restore_values` 的 prior/兜底/单项/`written` 不匹配）、`platforms/linux/fcitx5/tests/native.cpp::candidateThemePriority`（`--theme-priority`，真实 classicui：启动/焦点不夺回第三方主题、主动选择接管并记录、`Nord-Dark -> paper -> system` 恢复、单项被用户改过只恢复另一项、恢复不动字体与记录、恢复记录缺失或损坏时兜底、进程重启后尊重用户选择）。
