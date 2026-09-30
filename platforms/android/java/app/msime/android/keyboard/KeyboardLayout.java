package app.msime.android;

import java.util.List;

/** The two keyboard layers shared by the Android view and its host-side tests. */
public final class KeyboardLayout {
    public enum Layer { LETTERS, SYMBOLS }

    public static final int STANDARD_TOUCH_LAYOUT = 0;
    public static final int QUANPIN_NINE_KEY_LAYOUT = 1;
    public static final int JAPANESE_NINE_KEY_LAYOUT = 2;
    public static final int HANDWRITING_LAYOUT = 3;
    /** The 26 QWERTY keys drawn with Dubeolsik jamo keycaps; they still send the ASCII letters. */
    public static final int KOREAN_LAYOUT = 4;

    private static final List<List<String>> LETTER_ROWS = List.of(
        List.of("q", "w", "e", "r", "t", "y", "u", "i", "o", "p"),
        List.of("a", "s", "d", "f", "g", "h", "j", "k", "l"),
        List.of("z", "x", "c", "v", "b", "n", "m")
    );
    private static final List<List<String>> SYMBOL_ROWS = List.of(
        List.of("1", "2", "3", "4", "5", "6", "7", "8", "9", "0"),
        List.of(",", ".", "?", "!", ";", ":", "'", "\"", "@", "/"),
        List.of("(", ")", "[", "]", "<", ">", "\\", "-", "_", "=")
    );

    private KeyboardLayout() {}

    /** Resolves the host surface from the Engine view without conflating Japanese nine-key. */
    public static int resolveTouchLayout(boolean handwriting, boolean nineKey, int scheme,
                                         String touchLayout) {
        // Korean has only the 26-key Dubeolsik keyboard, whatever layout the document carries.
        if (scheme == KoreanInputPolicy.KOREAN_SCHEME) return KOREAN_LAYOUT;
        if (handwriting || "handwriting".equals(touchLayout)) return HANDWRITING_LAYOUT;
        if (scheme == 3 && "nine_key".equals(touchLayout)) return JAPANESE_NINE_KEY_LAYOUT;
        if (nineKey) return QUANPIN_NINE_KEY_LAYOUT;
        return STANDARD_TOUCH_LAYOUT;
    }

    /**
     * Whether the surface has a Shift state of its own: English case on the standard rows, the double consonants and ㅒ ㅖ on the Korean rows. Every other surface resets it.
     */
    public static boolean carriesLetterCase(int touchLayout) {
        return touchLayout == STANDARD_TOUCH_LAYOUT || touchLayout == KOREAN_LAYOUT;
    }

    /**
     * The keys a layer sends, always in their canonical form.
     *
     * <p>Letters stay lowercase here because this is what reaches the input engine, and the engine
     * only starts a pinyin composition from a lowercase letter. What the user sees is a separate
     * question answered by {@link LetterKeyFacePolicy}: a Chinese keyboard draws its 26 keys in
     * caps without any of them being a capital letter. This returned uppercase alongside the face
     * once, which sent `N` to the engine, got it declined, and committed the letter literally --
     * Chinese input produced `NIHAO` instead of 你好.
     */
    public static List<List<String>> rows(Layer layer) {
        return layer == Layer.SYMBOLS ? SYMBOL_ROWS : LETTER_ROWS;
    }
}
