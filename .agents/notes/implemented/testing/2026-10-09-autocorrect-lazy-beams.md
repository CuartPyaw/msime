# Agent Note: 自动纠错 beam 延迟分配

Status: implemented

## Problem

`pinyin::autocorrect::Search::new` 为输入的每个字节位置都创建容量为 `k` 的 beam。许多位置没有任何可达路径，尤其是无法解释的输入仍会为每个空位置分配一块 `Hypothesis` 缓冲。

## Decision

初始化时只建立空的 position `Vec`，第一条路径写入某个位置前再精确预留 `k` 个槽位。已有路径仍按原来的 beam 容量和追加顺序工作；序列 interning、最终排序、去重、截断和回溯结构保持不变。

## Alternatives considered

- **继续为所有位置预留 `k`**：实现最简单，但空位置的分配与输入长度成正比。
- **所有 beam 共用一块连续缓冲**：可以减少外层分配，但会改变按位置保存和 predecessor 索引的结构，扩大回溯改动范围。
- **使用固定栈数组承载 beam**：常用 `k` 为 9，但公开接口允许更大的 `k`，固定容量会引入截断或额外回退路径。

## Verification

新增测试证明 `Search::new(64, 9)` 的空位置初始化最多 3 次分配，首条路径到达时仍精确获得 `k` 个槽位；不可读的合成输入 `qqqqqqqq` 查询保持空结果，分配为 3 次。自动纠错测试 17 项、引擎全量单元测试 1368 项、golden 测试 31 项、doc tests、Clippy、Rustfmt、差异检查、笔记校验和 `bash scripts/verify-local.sh --quick` 均通过。

## Consequences

无可达路径的位置不再分配 beam 缓冲；可达路径的容量、候选顺序、纠错成本、去重结果和回溯内容保持不变。
