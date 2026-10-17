# Agent Note: Android 联系人导入

Status: implemented

## Problem

用户发送联系人手机号时需要离开输入法复制电话本内容，设置里也没有一个明确的导入、刷新和清空入口。

## Decision

在 Android 原生输入设置中增加联系人页。只有点击导入或刷新时申请 `READ_CONTACTS`，读取姓名和手机号后写入共享 mentions 名单；清空只删除输入法自己的名单，不触碰系统电话本。名单写入通过 `msime_client_plugins` 的 `save_mentions`，与输入进程共享并由引擎匹配。

## Alternatives considered

每次输入都直接查询系统电话本能得到最新数据，但会在每次输入时读取联系人并扩大隐私边界，也无法满足用户手动刷新要求，因此采用显式导入的本地快照。

## Consequences

权限请求只发生在用户操作时，输入进程不直接读取联系人。导入结果可被键盘和设置页共同使用；刷新会替换整份名单，清空不影响系统联系人。联系人来源图标仍需候选渲染层后续补充，当前共享 mentions 契约只携带文本和读音。
