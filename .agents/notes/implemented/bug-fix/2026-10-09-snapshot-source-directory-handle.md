# Agent Note: 快照队列源文件绑定目录句柄

Status: implemented

## Problem

快照入队在源文件类型和符号链接检查后仍按路径打开源文件。用户提供的快照父目录被替换时，哈希计算和复制内容可能来自不同对象。

## Decision

入队源文件读取改用 `open_private_file_in`，保留绝对路径、普通文件、大小上限和 SHA-256 校验。

## Alternatives considered

- 只重复 `symlink_metadata`：无法消除检查到打开之间的竞态。
- 修改队列内目标临时文件发布：这是独立的流式原子发布问题，本次只修源读取。

## Consequences

快照入队的哈希和复制相对于已确认的源父目录读取，队列状态与冲突语义保持不变。

## Verification

`cargo test -p msime-client-core --lib cloud::snapshot_queue --locked --quiet`：12 passed。`cargo fmt --all`、`cargo clippy -p msime-client-core --lib --locked -- -D warnings` 和 `git diff --check` 均通过。
