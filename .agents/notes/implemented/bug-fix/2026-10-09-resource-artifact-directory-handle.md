# Agent Note: 资源文件验证读取绑定目录句柄

Status: implemented

## Problem

资源代次验证在清单检查后仍按路径读取已列出的资源文件。资源目录被替换时，验证对象与实际哈希读取对象可能不一致。

## Decision

资源文件验证读取改用 `open_private_file_in`，保留锁文件清单、大小和 SHA-256 校验。

## Alternatives considered

- 只重复文件类型检查：无法消除检查到打开之间的竞态。
- 修改资源代次发布流程：当前问题只涉及验证读取，避免影响安装事务。

## Consequences

资源完整性验证相对于已确认的资源父目录读取普通文件，现有错误和哈希语义保持不变。

## Verification

`cargo test -p msime-client-core --lib resources --locked --quiet`：27 passed。`cargo fmt --all`、`cargo clippy -p msime-client-core --lib --locked -- -D warnings` 和 `git diff --check` 均通过。
