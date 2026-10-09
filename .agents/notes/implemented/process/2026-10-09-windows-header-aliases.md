# Agent Note: Windows 镜像一次扫描创建头文件别名

Status: implemented

## Problem

Windows 交叉镜像在大小写敏感的 Linux 上为 MinGW 头文件添加首字母大写别名。现有循环为每个头文件启动 basename、cut、tr、ln 等外部进程；amd64 在 arm64 主机上仿真时，本轮真实构建的该层耗时 947.6 秒。别名规则很简单，逐文件创建多个进程与规则本身的工作量不相称。

## Decision

利用镜像已经安装的 Python，在一次目录扫描中筛选顶层 ASCII 小写开头的 `.h` 条目，直接创建首字母大写的相对符号链接。保留名称剩余部分和已有目标条目，跳过缺失目录和失效源链接，不递归处理子目录。脚本随 Docker 构建上下文复制并在完成后删除，镜像构建只启动一个 Python 进程。

## Alternatives considered

- 用 Bash 参数展开替换 basename/cut/tr：改动更小，但仍为每个新增别名启动 ln，且需把当前默认 sh 改为 Bash 才能使用大小写转换扩展。
- 只为 Windows.h 等已知拼写建别名：最少工作量，但无法维持当前覆盖所有 MinGW 头文件的兼容策略，其他引用会随编译路径变化遗漏。
- 外部 find/xargs 并行 ln：能并行一部分工作，但仍保留逐文件进程与名称解析管线；现成 Python 可以直接调用文件系统接口。

## Consequences

独立脚本增加一个构建输入，但不增加镜像依赖。已有条目包括失效目标符号链接均保留，避免覆盖；已验证的正常新旧目录清单完全一致。Dockerfile 改动会重新执行该层及后面的 Rust 安装层，系统依赖和线程选择层仍可复用。Python 文件系统错误直接使构建失败，不静默丢弃未完成别名。

[固定交叉工具链](2026-10-09-windows-cross-toolchain.md) 的旧验证记录给出 628.7 秒基线，本决定只接管头文件别名算法，不改变工具链和目标标准库归属。检索已有 implemented/proposed/rejected 没有独立别名算法笔记；Linux 组件和 Cargo 缓存笔记涉及其他构建成本，与本篇无关。

## Verification

实现前在 Linux 容器运行新增 CLI 回归，4 项因扫描脚本不存在而失败。实现后 5 项合成文件系统测试通过，包含相对链接、保留已有有效/失效别名、失效源、顶层 ASCII 名称规则、目录和源链接、缺失/非目录、多目录与幂等。本机文件系统不区分大小写，测试明确跳过；Linux 容器中的临时目录区分大小写，实际执行测试。新版 amd64 镜像的别名 RUN 实际执行 `1.9s`，创建 `4413` 项，没有使用该层缓存；完整镜像构建退出 0。同一 Debian 基镜像摘要 `sha256:a29215f6a35e51e22adffa17f89e9d2ef06214e64a2bad10d765c46aea49f11f`、同一系统依赖及线程选择层的旧版 alias RUN 实际执行 `947.6s`，也未命中该层缓存。测量来自共享 arm64 主机上的 amd64 仿真，当时主机负载较高，仅记录本轮层耗时，不承诺其他主机的固定加速比例。旧版完整镜像构建退出 0，随后在两个断网新容器中采集三个 include 目录的完整顶层清单：共享目录 `2996` 项、i686 目录 `2997` 项、x86_64 目录 `2996` 项。名称、条目类型、符号链接目标以及普通文件 SHA256 逐项对照完全一致，比较退出 0。

新版断网 amd64 容器再次执行 5 项合成文件系统回归，退出 0。两个 MinGW C++ 编译器实际编译包含 `<Windows.h>` 和 `<Winuser.h>` 的合成源码，分别链接为 `pei-i386` 和 `pei-x86-64`。`bash platforms/windows/tests/tools/check-cross-image-toolchain.sh <image>` 退出 0，固定 Cargo `1.97.1`、Rustfmt `1.9.0-stable`、Clippy `0.1.97` 和两个已安装 GNU 标准库均可断网使用，合成 Rust 程序也完成双架构链接。探针只编译链接，不运行 PE，不替代完整 Windows 产品构建、Wine 或系统输入法验收。

`bash scripts/verify-local.sh --quick` 退出 0：102 项 Windows 主机测试通过，2 项按主机差异排除、59 项需要 Windows 构建、1 项不是测试；Rust workspace 和共享 Apple bridge 编译通过。Windows x86 语法、原生和 pipe-only 门禁因当前主机缺少 MinGW 或既有 x64 构建而跳过；桌面 workspace 包因 macOS 资源未准备跳过。Android、Wasm、HarmonyOS、Linux 与 macOS 原生阶段按 scope 跳过。

`pnpm run verify-notes` 与 `git diff --check` 通过。CI 中另有不使用 Dockerfile 的独立别名循环，本切片没有改动它。
