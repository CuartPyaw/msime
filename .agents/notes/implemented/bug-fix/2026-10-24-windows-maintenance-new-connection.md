# Agent Note: Windows 词库维护期间延迟新连接的会话创建

Status: implemented

## Problem

#9434 报告 Windows 设置新增快捷短语和五笔词条时失败，偶尔提示词库被占用。Server 的 `quiesce_dictionaries` 销毁既有会话，却没有阻止 `InputState::connected` 为随后接入的新客户端创建会话；新会话重新获取共享词库锁，设置进程就不能取得独占维护锁。这是源码中可以确认的竞争路径，尚未取得报告现场的证据，不能将所有新增失败都归因于它。

## Decision

维护期间仍登记连接及字段属性，但新客户端不创建 `FocusedSession`。维护结束由现有 `resume_dictionaries` 为全部无会话客户端统一重建；正常连接继续立即创建。会话创建及销毁始终在输入队列线程进行。

## Historical notes

- [词库访问租约结束时释放全部文件锁](../../implemented/bug-fix/2026-10-15-dictionary-access-lock-release.md)：部分重叠，处理 Rust 租约析构；本篇处理 Windows 宿主在维护窗口内重新创建持锁者，互链保留。
- [个人词库操作结束时显式释放锁](../../implemented/bug-fix/2026-10-15-personal-dictionary-lock-release.md)：无关，处理 Unix 描述符继承。
- `proposed` 与 `rejected` 中没有维护连接相关决定；其他维护命中涉及平台安装、资源准备或批处理，不规定 Windows 新连接的会话生命周期。

## Alternatives considered

- 维护期间拒绝新连接可以阻止重新持锁，但会让 TIP 承受断连和重试；既有恢复逻辑已能处理无会话连接，保留连接更直接。
- 延长设置进程重试可以吸收短暂竞争，但新会话会一直持有共享锁，等待不能修复维护期间再次创建会话的缺陷。

## Verification

`windows-input-queue` 新增不含有效引擎配置的维护连接回归：维护期间新连接和同客户端重连均成功登记，重复释放为零，随后可断开。旧实现会尝试创建会话而失败。`windows-session` 的真实后端回归覆盖先释放原连接，再登记新连接及重连、重复释放为零、恢复两个会话和再次恢复不重复创建。

本机 Clang 和 MinGW x64 对 `InputQueue.cpp`、`input_queue.cpp`、`session_smoke.cpp` 的 C++17 严格编译检查（`-fsyntax-only -Wall -Wextra -Werror`）通过。`pnpm run verify-notes` 和 `git diff --check` 通过。上述运行回归尚未执行；本机和远端磁盘均低于仓库 50 GB 构建门槛，没有启动新的完整构建树，完整 `scripts/verify-local.sh --quick` 与 Windows 设置页真实新增验收仍缺失。

## Consequences

维护期间新客户端暂时不能使用引擎，与已有客户端一致；恢复失败仍沿用现有重连回退。需要补齐真实 Windows 新增词条验收才能认定 #9434 全部解决。
