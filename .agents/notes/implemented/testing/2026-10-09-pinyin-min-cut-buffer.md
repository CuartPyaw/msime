# Agent Note: 拼音最短切分 DP 栈缓冲

Status: implemented

## Problem

`cut_one_piece_min_segments` 为每个输入建立长度为输入字节数加一的临时 `Vec<Option<(usize, usize)>>`，短拼音切分只使用其中很小的动态规划表，却在每次调用时产生一次堆分配。

## Decision

长度不超过 64 字节时使用栈上的 65 项 DP 表；更长输入继续使用原有堆向量，并把共同算法放入接受可变切片的内部函数。这样短路径消除临时表分配，同时不限制公开函数处理更长输入。

## Alternatives considered

始终使用固定表会把输入长度限制在 64 字节，可能改变公开函数对长拼音的行为。始终使用堆向量则保留短路径的无效分配。分支使用栈表或堆回退能同时保留边界和原有复杂度。

## Verification

新增短输入分配预算和超过边界的回退测试；旧实现短路径为 4 次分配，新路径为 3 次。更新受该临时表影响的 6 个会话分配预算断言。`msime-engine` 全量 1364 项单测、31 项 golden、doc tests、Clippy、Rustfmt 和 quick 门禁已通过。

## Consequences

常见短拼音最短切分不再为 DP 表申请堆内存；长输入仍按输入长度分配并保持原算法结果。
