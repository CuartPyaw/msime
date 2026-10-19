# Agent Note: 运行时接入 token 嵌入直接写入

Status: proposed

## Problem

默认 Runtime 九键仍超过 16ms。完整锁定资源的当前采样继续包含矩阵运算与二元评分热点，另从模型源码确认每个 token 的 embedding 临时 Vec 申请后立即复制析构。模型上游的直接写入原型通过逐位测试和完整 embedding 入口配对预算，主项目需要固定接入而不复制模型实现。

## Proposal

上游 PR #77 合并后，将两个 crate 清单、工作区和 fuzz 锁文件同时固定到实际合并提交。Float 与 Quantized token 嵌入直接写入最终输入行，保持先乘 scale 再加 position、token 上界裁剪和位置饱和；没有新增缓存、常驻模型展开、unsafe 或公开接口。

## Existing decisions

已检索 embedding、token.row、嵌入、模型前向、query scratch、矩阵调度和反量化。[query scratch](../../implemented/testing/2026-10-17-runtime-query-scratch.md)和[前馈 scratch](../../implemented/testing/2026-10-16-runtime-feed-forward-scratch.md)部分重叠，分别继续持有 query 和每层前馈缓冲寿命；本片只删除进入前向前的 token 临时行。矩阵交错、临时反量化、attention scratch 与短句入口的负结果均保持，不重复实现。没有完全吸收、翻转或过时提案。

## Alternatives considered

- 在主项目复制模型的嵌入代码：可以立即改动，但会分叉模型数值和缓存契约，继续由上游持有实现。
- 只接入上游任务分支：便于提前测试，但来源会移动，最终 pin 必须使用实际合并提交。
- 继续保留临时行：代码最少，但局部完整入口的全部 16 形状已证明直接写入能覆盖开销，没有增加持有状态。

## Acceptance criteria

上游完整 16 形状配对门槛、真实模型 embedding/前向逐位对照、完整模型旧新表、Rust 1.75 与全目标测试、Clippy、fmt通过并合并。主项目分别用全拼 60 例、九键 81 例保存完整报告和候选 dump，对接入后结果逐字节比较；不更新质量基线。固定模型与完整锁定资源，运行 engine/runtime 全测试、Clippy、全仓 fmt、笔记和 quick。构建终态后旧/新/新/旧串行运行默认 Runtime 九键逐键预算，每轮每臂 1791 键、预热 5 例，保留全部结果与构建监测；不从局部入口收益推断整机 16ms 通过。

## Risks

embedding 只是整体模型的一小段，本片即使局部提速也可能无法明显改变九键 p95。系统负载和构建进程监测不能证明独占机器；受干扰结果必须明确标注。没有修改文件 I/O、目录遍历、权限或 Android 资源，不把宿主编译说成设备验收。16ms 目标保持 active，直到默认真实路径确实通过。
