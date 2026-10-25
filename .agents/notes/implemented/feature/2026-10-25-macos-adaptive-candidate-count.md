# Agent Note: macOS 横向候选按宽度自适应与跨列展开

Status: implemented

## Problem

固定数量的横向候选随词长伸缩；历史最大宽度又会留下无用空白。需要按宽度选择一行可见候选，同时让展开后的长词占多列。

## Decision

macOS 横向增加 `candidate_count_mode`（`fixed` / `adaptive`）、`candidate_max_width`（默认 420pt，160–1200）和 `candidate_columns`（默认 6，1–10）。默认保持固定数量，不改纵向、其他平台和已有每页数量。固定数量取消横向最小宽度填充；自适应不足一行时收紧，溢出时以最大宽度封顶。滚动条占位保留。

自适应使用带代次的全候选快照做展示分页；导航同步引擎高亮，选择走已有全候选选择接口，不修改引擎排序、候选身份或共享分页协议。展开按等宽列向上取整跨度，键盘与滚动使用真实行边界。

自适应测量把候选内容宽度与序号栏分开，序号栏只按当前行的本地编号计算；换行重新从 1 开始，同一行达到两位数时同步扩展已有候选的序号栏。不能把全候选快照中的最大序号宽度塞进每格，否则显示时本地编号较短，差额就变成文字右侧的额外留白。皮肤最小项宽、用户内边距与滚动条占位仍参与测量。

滚动条占位只用于展开网格。未展开的单行没有滚动条，分页与实际排版只扣除窗口内边距和实际显示的翻页、展开按钮；不能提前扣掉展开网格的滚动条宽度，否则有空位也会少显示一项。短列表收窄时同样不追加这笔不存在的留白。

横向序号栏使用当前字体十个数字中的最宽字形乘以本地编号位数，测量与绘制共用该栏宽；保留原字体，不强制等宽数字。系统字体的数字 3 和 4 宽度不同，只按当前可见序号取最大值，会在候选数量从三项变成四项时移动所有候选的文字起点。同位数的栏宽不随候选数量变化，只有本地编号真正跨位数时才加宽；纵向继续沿用已有测量。

展开、收起用一条 `NSViewAnimation` 直接改变原候选窗的真实 `frame`（200ms），候选按钮按完整身份（会话、代次、索引）从旧位置移到新位置，不按文字匹配；`MSIMECandidateChromeView` 在尺寸变化时调整内容原点，保持顶部对齐。只有横向展开状态改变才播放；普通输入、选行、滚动和纵向同步更新，重排、隐藏或释放控制器时停止上一条动画。「减弱动态效果」沿用 `MSIMECandidateMotion.reduceMotion`，开着时直接到位。逻辑状态、命中和选择不等待动画。

参考仓库 `SlimeNull/macOS-Style-PinyinInputMethod` 的默认分支 `master`，固定提交 `5b5e27a438d94ac6ffedfd423177285f38a12167`。仅参考 `CandidateWorldPanel.ColumnCount` 的布局行为与移动动效，不复制源码；该提交没有许可证。

## Existing notes audit

- [恢复收窄](../../implemented/bug-fix/2026-10-22-macos-candidate-width-follows-content.md)：部分重叠，保留当前页可收窄的边界，不恢复历史宽度。
- [历史最大宽度](../../implemented/bug-fix/2026-10-20-macos-steady-candidate-width.md)：其 macOS 决定已被取代，不再实现。
- [单行宽度上限](../../implemented/feature/2026-10-22-candidate-single-line-adaptive-cap.md)：本方案只替代 macOS 横向的上限与分页，其他宿主规则不变。
- [排列选项模型](../../implemented/simplification/2026-10-16-candidate-layout-options.md)：仅管理排列方向，与数量模式独立。

## Alternatives considered

- 动态修改共享 runtime 的每页数量：可复用数字选词与翻页，但该接口只允许空闲时修改，且最大十项；按像素排布属于宿主，不应改变其他宿主的分页协议。
- 保留历史最大宽度：能减少抖动，但短词无法收窄，正是本次要去除的视觉缺陷。
- 同步缩窗再给内容挂图层裁剪与位移：窗口几何和命中立即就位，但旧网格已被真实窗口裁掉，收起过渡看不见。
- 截图覆盖窗：逻辑窗口可以立即切换并保留旧画面，但原窗与替代画面交接时真机仍闪烁，用户明确排除。

## Testing

设置顺序、横向可见性、配置保存与校验均有测试；真实原生控制器覆盖 160/400/600pt、50%/100%/200% 缩放与 3/6/10 列的矩阵，短列表收窄、跨列、翻页选择、纵向不变、滚动条留白与动画挂载。选择和导航的会话采用合成替身，纯布局测试验证一行可以超过十项，不把数字快捷键上限当显示上限。

右侧留白回归以 3、30、300 个合成候选检查首格宽度不受快照总数影响，并逐格验证文字之后只有既定内边距。纯布局测试覆盖本地序号从 9 到 10 时重测整行、放不下时换行重置序号，以及皮肤最小项宽。

单行装填阈值回归从真实按钮尺寸推算四个四字合成候选及展开按钮所需的精确宽度，验证恰好足够时显示四项、少 1pt 时只显示三项，并检查最后一项不覆盖展开按钮；展开网格的滚动条占位断言保持不变。

比例数字回归使用系统字体，在 18/32pt 与 50%/100%/200% 缩放下逐一改变一位数候选数量，检查序号到内容的起点保持一致；测试数据均为合成候选，不使用实际输入记录。

动画测试直接推进原窗口的动画进度（25%、75%、终点），验证动画目标就是同一个窗口、高度连续变化、内容顶部对齐，候选按钮起点对应展开前或收起前的位置；夹具含文字相同但身份不同的两项。完成回调由测试直接调用，不等真实时间：锁屏或熄屏时 `NSAnimation` 不前进。

- `pnpm --filter @msime/desktop exec vitest run tests/settings/candidate-page-size-section.test.tsx tests/settings/candidate-settings-layout.test.tsx tests/settings/candidate-window-style-section.test.tsx`：48 项通过；前端类型检查、生产构建与改动文件 lint 通过。
- `CARGO_TARGET_DIR=target/macos-cargo cargo test -p msime-client-core --locked adaptive_candidate_settings_defaults_round_trip_and_validate`：通过；`cargo fmt --all -- --check` 与同一产物目录的 `cargo clippy -p msime-client-core --all-targets --locked -- -D warnings` 通过。
- 原生 `candidate-row-layout-test`、`shortcut-test` 构建通过，候选布局、快捷键、工厂皮肤、声明动效与偏好默认值回归通过。
- `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer MSIME_SPARKLE_ROOT="$HOME/Developer/Dependencies/Sparkle-2.9.6" bash scripts/verify-local.sh --quick`：通过，含 macOS 原生宿主完整构建；缺少 Android NDK、HarmonyOS 工具链、Docker 和 Windows 交叉工具链的编译阶段跳过。
- `pnpm run verify-notes` 与 `git diff --check`：通过。

## Consequences

- 收益：横向数量与宽度独立于共享 runtime 的固定分页，不丢候选，也不改变纵向、其他平台或已有每页数量。长词跨列，短词可容纳更多；展开后的导航使用真实行边界而非下标除以固定数量。
- 代价：自适应每次排版测量完整候选，成本高于普通一页；异步候选按新代次重测。固定数量不再遵循皮肤的横向最小宽度和多余的均分留白，预编辑在窄窗内截取；必要翻页控件仍占实际宽度。
- 动效只在展开状态切换时直接改变原窗口的 frame；普通输入更新、展开后的选行、滚动和重排不播放布局动画。目标布局与输入状态同步完成，真实窗口在 200ms 内抵达目标，内容始终顶部对齐。减弱动态效果关闭过渡。保留展开网格的滚动条占位，不把最大宽度再乘整体缩放。
- 验证边界：离屏原生宿主与合成会话不是安装后在真实编辑器中的验收；不替换用户的已安装输入法。
