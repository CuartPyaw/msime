# Agent Note: 皮肤文件夹导入读取绑定目录句柄

Status: implemented

## Problem

皮肤文件夹导入在检查目录项类型后仍按路径读取文件。用户选中的源目录被替换时，类型检查对象与复制对象可能不一致。

## Decision

导入复制树中的文件读取改用 `open_private_file_in`，保留原有大小预算、目录深度和目标暂存目录逻辑。

## Alternatives considered

- 只再次检查 `file_type`：无法消除检查到读取之间的竞态。
- 重写整个递归复制器：当前问题只涉及叶文件打开，扩大范围会影响导入回滚。

## Consequences

皮肤导入复制相对于已确认的源父目录读取普通文件，现有导入规则和失败清理保持不变。

## Verification

`cargo test -p msime-client-core --lib skin::folder_import --locked --quiet`：12 passed。`cargo fmt --all`、`cargo clippy -p msime-client-core --lib --locked -- -D warnings` 和 `git diff --check` 均通过。
