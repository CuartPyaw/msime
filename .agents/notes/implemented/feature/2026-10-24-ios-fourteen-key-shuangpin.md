# Agent Note: iOS 双拼 14 键

Status: implemented

## Problem

iOS 已有全拼 14 键和多种 26 键双拼方案，但双拼只能使用 26 键。#9190 需要讯飞式的双拼 14 键，同时保留用户当前的小鹤、自然码、微软或首道双拼 profile。共享引擎的 14 键双拼解码由 #9404 提供，iOS 需要把它接到原生设置、会话和键面。

## Decision

复用 #9404 的 `touch_shuangpin_layout`（`twenty_six_key` / `fourteen_key`）作为 iOS 与 Android 共用的独立布局偏好。iOS 设置页在当前方案是双拼时提供 26 / 14 键选择，布局不改变 `shuangpin_profile`；键盘会话按文档读取布局，14 键使用现有 `FourteenKeyLayout` 和 `grid_key`，26 键继续走原有双拼字母路径。Host API 的自动网格门控扩展到 `HostPlatform::Ios`，从而新会话、会话重建和设置热加载保持一致。

布局选择写入共享 `PreferencesStore`，再镜像到键盘扩展的 App Group。切换双拼方案、重建会话和回到中文时保留选择；英文、本地模式和不支持组码的输入框回落到 26 键。物理字母仍经 26 键双拼路径，触摸 14 键按组首字母经 `grid_key` 进入共享引擎。iOS 账号同步继续使用原有全拼布局字段，不上传这个仅本机的双拼布局选择。

### 历史笔记检索

- [Android 14 键双拼](2026-10-23-android-fourteen-key-shuangpin.md)：部分重叠，共用 `touch_shuangpin_layout`、Host API 门控、14 键双拼引擎契约和多义读音处理；iOS 只增加原生 Swift 设置、会话桥和键盘扩展接线。
- [全拼 14 键](2026-10-10-fourteen-key.md)：部分重叠，复用键位、触摸来源、拼音选择和候选面板契约；本方案保留双拼 profile。
- [九键双拼](2026-10-10-nine-key-shuangpin.md)：部分重叠，复用两键多义解码、拼音选择和退格恢复；不改变九键数字方案。
- [自定义双拼方案](2026-10-10-custom-shuangpin-profile.md)：部分重叠，方案解析及非法表回退仍归现有实现，14 键只消费解析结果。

## Alternatives considered

- **增加一个名为「双拼 14 键」的独立输入方案**：设置入口简单，但会把布局和双拼 profile 绑在一起；从小鹤切到自然码时会丢掉用户的方案，且共享 `touch_keyboard_schemes` 没有这个方案值，因此不采用。
- **只在 iOS App Group 保存布局，并由键盘每次手动调用 `set_key_grid`**：可以避免扩展 Host API 的平台门控，但新会话、词库维护重建和跨进程设置读取会出现不同步，且 canonical `PreferencesStore` 不知道用户选择，因此不采用。
- **把 iOS 布局写进 `touch_keyboard_layout`**：字段已经表示全拼、九键和手写的方案布局，双拼另有当前 profile；混用会让全拼与双拼的选择互相覆盖，因此沿用 #9404 的独立字段。

## Consequences

- iOS 原生设置、键盘扩展和 Host API 共享同一个布局字段；切换双拼 profile 不会丢失 14 键选择，默认值仍是 26 键。
- 14 键双拼复用现有键面、拼音选择条、候选面板、分词和退格契约，物理字母输入保持原有 26 键行为。
- 旧版客户端不认识 `touch_shuangpin_layout` 时会按 #9404 已接受的降级边界处理；iOS 账号同步不携带该本机布局，因此跨设备不会覆盖本机选择。
- #9404 当前尚未合入 `develop` 且远端状态为冲突；本分支只能作为 stacked PR，合并前需先让共享基础 PR 更新并合入。

## Verification

- `cargo fmt --all -- --check` 通过。
- `git diff --check` 通过。
- `cargo test -p msime-host-api fourteen_key_shuangpin_preference_is_applied_by_touch_hosts --locked` 通过。
- `cargo test -p msime-host-api fourteen_key_shuangpin --locked` 两项通过，覆盖 iOS/Android 宿主门控、profile 变化、硬件输入与 14 键布局保持。
- `cargo test -p msime-client-core touch_shuangpin_layout --locked` 通过。
- iOS Xcode 工程已完成 Swift 编译阶段检查；最终链接因当前 worktree 缺少 `target/ios/simulator/libmsime_host_api.a` 失败，因此没有宣称 iOS 模拟器运行验收通过。
- `pnpm run verify-notes`、本次改动的 Rust 格式检查和差异检查在提交前执行。
