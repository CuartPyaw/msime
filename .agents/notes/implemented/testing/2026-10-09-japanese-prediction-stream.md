# Agent Note: 日文完整读音预测词条流式写入候选行

Status: implemented

## Problem

完整日文读音查询需要先显示句子转换结果，再显示读音前缀预测。`JapaneseProvider::query_into` 先把预测词条借用视图收集到临时 `Vec`，等待句子搜索结束后再逐行写入候选。这个向量只在同一次查询中消费，热查询因此保留一份不需要长期存在的结果容器。

## Decision

`JapaneseDictionary` 提供只消费一次的 `for_each_prefix_lemma_view` 内部入口。它调用原来的泛型 `prefix_lemmas_with`，访问闭包返回 `()`，因此结果类型是无需元素存储的 `Vec<()>`；不复制前缀短索引、下界扫描和成本排名逻辑。零大小元素不分配存储是 [Rust 标准库的 Vec 保证](https://doc.rust-lang.org/std/vec/struct.Vec.html#guarantees)。完整读音分支先运行句子搜索，再通过该入口把预测词条直接写入 `Rows`；待定读音和不满足预测长度的查询继续走原有路径，候选顺序和去重规则保持不变。

这个入口只暴露 crate 内部借用视图，闭包调用期间词库保持借用，最终候选仍由 provider 拥有。它不改变词库映射、预加载、模型生命周期或前缀筛选的边界。

## 既有笔记审计

[日文词条文本借用](2026-10-09-japanese-lemma-borrow.md) 已建立生产路径使用借用视图的边界，本篇在其基础上去掉完整读音预测的临时结果向量；两篇互相补充。[日文未命中排名存储](2026-10-09-japanese-empty-ranking.md) 继续约束空排名堆的延迟分配，本篇不改变排名算法。对活跃笔记检索 `for_each_prefix_lemma_view`、预测词条和完整读音流式消费，没有发现同一决定的 proposed 或 rejected 方案。

## Alternatives considered

- **继续收集 `Vec<JapaneseLemmaRef>`** — 代码路径直接且能一次计算预测长度，但向量只服务当前查询，并为每个完整读音保留一份临时容器。
- **把预测词条缓存到 provider 字段** — 可以复用容量，但视图生命周期绑定词库而 provider 同时持有词库，形成自引用存储边界，且缓存会把单次查询状态带入跨查询状态。
- **让句子搜索接收预测输出回调** — 能省掉一个入口，但会把句子排序和 provider 候选编排耦合在一起；当前实现只让词库结果构造支持流式消费，保留两层职责。

## Consequences

- **收益**：完整日文预测查询直接把借用词条写入已有候选行，充分预热的合成查询分配从 15 次降为 14 次，词条顺序和结果内容不变。
- **代价与已知上限**：完整读音候选缓冲按 16 条预测的上限预留空间，原实现按预测实际长度预留，因此少量或未命中预测可能保留更多行槽位；实际容量仍按 `Vec` 的增长规则决定。前缀排名仍需临时排名存储，句子搜索和最终候选仍按原路径拥有输出；该入口只减少预测视图结果向量，不代表完整日文查询零分配，也不据此宣称整段查询延迟改善。

## Verification

`prefix_lemma_views_can_be_consumed_without_result_vector` 断言三条合成预测被访问且只保留排名存储分配。70 条合成词库覆盖短索引与扫描、负成本、平局、空前缀、未命中和限额 0/1/2/24/64/65/70/100；保存的视图在临时查询释放后仍有效。`complete_model_query_streams_predictions` 固定完整 `kana` 查询的候选顺序，并对热查询完整行作一致性检查；恢复旧 provider 时以 15 次对 14 次的预算失败，流式消费后分配为 14 次。

相关日文 93 项测试通过，4 项计时测试保持忽略；完整 engine 1480 项单测和 31 项 golden 通过，Clippy、Rustfmt、差异与笔记检查通过。quick 门禁通过共享 Rust、Android 编译与 Java 检查、Linux 桌面与原生 73 项测试、macOS 原生构建和 Apple bridge；本机可运行的 Windows 102 项测试通过。Wasm、Windows 交叉构建与 HarmonyOS 真编译缺少工具链或准备输入，由脚本跳过，未做设备或系统输入法界面验收。
