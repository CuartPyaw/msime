# Agent Note: Windows 桌面对象与候选 UI 归属

Status: proposed

## Problem

#9439 使用 Sysinternals Desktops 的独立 Windows desktop object。当前一个 Server 的主线程创建一个 `CandidateWindow`，所有 TSF 客户端连接本版本的固定管道；在其他桌面输入时，候选仍只能属于建窗的桌面。改变已有窗口的位置或置顶状态不能改变其桌面归属。

## Proposal

保留一个生产 Server 及现有会话、词库、统计、剪贴板和后台服务，将候选 UI 的线程归属按输入客户端所属桌面拆开。每个候选 UI 线程在任何建窗和钩子之前绑定自己的桌面，独占候选窗、上下文菜单、读屏对象与候选关联浮层的创建、更新和销毁。主线程不得直接调用另一个桌面线程的窗口方法。

候选所属桌面必须来自已经验证归属的客户端与焦点租约，不能靠主线程的当前前台窗口猜测，也不能把任意客户端字符串当成可打开的桌面。路由信息应绑定现有连接代次和焦点代次；断开、失焦、桌面切换及旧代次到达时，各桌面窗口按既有隐私收起规则丢弃失效快照。点击、翻页、展开和渲染回执继续使用原会话身份，不创建第二份引擎或复用另一个桌面的候选索引。

下一步先验证跨进程客户端窗口／线程的归属查询及最小候选绘制路由，再确定可选协议能力与载荷；现有旧协议不得收到未经协商的新事件。尚未确定具体载荷，不能把本提案当作协议已经落地。

## Verified platform constraints

`platforms/windows/experiments/desktop-ownership.ps1` 在 Windows 原生运行时创建两个私有桌面，每个独立线程绑定后创建一个隐藏的标准窗口，通过 `EnumDesktopWindows` 检查窗口互不串入；主线程通过窗口线程 ID 和 `GetThreadDesktop` 查询两者的桌面归属。窗口和桌面在实验结束时销毁，实验不调用 `SwitchDesktop`，不启用或修改真实输入法。

同一实验确认未按桌面区分的命名互斥量在两个桌面都报告已存在，而各自带合成桌面后缀的名字互不冲突。因此「每个桌面直接启动现有 Server」会被当前 Server、启动门和 Watchdog 单实例限制挡住；只改候选坐标或单个互斥量不足以解决需求。

实验在远端 Windows runner 通过，退出 0；只证明同一进程内的系统对象及线程归属，尚未证明跨进程 TSF 归属、签名产品、候选绘制、焦点或选词正确。连接另一台 Windows 开发机超时不构成产品失败。

Microsoft 的 [Desktops 说明](https://learn.microsoft.com/en-us/sysinternals/downloads/desktops)明确每个窗口在创建时绑定桌面对象，系统不提供把已有窗口移到另一桌面的操作；[SetThreadDesktop](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setthreaddesktop)要求目标属于进程窗口站，且线程已有窗口或钩子时会失败；[线程连接规则](https://learn.microsoft.com/en-us/windows/win32/winstation/thread-connection-to-a-desktop)区分显式绑定、继承桌面与启动桌面。

## Related notes audit

检索活跃笔记中的 issue 编号、桌面对象和按桌面候选路由，未发现持有同一决定的记录。[游戏 UI-less 与 IMM](../../implemented/bug-fix/2026-10-16-windows-game-uiless-and-imm.md)及[后续候选协议](../../implemented/feature/2026-10-16-windows-source2-profile-and-local-online.md)持有宿主自绘和会话身份边界；本提案部分关联其点击与回包约束，不替代它们。[候选非阻塞读取](../../implemented/bug-fix/2026-10-23-windows-candidate-read-contention.md)持有锁争用下的可见性规则，增加 UI 线程时仍需保留。

## Alternatives considered

- 每个桌面一套 Server 和 Watchdog：复用当前单线程窗口与消息循环，线程归属最直接；但需同时区分全部主／回包／worker／Aux 管道、启动门、实例事件和监督归属，还会产生多个统计、剪贴板与后台服务写入者。优先保留共享状态的单一所有者，先验证候选线程路由。
- 在 TSF DLL 内画候选：窗口天然归属正在输入的应用桌面；但会把完整候选渲染、皮肤、资源与故障面带进宿主进程，改变当前 DLL 与 Server 的产品边界，因此不作为首选。
- 切换现有 Server UI 线程的桌面：不必维护多套候选窗；但线程已创建窗口与钩子，`SetThreadDesktop` 不允许这种切换，不能作为实现。
- 仅依赖已有 TSF UI-less：应用参与自绘时不需要外部窗口；但普通应用未必实现自绘，无法覆盖每个桌面正在输入的普通编辑器。

## Acceptance criteria

- 在两个以上独立桌面对象中的真实 TSF 编辑器分别输入、点击选词、键盘选词、翻页和展开候选，结果只写入原文档。
- 反复切回原桌面、失焦、销毁客户端与重连时，旧窗口及时收起，旧点击和旧渲染回执不能影响新会话。
- 默认桌面与旧协议行为保持兼容；错误桌面归属、拒绝访问、创建窗口失败和 UI 线程退出均有有界失败路径。
- 验证候选皮肤、鼠标操作、关联浮层与 UI-less；非候选的全局服务保持单一所有者。
- 原生对象探针、协议与路由回归、Windows 产品构建和完整 quick 通过；API 对象探针不能替代真实桌面编辑器验收。

## Risks

Windows runner 的 SSH 会话不等于交互桌面，现有实验没有验证跨进程 TSF 与 UIAccess 产品权限。多 UI 线程会影响当前候选字号、皮肤、键位图、打字特效和诊断回调的主线程所有权；必须先明确消息传递和销毁顺序。不得为了少改代码而让多个线程共享现有 `CandidateWindow` 或可变绘制状态。
