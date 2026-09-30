import app.msime.android.EditorBridge;
import app.msime.android.KoreanInputPolicy;
import java.util.ArrayList;
import java.util.List;

public final class KoreanInputPolicySmoke {
    private static void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    public static void main(String[] args) {
        check(KoreanInputPolicy.KOREAN_SCHEME == 4, "the shared Engine ordinal for Korean is 4");
        check(KoreanInputPolicy.active(4, false), "Korean scheme composes Hangul");
        check(!KoreanInputPolicy.active(4, true), "dedicated English is not Korean input");
        check(!KoreanInputPolicy.active(3, false) && !KoreanInputPolicy.active(0, false),
            "other schemes are not Korean");

        // The inline composition is the Hangul, never the key letters editing_text holds.
        check("녕".equals(KoreanInputPolicy.composing(true, "", "sud", "녕")),
            "Korean marks the composing syllable");
        check("".equals(KoreanInputPolicy.composing(true, "", "", "")),
            "nothing composing marks nothing");
        check("".equals(KoreanInputPolicy.composing(true, "", "", "안")),
            "editing_text decides whether a syllable is open");
        check("".equals(KoreanInputPolicy.composing(true, "", "gk", null)),
            "a missing reading is bounded to empty text");
        check("你好nihao".equals(KoreanInputPolicy.composing(false, "你好", "nihao", "")),
            "other schemes keep the shared phrase-prefix rule");

        // Shift decides the case; Caps Lock does not.
        check(KoreanInputPolicy.hardwareCharacter('R', false) == 'r',
            "Caps Lock alone types the plain consonant");
        check(KoreanInputPolicy.hardwareCharacter('r', true) == 'R'
                && KoreanInputPolicy.hardwareCharacter('R', true) == 'R',
            "Shift types the double consonant");
        check(KoreanInputPolicy.hardwareCharacter('1', true) == '1'
                && KoreanInputPolicy.hardwareCharacter(',', false) == ','
                && KoreanInputPolicy.hardwareCharacter(0, false) == 0,
            "non-letters pass through");

        // "dkssud": the fourth key finishes 안 and opens ㄴ in the same transition. The commit has to
        // land before the new syllable is marked, or the marked region would swallow it.
        List<String> writes = new ArrayList<>();
        EditorBridge bridge = new EditorBridge();
        EditorBridge.Sink sink = sink(writes);
        check(bridge.apply(sink, null, KoreanInputPolicy.composing(true, "", "dks", "안")),
            "the open syllable is marked");
        check(bridge.apply(sink, "안", KoreanInputPolicy.composing(true, "", "s", "ㄴ")),
            "an auto-committed syllable and the next composition apply together");
        // Space: handled=false, the syllable is committed and the composition emptied; the host then inserts the space.
        check(bridge.apply(sink, "녕", KoreanInputPolicy.composing(true, "", "", "")),
            "a syllable-ending key commits and clears the mark");
        check(writes.equals(List.of("begin", "compose:안", "end",
                "begin", "commit:안", "compose:ㄴ", "end",
                "begin", "commit:녕", "end")),
            "writes are ordered commit first, then the new mark: " + writes);
        System.out.println("Android Korean input: inline Hangul, hardware case and commit order passed");
    }

    private static EditorBridge.Sink sink(List<String> writes) {
        return new EditorBridge.Sink() {
            public void begin() { writes.add("begin"); }
            public boolean commit(String text) { writes.add("commit:" + text); return true; }
            public boolean compose(String text) { writes.add("compose:" + text); return true; }
            public boolean finish() { writes.add("finish"); return true; }
            public void end() { writes.add("end"); }
        };
    }
}
