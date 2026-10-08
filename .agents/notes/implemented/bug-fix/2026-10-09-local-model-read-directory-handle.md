# Agent Note: 本地语音模型读取绑定目录句柄

Status: implemented

## Problem

本地语音模型安装、恢复和已安装状态检查中的若干归档、清单及模型文件读取，在路径检查后仍按路径打开。模型根目录或暂存目录被替换时，校验对象与读取对象可能不一致。

## Decision

所有剩余的模型文件读取点统一改用 `open_private_file_in`，覆盖收编记录、下载归档、收编后的模型文件、已安装清单、链接复制源和归档解包源；保留已有根目录、大小、哈希和归档路径校验。

## Alternatives considered

- 只重复符号链接检查：无法消除检查到打开之间的竞态。
- 改动模型发布、回滚和递归清理：这些流程已有目录句柄保护，扩大范围会增加安装恢复风险。

## Consequences

模型安装与读取相对于已确认的父目录执行，校验和发布语义保持不变。

## Verification

`cargo test -p msime-client-core --lib voice::local_models --locked --quiet`：53 passed、1 ignored。`cargo fmt --all`、`cargo clippy -p msime-client-core --lib --locked -- -D warnings` 和 `git diff --check` 均通过。
