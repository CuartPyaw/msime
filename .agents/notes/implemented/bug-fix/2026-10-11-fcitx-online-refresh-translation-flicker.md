# Agent Note: Fcitx5 已显示在线查询不重复请求

Status: implemented

## Problem

Fcitx5 的 `refreshOnline()` 每 250 ms 运行一次，空闲防抖到期后仍反复请求同一份云候选和 AI 候选。每次成功回包都经 `msime_client_apply_online_candidates` 推进候选代次，`Runtime::refresh()` 清空译文；翻译重新完成后下一份在线回包又清空它，形成出现、消失、反复出现的循环。Omarchy 上同时开启在线候选与英文释义时能走到这条链路。

## Decision

请求分发前比较当前查询与该来源的 `online_display_queries_`；已成功显示的查询不再分发。云和 AI 分别判断，未命中的 AI 缓存探测仍能进入网络请求，未成功显示的请求仍保留现有重试行为。新组合、来源配置变化和 provider 切换仍按既有查询身份及失效机制刷新。

## Alternatives considered

- 保留跨代次译文，能避免 UI 出现空白；但真实输入或候选变化也可能推进代次，直接保留会混入旧释义，而且不能停止重复请求。
- 在共享 runtime 中比较前后候选，能让多个宿主的重复应用幂等；但会扩大到所有平台，重复请求和翻译查询重建仍存在。先修 Fcitx5 的请求分发根因，不改变共享接口。

## Verification

原生 Fcitx5 测试先显示真实翻译 socket 的合成译文，再执行三轮已过防抖的在线刷新；旧版本重复分发时回放同一份成功回包，断言捕获译文消失。修复后相同查询不再发请求、推进代次或清掉译文；提交后再输入相同拼音仍能发起新的在线请求。

以 `ce0865afa` 为基线时，`ctest --test-dir target/linux-ibus -R '^fcitx5-native-(context|ai)$' --output-on-failure` 在本机已安装的 Host API 库和源码编出的库上均复现失败；修复分发后云和 AI 两条路径均通过。Linux 原生全量 `ctest` 共 81 项通过，包括隔离 daemon/frontend 验收。

变基到上游 `ee2ab5a80` 后，这两项测试分别在更早的 `disabling the active provider succeeds` 和 `AI preference update succeeds` 断言失败，尚未进入翻译回归场景；还原宿主与测试两个文件为该上游提交、重新编译运行后也得到相同失败。因此旧基线的通过不能视为新基线上的翻译回归已通过。

## Consequences

复用每个来源已有的显示查询记录，不引入新的状态或修改共享 ABI；成功显示的同一查询只请求一次，译文不再被重复候选更新清空。同一查询不会持续获取新的服务答案，直到输入或配置改变；这与 IBus 的成功查询去重一致。

去重只覆盖已成功显示的查询，失败和空结果仍按现状重试。测试运行隔离原生输入上下文，不等同于安装修复后的插件在真实编辑器验收。

## Related notes

检索 `refreshOnline`、在线重复、翻译闪烁后的命中中，Windows 全屏候选笔记与本问题无关；[在线批次已有词去重](../../implemented/testing/2026-10-08-online-batch-existing-dedup.md) 处理的是批次内部词条去重，与查询分发去重互不替代，保留原决定。
