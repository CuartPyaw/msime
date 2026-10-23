# Agent Note: Windows 7 更新只选择专用发布与安装包

Status: implemented

## Problem

普通 Windows 发布依赖现代设置和运行库。兼容宿主如果继续使用 `platform: windows` 检查更新，会把 Win7 用户引向不能运行的普通安装包；仅在 UI 标注系统要求无法保证下一次更新仍保持支持。

## Decision

共享更新服务增加 `windows7` 平台契约，只读取 `windows7-v<版本>` 正式 tag，安装包限定当前 edition 的 `MetasequoiaIME-<Edition>_Windows7_Setup_v<版本>.exe`。同一包承载 x86／x64，不按宿主架构误选普通 Windows 包。继续使用同一共享下载、超时、版本排序、摘要与唯一资产规则，不新增一套网络实现。没有专用发布或资产时不回退普通发布；现代 `windows` 行为保持。

这只是选择契约，不触发发布。后续 Win7 构建和安装器必须遵守同样名称，完整打包、签名与 Win7 实机升级仍是独立验收项。

### 既有笔记审计

- [更新规则归共享 Rust](../../implemented/architecture/2026-10-09-update-check-in-rust.md)：部分重叠，继续使用该服务，只增加明确平台和资产命名分支。
- [Windows 关于页](../../implemented/feature/2026-10-20-windows-settings-about-tabs.md)：部分重叠，兼容页面复用现有展示模型，但请求专用平台。
- [完整 Windows 7 支持](../../proposed/feature/2026-10-23-windows7-support.md)：部分重叠，此契约防止更新丢失支持，不能替代整体安装验收。

## Alternatives considered

- **复用普通 Windows tag，再由文件名筛选。** 同一版本只有一篇发布说明，维护更少。但缺少兼容安装包时普通发布仍可能被报告为可更新，因此使用独立平台前缀，并且仅将包含本 edition 唯一兼容安装包的正式发布列为候选。

## Testing

- Windows runner 的 Win7 x64 目标执行 `cargo test -p msime-client-services --lib update_check::tests --release`，15 项通过，包括 3 项新专用发布测试和全部现代选择回归；同一目标 `cargo clippy -p msime-client-services --lib --release -- -D warnings` 通过。Tier 3 的 clippy 需要将 `-Z build-std=std,panic_unwind` 放在 `clippy` 子命令后面，放在 cargo 全局位置不会为它构建标准库。
- 同源 Win7 x64／x86 核心完成重建、真实 ABI／字体／音频枚举加载和已知导入阻断检查；x64 MCP 也完成构建。
- 原生关于页以合成发布响应验证专用请求、普通安装包拒绝、旧链接清除、None／Current／Available／Error、系统与问题报告复制、匿名统计 CAS 和帮助／反馈定位。完整版 x64／x86、五笔与日文 x64 的八套设置 CTest 全部通过。设置 EXE 已知导入审计为 x64 338／x86 332 项。

## Consequences

兼容宿主新增关于／帮助／反馈页：当前版本、检查更新、系统信息、匿名统计、随包许可、隐私链接、问题草稿和浏览器预填 issue 入口。诊断只报告实际系统版本、应用版本和方案，不含用户路径；复制与提交时重新读取草稿，不能依赖程序化文本变化一定发出 `EN_CHANGE`。窗口只在用户点击后打开浏览器，测试不发送消息、不打开浏览器、不改变真实账号。

专用发布命名隔离现代安装包，代价是打包流程必须遵守同一契约。目前不触发发布，不代表完整 Win7 安装、升级、实机输入和全部关于页附属能力已经完成；MCP 连接设置和赞助等仍须继续适配。最新全部源码 quick 另行记录，不能用上一个插件快照的 quick 代替。
