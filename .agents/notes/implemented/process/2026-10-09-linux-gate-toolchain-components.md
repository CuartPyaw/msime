# Agent Note: Linux 门禁镜像预装工具链组件

Status: implemented

## Problem

仓库的 `rust-toolchain.toml` 要求 `rustfmt` 和 `clippy`。两个 Linux 门禁镜像实际只装有 `cargo`、`rustc` 和 `rust-std`，缺少这两个组件。门禁每次创建临时容器，Cargo 首次启动时会按声明补装；删除容器也删除这次安装，下一次门禁再次下载。

## Decision

在原生构建和桌面检查的 Dockerfile 中显式安装两个组件，沿用固定 Rust 镜像的默认工具链，把组件保存在镜像构建缓存里。在现有 apt 层之后安装，保留已经缓存的系统开发包层。

`tests/tools/check-image-toolchain.sh` 在断网的新容器里只读挂载真实仓库，执行 `cargo`、`cargo fmt` 和 `cargo clippy` 的版本命令。缺失组件或工具链版本与仓库声明不匹配时无法临时下载，应当失败；修复后的两个门禁镜像均须通过。

## Alternatives considered

- 从仓库工具链声明中去掉组件：会消除当前补装动作，但开发者不再自动取得格式和 Clippy 工具，削弱原有工具链约束。
- 把 `rustup` 目录另设持久化挂载：能跨运行复用安装，但新增可变缓存、挂载规则与清理责任，而固定版本的组件适合直接存入既有镜像层。
- 把安装放在两个 apt 层之前：可以让两镜像共享组件层，但会使现有大体积 apt 缓存失效并重装系统包。本切片先复用既有系统依赖层。

## Consequences

升级仓库 Rust 版本或新增工具链组件时，需要同步镜像准备步骤；断网回归以真实工具链文件为依据，能暴露不匹配。镜像增加组件占用，不引入新的构建目录或共享可变缓存。

断网自检用于镜像构建或工具链升级后的定向回归，不在每次 quick 中另启预检容器。日常门禁仍按原来的两条编译路径执行，避免为减少组件下载而增加容器启动开销。

检索现有流程、测试、提案和拒绝笔记，没有镜像组件准备的既有归属。Linux 测试构建开关笔记约束 CMake 目标生成，与本篇的镜像准备互不替代。

## Verification

修复前的两个镜像在断网容器中首次 Cargo 调用均尝试下载工具链清单，并因 DNS 不可用失败；镜像组件列表均只有 `cargo`、`rustc` 和 `rust-std`。预装后的两个镜像均在断网的新容器中成功执行 Cargo、Rustfmt、Clippy 的六条版本命令，两个回归进程均以 0 退出。两个镜像重建时的原有 apt 层均命中缓存；原生镜像定向重复构建和桌面门禁重复构建的组件安装层均命中缓存，没有再次下载安装组件。

`bash scripts/verify-local.sh --quick` 通过：workspace 与 Android Rust 编译、Linux 桌面编译、原生 73 项 CTest、五笔生产构建及两版本安装/链接隔离、日文登记和共享 Apple bridge 编译。Linux 两条实际 Cargo 构建日志没有工具链同步或组件下载。Android 桌面库仍有原有的 11 条警告；Windows 本机测试、Wasm、Android Java 和 macOS 原生阶段按改动范围跳过，Windows 交叉构建缺少 MinGW，HarmonyOS 设置依赖和真编译准备输入缺失。没有执行设备或真实输入法框架验收。Shell 语法、笔记校验和 `git diff --check` 通过。
