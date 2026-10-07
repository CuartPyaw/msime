# Agent Note: Android 系统语音识别留在键区里聆听

Status: implemented

## Problem

#5553：vivo X60（Android 11）上长按空格开始语音输入，键盘收起，整屏只剩一层灰色蒙层，约 4 秒后弹出「语音识别未返回结果」。报告人没有配置任何识别服务商（诊断包里 `voice_input.asr_provider` 是默认的 `doubao`，没有密钥），所以走的是设备自带的系统识别服务（SpeechRecognizer），而这条路在键盘里一律交给 `VoiceRecognitionActivity`：

- 识别窗口是对话框主题、系统识别器那条路不设内容视图，屏幕上只剩对话框的灰色蒙层，看不到任何「正在录音」的提示；窗口抢走焦点，键盘随之收起。
- `onError` 不管错误码是什么都报同一句「语音识别未返回结果」，诊断包也不记语音事件，用户和维护者都分不出是网络、权限、没听到声音还是服务本身不可用。录屏里状态栏的麦克风指示亮了约 4 秒，说明识别服务确实在录音，失败发生在服务一侧。
- 结果只存进跨进程的语音结果交接，要再到语音结果面板里点「插入」，而本机模型和豆包已经在键区里聆听并直接上屏。

这台设备上系统识别服务具体为什么失败，诊断包里没有可用的信息，这次没法定位；能修的是我们这一侧把错误吞掉、把用户晾在灰色蒙层里的那部分。

## Decision

- `ImeVoiceEntry.choose` 多一种 `Engine.PLATFORM`：没有本机模型和豆包、离线回退也不成立、没有配置上传式服务商、`SpeechRecognizer.isRecognitionAvailable` 为真时，系统识别服务在键区里聆听，和本机模型、豆包共用 `VoiceListeningView`。识别结果直接经 `commitText(…, TypingSource.VOICE)` 上屏；聆听期间输入位置变了才存进语音结果，与另外两种引擎相同。配置了润色时在工作线程上润色再上屏。
- 键区里的系统识别带 `EXTRA_PARTIAL_RESULTS`，边听边把识别出的文字显示在提示行，但不像本机模型和豆包那样按 1.5 秒停顿自动结束：系统识别服务自己判断说完了没有。再按一次语音键是 `stopListening`，点面板是取消。
- `onRmsChanged` 经 `PlatformSpeechPolicy.level` 换成 0–1 的音量，`VoiceListeningView.setLevel` 在麦克风圆盘外画一圈随音量涨落的光圈（最多 12 dp，变大立即跟上、变小按 0.75 回落），原来的 1.2 秒脉冲环保留。
- 错误提示统一由 `PlatformSpeechPolicy.message` 给出，每个错误码一句具体原因，结尾带「（错误码 N）」；键盘和识别窗口都用它。
- 键区里还没报 `onReadyForSpeech` 就失败、并且是 `ERROR_CLIENT`、`ERROR_INSUFFICIENT_PERMISSIONS`、`ERROR_SERVER_DISCONNECTED` 这类调用方一侧被拒时，`MSIMEInputService.launchVoiceActivity` 用原来的识别窗口再试一次；已经开始聆听之后的失败直接提示，不重试。没有麦克风权限时仍交给识别窗口申请。
- 识别窗口里的系统识别器也显示「正在录音」和「完成」「取消」，不再是只有灰色蒙层的空白对话框。设置应用的语音面板（`VoicePlugin`）也走这个窗口。
- 识别请求带 `EXTRA_CALLING_PACKAGE`；两份清单（原生宿主和 Tauri 壳）在 `RECOGNIZE_SPEECH` 之外声明对 `android.speech.RecognitionService` 的包可见性，`check-host.sh` 守着这一条。

## Alternatives considered

- **只把错误码翻译清楚，继续走识别窗口** — 改动最小，下一份反馈就能带回错误码；但灰色蒙层、键盘收起、结果还要手动插入这几个问题都还在，评论里要的录音动画也只能画在一个抢焦点的对话框里。
- **识别窗口改成透明、不变暗的全屏 Activity，在里面画麦克风动画** — 不用改输入法进程；但窗口照样抢焦点、键盘照样收起，动画画在宿主应用上方而不是键盘里，和本机模型、豆包两种引擎的体验不一致。
- **全部改走识别窗口之外的自有录音（AudioRecord + 本机模型）** — 不再依赖设备的识别服务；但本机模型要先下载运行库和模型，几百 MB，不能当作没配置任何服务时的默认。

## Consequences

- **收益**：默认配置下的语音输入不再离开键盘，有录音和音量动画，识别完直接上屏；失败时提示里有具体原因和错误码，下次反馈能直接对上 SpeechRecognizer 的错误。
- **代价与已知上限**：在输入法服务里直接创建 SpeechRecognizer 依赖键盘可见时输入法进程拥有前台的麦克风能力，本机模型和豆包已经依赖同一点；个别识别服务若只接受前台 Activity 发起的请求，第一次会在开始聆听前被拒，再转交识别窗口，多一次往返。音量光圈只对系统识别服务生效，本机模型和豆包的录音在各自的识别器里，没有接音量。这次没有 vivo 真机，键区里的系统识别、音量光圈和转交识别窗口都只有 JVM 冒烟覆盖纯逻辑，没有真机验收。#5553 设备上系统识别服务失败的真正原因仍未知，要等报告人用新版本给出错误码。

## Verification

`bash platforms/android/check-host.sh`：`tests/voice/PlatformSpeechPolicySmoke.java`（错误提示、转交条件、音量换算）、`tests/keyboard/ms_w4_kpb_KeyboardExtrasSmoke.java`（`choose` 选出 `PLATFORM`、上传式服务商和没有识别服务时仍交给识别窗口、离线回退优先）、`tests/keyboard/ms_w2_kb_ViewLogicSmoke.java`（音量光圈有界），以及清单的 `RecognitionService` 守卫。共享设置文案的改动由 `apps/desktop/tests/settings/voice-settings-copy.test.tsx` 覆盖。
