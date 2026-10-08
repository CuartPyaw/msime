# Agent Note: 社区皮肤清单读取绑定目录句柄

Status: implemented

## Problem

社区皮肤打包、添加预览和添加许可证时，清单文件经过路径约束后仍按路径打开。皮肤目录被替换时，约束检查与读取对象可能不一致。

## Decision

三个清单读取点统一改用 `open_private_file_in`，保留现有清单大小、UTF-8、TOML 和包完整性校验。

## Alternatives considered

- 只重复 `symlink_metadata`：无法消除检查到打开之间的竞态。
- 同时重写清单原子发布流程：写入流程涉及预览回滚和目录替换，超出本次叶文件读取修复范围。

## Consequences

社区皮肤清单读取相对于已确认的父目录执行，打包和编辑操作的现有回滚语义不变。

## Verification

`cargo test -p msime-client-core --lib skin::candidate_community --locked --quiet`：51 passed。`cargo fmt --all`、`cargo clippy -p msime-client-core --lib --locked -- -D warnings` 和 `git diff --check` 均通过。
