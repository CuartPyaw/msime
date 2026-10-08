# Agent Note: 账户文件读取绑定目录句柄

Status: implemented

## Problem

账户头像上传和匿名会话 JSON 读取在路径检查后仍按路径打开文件。用户选择文件或本地会话目录被替换时，检查对象与读取对象可能不一致。

## Decision

两个读取点统一改用 `open_private_file_in`，保留原有大小、权限、普通文件和内容格式校验。

## Alternatives considered

- 只重复符号链接和权限检查：无法消除检查到打开之间的竞态。
- 修改会话写回流程：本次问题只涉及读取，写回流程另有 no-clobber 语义，避免混合变更。

## Consequences

账户头像和匿名会话读取相对于已确认的父目录执行，错误映射和隐私边界保持不变。

## Verification

`cargo test -p msime-client-core --lib account::anonymous --locked --quiet`：9 passed；`cargo test -p msime-client-core --lib account::tests --locked --quiet`：51 passed。头像模块无专属测试但随 workspace 编译通过。`cargo fmt --all`、`cargo clippy -p msime-client-core --lib --locked -- -D warnings` 和 `git diff --check` 均通过。
