import android.speech.SpeechRecognizer;
import app.msime.android.PlatformSpeechPolicy;

/** 系统语音识别服务的错误提示、转交识别窗口的条件与音量换算（#5553）。 */
public final class PlatformSpeechPolicySmoke {
    static void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    public static void main(String[] args) {
        // 每个错误码的提示都不同于旧的那句「语音识别未返回结果」，并且带着错误码，反馈时能对上号。
        int[] errors = {
            SpeechRecognizer.ERROR_NETWORK_TIMEOUT, SpeechRecognizer.ERROR_NETWORK,
            SpeechRecognizer.ERROR_AUDIO, SpeechRecognizer.ERROR_SERVER, SpeechRecognizer.ERROR_CLIENT,
            SpeechRecognizer.ERROR_SPEECH_TIMEOUT, SpeechRecognizer.ERROR_NO_MATCH,
            SpeechRecognizer.ERROR_RECOGNIZER_BUSY, SpeechRecognizer.ERROR_INSUFFICIENT_PERMISSIONS,
            SpeechRecognizer.ERROR_TOO_MANY_REQUESTS, SpeechRecognizer.ERROR_SERVER_DISCONNECTED,
            SpeechRecognizer.ERROR_LANGUAGE_NOT_SUPPORTED, SpeechRecognizer.ERROR_LANGUAGE_UNAVAILABLE,
            SpeechRecognizer.ERROR_CANNOT_CHECK_SUPPORT, 99,
        };
        for (int error : errors) {
            String message = PlatformSpeechPolicy.message(error);
            check(message.endsWith("（错误码 " + error + "）"), "message carries error " + error);
            check(!message.startsWith("语音识别未返回结果"), "message names a reason for " + error);
        }
        check(PlatformSpeechPolicy.message(SpeechRecognizer.ERROR_NETWORK).contains("网络"), "network errors say network");
        check(PlatformSpeechPolicy.message(SpeechRecognizer.ERROR_NETWORK_TIMEOUT)
                .startsWith(PlatformSpeechPolicy.message(SpeechRecognizer.ERROR_NETWORK).replace("（错误码 2）", "")),
            "both network errors share one explanation");
        check(PlatformSpeechPolicy.message(SpeechRecognizer.ERROR_INSUFFICIENT_PERMISSIONS).contains("麦克风"),
            "a permission error points at the microphone permission");
        check(PlatformSpeechPolicy.message(SpeechRecognizer.ERROR_SPEECH_TIMEOUT).contains("没有听到声音"),
            "a speech timeout says nothing was heard");
        check(PlatformSpeechPolicy.message(99).startsWith("系统语音识别失败"), "an unknown code still says what failed");

        // 只有还没开始聆听、并且是调用方一侧被拒时才转交识别窗口；已经在听之后的失败直接提示。
        check(PlatformSpeechPolicy.retryInActivity(SpeechRecognizer.ERROR_CLIENT, false),
            "a client error before listening retries in the activity");
        check(PlatformSpeechPolicy.retryInActivity(SpeechRecognizer.ERROR_INSUFFICIENT_PERMISSIONS, false),
            "a permission refusal before listening retries in the activity");
        check(PlatformSpeechPolicy.retryInActivity(SpeechRecognizer.ERROR_SERVER_DISCONNECTED, false),
            "a dropped service connection before listening retries in the activity");
        check(!PlatformSpeechPolicy.retryInActivity(SpeechRecognizer.ERROR_CLIENT, true),
            "once listening, the activity would fail the same way");
        check(!PlatformSpeechPolicy.retryInActivity(SpeechRecognizer.ERROR_NETWORK, false)
                && !PlatformSpeechPolicy.retryInActivity(SpeechRecognizer.ERROR_NO_MATCH, false)
                && !PlatformSpeechPolicy.retryInActivity(SpeechRecognizer.ERROR_SPEECH_TIMEOUT, false),
            "network and no-speech errors are reported, not retried");

        check(PlatformSpeechPolicy.level(-10f) == 0f && PlatformSpeechPolicy.level(Float.NaN) == 0f,
            "quiet or missing values are silence");
        check(PlatformSpeechPolicy.level(4f) == 0.5f, "the middle of the range is half");
        check(PlatformSpeechPolicy.level(30f) == 1f, "loud values are capped");
        check(PlatformSpeechPolicy.smoothed(0.2f, 0.9f) == 0.9f, "rising volume is followed at once");
        float falling = PlatformSpeechPolicy.smoothed(0.8f, 0f);
        check(falling > 0f && falling < 0.8f, "falling volume decays rather than dropping");
        check(PlatformSpeechPolicy.smoothed(0.5f, Float.NaN) < 0.5f && PlatformSpeechPolicy.smoothed(0f, 5f) == 1f,
            "the shown level stays within 0-1");
        System.out.println("Android platform speech policy passed");
    }
}
