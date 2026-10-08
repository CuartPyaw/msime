# Agent Note: 模糊拼音伙伴引用栈缓冲

Status: implemented

## Problem

`pinyin::fuzzy::with_partners` 每次为初声或韵母收集少量字符串引用时都会创建临时 `Vec`。这些引用只在当前 `fuzzy_syllables` 调用期间使用，初声最多 6 个伙伴，韵母最多 5 个伙伴。

## Decision

使用按伙伴表长度加一的 const 泛型数组保存引用，并返回有效元素数量。`fuzzy_syllables` 只把有效切片用于笛卡尔积，结果字符串仍由原有结果向量拥有。数组容量由实际规则表确定，不引入运行时输入上限。

## Alternatives considered

继续返回 `Vec` 并只减少预留容量仍会为每次模糊展开创建堆状态，不能消除短路径的临时分配。

使用固定的单一最大数组会让不相关的规则表承担更大的栈帧；按表长度生成数组能保持边界与数据定义一致。

## Verification

新增伙伴引用分配预算测试；旧路径产生 1 次临时分配，栈数组路径为 0 次。模糊拼音 4 项测试、`msime-engine` 全量 1362 项单测、31 项 golden、doc tests、Clippy 和 Rustfmt 覆盖行为与接入路径。

## Consequences

模糊拼音短路径不再为初声和韵母引用建立堆向量，结果顺序、去重和完整音节判断保持不变；每次调用在栈上保留两个很小的引用数组。
