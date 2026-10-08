# Agent Note: 插件导入文件绑定目录句柄

Status: implemented

## Problem

插件导入在复制目录文件、读取压缩包和移除目标位置的杂项文件时，仍有路径型打开或删除。源目录或目标目录被替换时，检查对象与实际操作对象可能不一致。

## Decision

复制和压缩包读取改用 `open_private_file_in`，目标杂项文件删除改用 `remove_private_file`，保留原有导入校验、原子目录替换和错误映射。

## Alternatives considered

- 只重复 `symlink_metadata`：无法覆盖检查到打开/删除之间的竞态。
- 改动整个插件目录替换流程：问题只涉及文件叶节点，扩大范围会增加导入回滚风险。

## Consequences

插件导入的文件读取和清理相对于已确认的父目录执行，符号链接拒绝和插件包格式规则保持不变。

## Verification

`cargo test -p msime-client-core --lib plugins::tests --locked --quiet`：49 passed。`cargo fmt --all`、`cargo clippy -p msime-client-core --lib --locked -- -D warnings` 和 `git diff --check` 均通过。
