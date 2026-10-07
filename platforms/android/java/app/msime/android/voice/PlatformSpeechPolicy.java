package app.msime.android;

import android.speech.SpeechRecognizer;

/**
 * 系统语音识别服务（SpeechRecognizer）的纯逻辑：错误码翻译成用户看得懂、反馈时也能对上号的提示，键盘里没起来时要不要改走识别窗口，以及 onRmsChanged 的分贝值换成聆听面板的音量。
 *
 * <p>只用 SpeechRecognizer 的 int 常量（编译期内联），不调用任何 Android API，JVM 冒烟可以直接跑。以前所有错误都报同一句「语音识别未返回结果」，用户和诊断包里都看不出是网络、权限、没听到声音还是服务本身不可用（#5553），所以提示里一律带上错误码。
 */
public final class PlatformSpeechPolicy {
    /** onRmsChanged 在常见实现里大致落在 -2–10 dB：低于下限当作安静，高于上限当作满格。 */
    static final float QUIET_RMS_DB = -2f;
    static final float LOUD_RMS_DB = 10f;
    /** 音量回落时每次保留上一帧的比例：声音一大立刻跟上，变小时缓缓落下，光圈不会一闪一闪。 */
    static final float LEVEL_RELEASE = 0.75f;

    private PlatformSpeechPolicy() {}

    /** 一个错误码对应的提示，结尾带「（错误码 N）」。 */
    public static String message(int error) {
        String reason = switch (error) {
            case SpeechRecognizer.ERROR_NETWORK_TIMEOUT, SpeechRecognizer.ERROR_NETWORK ->
                "系统语音识别服务连不上网络，请检查网络；部分设备自带的识别服务在当前网络下无法使用";
            case SpeechRecognizer.ERROR_SERVER, SpeechRecognizer.ERROR_SERVER_DISCONNECTED ->
                "系统语音识别服务出错，请稍后重试";
            case SpeechRecognizer.ERROR_AUDIO -> "系统语音识别服务无法录音，麦克风可能被其他应用占用";
            case SpeechRecognizer.ERROR_SPEECH_TIMEOUT -> "没有听到声音，请靠近麦克风再试";
            case SpeechRecognizer.ERROR_NO_MATCH -> "没有识别出文字，请说得清楚些再试";
            case SpeechRecognizer.ERROR_RECOGNIZER_BUSY -> "系统语音识别服务正忙，请稍后重试";
            case SpeechRecognizer.ERROR_INSUFFICIENT_PERMISSIONS ->
                "系统语音识别服务没有麦克风权限，请在系统设置里给默认语音识别应用开启麦克风";
            case SpeechRecognizer.ERROR_TOO_MANY_REQUESTS -> "系统语音识别服务请求过于频繁，请稍后重试";
            case SpeechRecognizer.ERROR_LANGUAGE_NOT_SUPPORTED, SpeechRecognizer.ERROR_LANGUAGE_UNAVAILABLE ->
                "系统语音识别服务不支持所选语言，请在语音设置里换一种识别语言";
            case SpeechRecognizer.ERROR_CLIENT -> "系统语音识别服务无法启动";
            default -> "系统语音识别失败";
        };
        return reason + "（错误码 " + error + "）";
    }

    /**
     * 键盘里直接调起的识别在开始聆听（onReadyForSpeech）之前就失败、而且是调用方一侧的原因（服务拒绝了这个进程、服务连接断开、权限被判为不足）时，改用识别窗口再试一次：窗口是个前台 Activity，原先的流程就是它，有的识别服务只认前台界面发起的请求。已经开始聆听之后的失败（网络、没听到声音等）换个窗口也一样，直接报给用户。
     */
    public static boolean retryInActivity(int error, boolean listening) {
        if (listening) return false;
        return error == SpeechRecognizer.ERROR_CLIENT
            || error == SpeechRecognizer.ERROR_INSUFFICIENT_PERMISSIONS
            || error == SpeechRecognizer.ERROR_SERVER_DISCONNECTED;
    }

    /** onRmsChanged 的分贝值换成 0–1 的音量；NaN 当作安静。 */
    public static float level(float rmsDb) {
        if (Float.isNaN(rmsDb)) return 0f;
        float scaled = (rmsDb - QUIET_RMS_DB) / (LOUD_RMS_DB - QUIET_RMS_DB);
        return Math.max(0f, Math.min(1f, scaled));
    }

    /** 下一帧显示的音量：变大直接跟上，变小按 {@link #LEVEL_RELEASE} 回落。 */
    public static float smoothed(float shown, float next) {
        float target = Math.max(0f, Math.min(1f, Float.isNaN(next) ? 0f : next));
        if (target >= shown) return target;
        return shown * LEVEL_RELEASE + target * (1f - LEVEL_RELEASE);
    }
}
