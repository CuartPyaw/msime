# Agent Note: 皮肤目录文件读取绑定目录句柄

Status: implemented

## Problem

皮肤清单、图片尺寸检查和资源加载在路径约束检查后仍按路径打开文件。皮肤目录或资源路径被替换时，校验对象与读取对象可能不一致。

## Decision

三个皮肤文件读取点统一改用 `open_private_file_in`，保留现有路径包含、符号链接、大小和图片尺寸验证。

## Alternatives considered

- 只重复 `canonicalize` 或元数据检查：仍不能消除检查到打开之间的竞态。
- 重新设计皮肤目录扫描：当前问题只涉及叶文件读取，扩大范围会影响已有皮肤兼容行为。

## Consequences

皮肤解析和资源加载相对于确认过的父目录打开普通文件，错误映射和资源限制保持不变。

## Verification

`cargo test -p msime-client-core --lib skin::catalog --locked --quiet`：41 passed。`cargo fmt --all`、`cargo clippy -p msime-client-core --lib --locked -- -D warnings` 和 `git diff --check` 均通过。
