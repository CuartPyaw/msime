# Agent Note: 账户词库快照读取绑定目录句柄

Status: implemented

## Problem

云端词库恢复接口读取用户选择的本地快照时，在路径检查后仍按路径打开文件。快照父目录被替换时，检查对象与上传对象可能不一致。

## Decision

恢复快照文件读取改用 `open_private_file_in`，保留绝对路径、父目录、普通文件和大小边界校验。

## Alternatives considered

- 只重复 `symlink_metadata`：无法消除检查到打开之间的竞态。
- 同时修改下载到文件的原子发布：那是独立的流式写入问题，避免混合改变上传读取语义。

## Consequences

恢复操作相对于已确认的快照父目录读取文件，网络请求和响应校验保持不变。

## Verification

`cargo test -p msime-client-core --lib account::tests --locked --quiet`：51 passed。`cargo fmt --all`、`cargo clippy -p msime-client-core --lib --locked -- -D warnings` 和 `git diff --check` 均通过。
