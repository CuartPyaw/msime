# Agent Note: Android 向外部调用方提供本地语音输入

Status: implemented

## Problem

#9159 希望 MacroDroid 的「语音输入」直接调用水杉的离线模型，并显示水杉娘的放大聆听耳部和随音量变化的红色信号。现有 `VoiceRecognitionActivity` 不导出，接收内部解析后的服务地址、凭据与文件路径，且把结果写入输入法待插入队列，不能直接作为外部接口。

## Decision

新增独立导出的 `OfflineVoiceRecognitionActivity`，声明 `android.speech.action.RECOGNIZE_SPEECH` 与 `DEFAULT`。仅从标准参数读取有界语言标签；本地配置、模型及运行库仍由应用自己读取并经既有共享规则解析。外部参数不能指定端点、凭据、文件路径或热词。复用 `LocalAsrRecognizer`，不调用系统识别、网络识别或云端润色。

当前共享移动配置按用户选择的识别服务解析；沿用该入口，明确要求在语音设置中选用「本地模型」。没有配置或准备不完整时显示原因及设置入口，不替用户改写服务配置。若后续需要独立于键盘服务的模型选择，须由共享 Rust 层新增本地配置解析接口，不能在 Java 再复制一套规则。

结果经 `RecognizerIntent.EXTRA_RESULTS` 返回调用者；提供标准 `EXTRA_RESULTS_PENDINGINTENT` 时按 Android 契约转交。取消、失败与空结果不含结果列表，每次调用最多投递一次。外部结果不进入 `VoiceResultStore`、剪贴板、日志或统计。录音仅在用户可见且授权后开始；返回、Home、销毁或重建时取消并释放，旧工作线程不得向新窗口投递结果。确认完成后停止采集并等待本机解码，取消则丢弃结果。权限弹窗仅暂停窗口时保留尚未开始的请求，`onStop` 在窗口真正不可见时取消未授权请求。识别器在模型及运行库准备的边界检查取消；`openRecorder`、`startRecording` 与 `stop`/`cancel` 共用启动锁，`cancel` 返回后不能再调用录音启动。句柄创建与原生取消共用另一把锁，两锁不交叉嵌套；模型冷加载不持有这两把锁。

聆听界面复用已打包的水杉娘透明贴纸，增加耳部放大表现和红色弧线。识别器在读取每个 PCM 块后报告有界的 RMS 音量，不保留额外录音、不写原始采样日志；旧监听器使用默认回调保持兼容。音量动画须由真实输入幅度驱动，不能用固定循环动画代替。

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

## Device verification requirements

- API 28 与 35 上能通过标准隐式 Intent 发现并调用；原内部 Activity 仍不导出。
- 覆盖权限拒绝、无模型、缺运行库、取消、Home、旋转、重复调用、一次性结果及 PendingIntent 的成功和失败语义。
- 合成 PCM 覆盖静音、幅度增长、符号对称、最大负数、部分缓冲与上限，音量回调不改变识别结果。
- 飞行模式下用真实本地模型完成识别，结果只返回请求者；检查待插入队列和剪贴板没有新增结果。
- 安装官方 MacroDroid，执行其真实语音输入宏，确认返回字符串变量；标准测试调用器只能证明 Android 契约，不能替代此项。
- 人物、放大耳部和红色信号在原生 APK 中可见，信号随录音音量变化；资源、生命周期与设备验证通过后才关闭 issue。

## Consequences

新增标准外部调用入口与人物实时音量界面，复用本地模型、凭据解析和运行库管理；外部调用增加独立的结果通道与生命周期，不改变键盘原有语音结果队列。当前仅宿主编译与 JVM 层完成验证，设备及互通要求尚未完成；此提交不能作为关闭 issue 的依据。现阶段尚未取得并安装官方 MacroDroid，Activity 方案仍须互通确认。冷模型加载期间必须允许取消且不阻塞主线程；错误信息使用固定文案，不暴露路径、凭据或用户转写。现有录音有六十秒上限，外部参数不能扩大它。当前并行构建占用磁盘，新增 APK 构建前先确认剩余空间，不另建 Cargo 产物目录。

## References

- [Android RecognizerIntent](https://developer.android.com/reference/android/speech/RecognizerIntent)：标准 Activity、结果列表与 PendingIntent 契约。
- [报告者截图](https://github.com/metasequoiaime/msime/issues/9159#issuecomment-6314999955)：应用选择器证据，不能单独证明具体 action。
- [FUTO 固定提交参考](https://github.com/futo-org/voice-input/blob/d6e1eb2d139dc1a6342a4681c283686cca4bfceb/app/src/main/java/org/futo/voiceinput/RecognizeActivity.kt)：仅研究结果回传，不复制实现。

## Verification progress

- 已通过独立审查并修复授权阶段 Home 遗漏与取消后仍可能启动录音的竞态；修复复审未发现新阻塞。
- `LocalAsrCancellationSmoke` 初次实际执行以「创建原生会话之前取消不应调用 JNI」断言失败；修复后通过。补充用线程屏障覆盖资源准备期间取消；设备用例另在实际采集入口前暂停工作线程，取消后验证没有一次 `startRecording` 调用。
- PCM 与一次性会话 JVM 用例通过；Android 入口与结果归属契约脚本通过。先前完整 host 门禁通过 260 个 JVM 冒烟及 222 个设备源码编译；最终 host 门禁通过 261 个 JVM 冒烟及 223 个设备源码编译（日志 `issue-9159-host-final.log`）。设备用例另断言未授权、`started=false` 以及权限弹窗存在，避免已授权状态误通过。
- 设备测试 APK 已成功构建，API 28 专用模拟器已启动；完整本地 quick 已通过，真实原生产品 APK 仍在构建。API 28/35 用例、离线模型与 MacroDroid 互通均未计为通过。官方论坛索引的 MacroDroid beta 下载地址返回 404，未获得安装包。
