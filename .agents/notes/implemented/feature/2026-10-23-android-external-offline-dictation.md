# Agent Note: Android 向外部调用方提供本地语音输入

Status: implemented

## Problem

#9159 希望 MacroDroid 的「语音输入」直接调用水杉的离线模型，并显示水杉娘的放大聆听耳部和随音量变化的红色信号。现有 `VoiceRecognitionActivity` 不导出，接收内部解析后的服务地址、凭据与文件路径，且把结果写入输入法待插入队列，不能直接作为外部接口。

## Decision

新增独立导出的 `OfflineVoiceRecognitionActivity`，声明 `android.speech.action.RECOGNIZE_SPEECH` 与 `DEFAULT`。仅从标准参数读取有界语言标签；本地配置、模型及运行库仍由应用自己读取并经既有共享规则解析。外部参数不能指定端点、凭据、文件路径或热词。复用 `LocalAsrRecognizer`，不调用系统识别、网络识别或云端润色。

当前共享移动配置按用户选择的识别服务解析；沿用该入口，明确要求在语音设置中选用「本地模型」。没有配置或准备不完整时显示原因及设置入口，不替用户改写服务配置。若后续需要独立于键盘服务的模型选择，须由共享 Rust 层新增本地配置解析接口，不能在 Java 再复制一套规则。

结果经 `RecognizerIntent.EXTRA_RESULTS` 返回调用者；提供标准 `EXTRA_RESULTS_PENDINGINTENT` 时按 Android 契约转交。取消、失败与空结果不含结果列表，每次调用最多投递一次。外部结果不进入 `VoiceResultStore`、剪贴板、日志或统计。录音仅在用户可见且授权后开始；返回、Home、销毁或重建时取消并释放，旧工作线程不得向新窗口投递结果。确认完成后停止采集并等待本机解码，取消则丢弃结果。权限弹窗仅暂停窗口时保留尚未开始的请求，`onStop` 在窗口真正不可见时取消未授权请求。识别器在模型及运行库准备的边界检查取消；`openRecorder`、`startRecording` 与 `stop`/`cancel` 共用启动锁，`cancel` 返回后不能再调用录音启动。句柄创建与原生取消共用另一把锁，两锁不交叉嵌套；模型冷加载不持有这两把锁。

聆听界面复用已打包的水杉娘透明贴纸，增加耳部放大表现和红色弧线。识别器在读取每个 PCM 块后报告有界的 RMS 音量，不保留额外录音、不写原始采样日志；旧监听器使用默认回调保持兼容。音量动画须由真实输入幅度驱动，不能用固定循环动画代替。

原生识别器与共享安装器接受相同的 Android 系统别名：仅允许 `/data/user/0` 指向 `/data/data`。Android 同时定义 `__linux__`，但 `/data/user` 属于 `system`，不能套用要求父目录属于 root 的 Linux 发行版链接规则。其余目录链接、模型末级链接、清单链接与多硬链接保护继续生效。

## Historical audit

- [Android 本地模型](../../implemented/feature/2026-10-12-android-local-voice-models.md)、[离线导入](../../implemented/feature/2026-10-12-android-offline-voice-import.md)：部分重叠，复用安装和选用规则，不新增模型安装器。
- [本地准备状态](../../implemented/bug-fix/2026-10-21-android-local-voice-readiness.md)：部分重叠，外部入口沿用模型与运行库均需准备的边界。
- [语音自动填入](../../implemented/bug-fix/2026-10-21-android-voice-auto-insert.md)：不同结果归属。键盘请求仍使用原队列，外部请求只回调用方，不修改原决定。
- [水杉娘贴纸](../../implemented/feature/2026-10-20-mobile-mascot-stickers.md)：复用单一素材源和现有打包路径，不另存一套位图。该笔记未单列素材授权，实施时继续核对原始引入记录。
- 检索活跃笔记中的 `RecognizerIntent`、`MacroDroid`、`VoiceRecognitionActivity`、本地语音与人物素材；其余平台的模型、文件句柄、设置和发布笔记不定义外部 Android 调用契约，没有需要废弃的旧提案。

## Alternatives considered

- 导出现有语音窗口：录音与界面复用最多。但其参数包括内部服务配置，结果进入输入法队列；外部调用与键盘请求的权限和结果归属不同，因此保留非导出边界。
- 实现 `RecognitionService`：能供 `SpeechRecognizer` 服务客户端使用。但报告者截图是应用选择器，且需求包含可见人物界面；先实现标准 Activity，并以真实 MacroDroid 实测确认，不能仅凭截图宣布互通。
- 新写一套录音和识别流程：外部生命周期可完全独立。但模型、热词、取消与运行库规则会重复；保留独立 Activity，复用已有识别器。
- 只提供接口并使用既有麦克风动画：实现更少。但遗漏原 issue 的人物、耳部和实时红色信号要求，不作为完整交付。

## Device verification coverage

- API 28 与 API 35 已通过标准隐式 Intent 发现、调用、一次性 PendingIntent 成功与取消回传、独立 UID 应用的 Activity 成功与取消回传、采集启动边界取消以及授权阶段 Home；原内部 Activity 仍不导出。
- 待设备验证：权限拒绝、无模型、缺运行库、旋转及重复调用；API 35 真实模型识别尚未通过。
- JVM 已验证：合成 PCM 的静音、幅度增长、符号对称、最大负数、部分缓冲与上限；API 28 真实合成音频已驱动完整识别与音量回调。
- API 28 已在飞行模式下用真实本地模型完成识别并返回请求者；API 35 仍待通过。API 28 与 API 35 的跨应用成功和取消调用均已确认语音待插入队列与系统剪贴板保持调用前的合成值。
- 待互通验证：安装官方 MacroDroid，执行其真实语音输入宏，确认返回字符串变量；标准测试调用器只能证明 Android 契约，不能替代此项。
- API 28 原生 APK 截图已确认人物、放大耳部和红色信号可见，设备用例确认信号由合成录音驱动；API 35 尚待完成。资源、生命周期与互通验证通过后才关闭 issue。

## Consequences

新增标准外部调用入口与人物实时音量界面，复用本地模型、凭据解析和运行库管理；外部调用增加独立的结果通道与生命周期，不改变键盘原有语音结果队列。宿主编译、JVM 与 API 28 的结果及取消边界已完成验证，API 28 的完整识别也已通过，API 35 与互通要求尚未完成；此提交不能作为关闭 issue 的依据。现阶段尚未取得并安装官方 MacroDroid，Activity 方案仍须互通确认。冷模型加载期间必须允许取消且不阻塞主线程；错误信息使用固定文案，不暴露路径、凭据或用户转写。现有录音有六十秒上限，外部参数不能扩大它。当前并行构建占用磁盘，新增 APK 构建前先确认剩余空间，不另建 Cargo 产物目录。

## References

- [Android RecognizerIntent](https://developer.android.com/reference/android/speech/RecognizerIntent)：标准 Activity、结果列表与 PendingIntent 契约。
- [报告者截图](https://github.com/metasequoiaime/msime/issues/9159#issuecomment-6314999955)：应用选择器证据，不能单独证明具体 action。
- [FUTO 固定提交参考](https://github.com/futo-org/voice-input/blob/d6e1eb2d139dc1a6342a4681c283686cca4bfceb/app/src/main/java/org/futo/voiceinput/RecognizeActivity.kt)：仅研究结果回传，不复制实现。

## Verification progress

- 已通过独立审查并修复授权阶段 Home 遗漏与取消后仍可能启动录音的竞态；修复复审未发现新阻塞。
- `LocalAsrCancellationSmoke` 初次实际执行以「创建原生会话之前取消不应调用 JNI」断言失败；修复后通过。补充用线程屏障覆盖资源准备期间取消；设备用例另在实际采集入口前暂停工作线程，取消后验证没有一次 `startRecording` 调用。
- PCM 与一次性会话 JVM 用例通过；Android 入口与结果归属契约脚本通过。先前完整 host 门禁通过 260 个 JVM 冒烟及 222 个设备源码编译；最终 host 门禁通过 261 个 JVM 冒烟及 223 个设备源码编译（日志 `issue-9159-host-final.log`）。设备用例另断言未授权、`started=false` 以及权限弹窗存在，避免已授权状态误通过。
- 设备测试 APK 与真实原生产品 APK 已成功构建并安装，完整本地 quick 已通过。API 28 已通过标准入口的 PendingIntent 成功、取消、采集启动前取消和授权阶段 Home；权限用例先因系统 Launcher/SystemUI ANR 受阻，专用模拟器重启后通过。API 35 与 MacroDroid 互通仍未通过；官方论坛索引的 MacroDroid beta 下载地址返回 404，未获得安装包。

- 模型验收使用固定目录中的中英流式归档，已在主机按锁定大小和 SHA-256 校验。`offline-prepare` 在设备内经共享模型导入及偏好保存接口准备环境；`offline-asr` 等待实际麦克风、音量视图和本地模型给出合成问候后点击产品完成按钮，检查标准结果。`offline-prepare` 已在 API 28 与 API 35 经真实共享安装器完成。模拟器 gRPC 只监听本机并使用令牌，实际宿主麦克风已关闭；测试音频由系统语音合成生成。

- API 28 的真实 JNI 返回固定错误 `Not an installed local speech model`，测试确认配置使用系统别名而非规范化路径；设备目录属主与 AOSP 初始化规则一致。`LocalSpeechHandleDeviceSmoke` 新增合成清单的私有目录回归，修复前实际失败，修复后通过，同时断言应用自建链接仍被拒绝。修复后真实模型已启动，合成麦克风音频驱动音量界面；随后发现并修复下面记录的中间结果空值语义问题。

- JNI 用 `null` 表示没有更新，原 Java 文本校验却将其变为空串，使中间结果刚显示又被下一块音频清空。`LocalAsrPolicySmoke` 的新回归实际失败后，文本校验保留 `null`；其余合法性、长度与控制字符检查继续执行。修复后全套 261 个 JVM 冒烟与 223 个设备源码编译通过；独立复审确认最终结果、热词纠错及数字转换的调用链仍保证非空引用，不引入空指针。
- 修复后的 API 28 `offline-asr` 通过真实麦克风、固定模型、音量视图、合成英文问候及一次性 PendingIntent 完整链路（`issue-9159-api28-asr-null-fixed.log`，`INSTRUMENTATION_CODE: -1`）。原生产品截图已确认人物、放大耳部和红色弧线可见。仅使用合成音频，宿主真实麦克风禁用；流按实际采集速度持续供给，在测试完成后结束。该证据不代替 API 35、实际 MacroDroid 或真实设备验收。
- 最终补丁的完整 quick 检出旧源码正则仍要求无锁创建录音器与句柄；更新该契约检查，要求取消锁内创建、启动失败释放录音器，以及句柄锁内销毁。定向检查通过，完整 quick 必须重新通过。API 35 飞行模式识别首轮未注入音频，直接注入重试时模拟器宿主进程以 139 退出；均未计为识别通过，重启专用模拟器后继续验收。
- Android 系统别名的原生回归已分别在 API 28 和 API 35 通过。API 35 的权限窗口实际存在，但原观察只查询活动辅助功能根；设备用例启用交互窗口检索并检查各系统权限窗口，仍保留未授权、`started=false`、弹窗可见及 Home 取消断言。更新后的用例分别在 API 28 与 API 35 通过（`issue-9159-api28-permission-windows.log`、`issue-9159-api35-permission-windows.log`），独立复审无新增阻塞。
- API 35 的音频注入使模拟器宿主崩溃，故障栈位于 `audio_forwarder_enable`；去掉 `-no-audio` 后仍发生，不能认定禁音参数是唯一原因。AOSP 的转发实现直接访问当前输入 voice，后续须先确认设备实际录音已启动再注入；尚未获得 API 35 真实识别通过的证据。仅记录固定符号与结果，不提交系统崩溃报告或模拟器令牌。
- 更新生命周期契约后的整轮 quick 通过（`issue-9159-quick-current.log`），Linux 原生 111 项通过；本分支 API 28 四组共享库设备单测合计 2034 项通过、0 失败。API 35 确认实际录音已启动后，默认后端注入仍使模拟器宿主崩溃；显式空音频后端避免崩溃，但尚未驱动产品音量信号，不能算识别通过。用户没有官方 MacroDroid APK，后续继续按代码与标准调用器核对契约，保留真实互通的验证缺口。

- 本分支 API 35 四组共享库设备单测也通过，合计 2034 项、0 失败（`issue-9159-api35-core-current.log`）；不把隔离子进程输出重复计数。
- 标准 Activity 结果增加独立 UID 的测试调用器：它自己发出 `startActivityForResult`，产品请求不带结果 PendingIntent；只有调用器的 `onActivityResult` 将实际结果码、原始结果字段和回调次数转发给本次测试专用的报告桥。测试检查成功列表精确相等、取消时完全没有 `EXTRA_RESULTS`，并尝试第二次产品回传以覆盖迟到结果覆盖。测试 APK 不访问模型和输入法存储。初版辅助功能观察在 SystemUI 无响应弹窗及空活动根处失败，临时诊断确认调用器实际已收到一次标准成功结果；改为读取调用器的实际回调报告，避免系统窗口状态混入结果协议断言，设备重跑结果另行记录。

- 独立 UID 调用器的 `offline-activity-result` 和 `offline-activity-cancel` 已分别在 API 28 与 API 35 通过，四项均返回 `INSTRUMENTATION_CODE: -1`（`issue-9159-emulator-5566-activity-relay.log`、`issue-9159-emulator-5580-activity-relay.log`）。本次测试专用桥经过独立复审：产品请求不含报告通道，桥只转发调用器实际收到的 Activity 结果，不合成成功数据。

- API 35 在显式空音频后端下另做无显式限速、`MODE_UNSPECIFIED` 注入对照，与 API 28 已通过的注入方式对齐；已确认实际 AudioFlinger 采集启动，注入进程正常退出，但测试仍因音量信号未响应而失败（`issue-9159-api35-asr-unpaced-orchestration.log`）。没有放宽识别时限，也未将命令退出码 0 当作通过；API 35 的真实识别缺口保留。

- API 28 当前原生产品 `smoke.sh emulator-5566 --core` 以 1 退出（`issue-9159-api28-core-smoke-current.log`）：短语上屏与删除、延迟批量组字、数字层三组通过，随后在组字放大镜的长拼写准备断言失败。没有精确本分支原始基线的同场对照，故不将该失败归为基线，完整装包验收仍未完成。
- 按用户要求继续代码核对：外部 Activity 只读取共享解析的本地模型，结果仅经 `setResult` 或请求方指定的 PendingIntent 返回；取消与迟到回调共用一次性终止状态。该入口没有 `VoiceResultStore` 或剪贴板写入调用，静态核对不能证明实际 MacroDroid 互通；设备前后状态断言的执行结果见下段。

- 跨应用 Activity 成功与取消两条用例增加实际存储边界断言：以目标 UID 的前台原生首页设置合成剪贴板及语音待插入值，调用后重新取得前台焦点，再检查队列 ID、文本与时间戳完全一致，系统剪贴板也未改变。随后恢复原剪贴板、消费本测试自己的合成队列并关闭观察窗口；遇到其它队列内容不覆盖。中断遗留的同一合成值可继续观察和清理。首轮 API 35 因 SystemUI/Launcher ANR 无焦点而失败，API 28 的 `startActivitySync` 因复用已有首页等待超时；改用独立任务的新首页实例并重启自有 API 35 后，最终源码在 API 28、API 35 上的成功与取消四项均通过、均返回 `INSTRUMENTATION_CODE: -1`（`issue-9159-emulator-5566-storage-boundary-raw.log`、`issue-9159-emulator-5580-storage-boundary-raw.log`）。没有把普通流输出的通过文案代替原始仪器化退出码；这只验证返回与存储边界，不新增 API 35 的真实 ASR 通过证据。

- 存储边界夹具以 `try-with-resources` 保留主体断言，清理失败作为 suppressed 异常；剪贴板恢复、按测试自身 UUID 消费队列及每个观察窗口结束分别尝试，外层仍强制停止独立调用方。独立复审确认此前清理顺序问题已解决，最终测试包重编通过，并在 API 28、API 35 重新运行成功与取消四项，原始仪器化结果全部为 `INSTRUMENTATION_CODE: -1`。
