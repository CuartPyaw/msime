package app.msime.android;

/**
 * How this host drives the Korean scheme, whose Engine is a Dubeolsik Hangul syllable automaton with no candidates.
 *
 * <p>The automaton belongs to the Engine. This host sends the QWERTY letters with their case, marks the composing Hangul inline, and inserts whatever a transition commits before it lets a declined key do its own work. It keeps no jamo composition table of its own.
 */
public final class KoreanInputPolicy {
    /** `SchemeType::Korean`: the value `View.scheme` and `commit_context.scheme` carry for this scheme. */
    public static final int KOREAN_SCHEME = 4;

    private KoreanInputPolicy() {}

    /** Whether the Engine is composing Hangul: the Korean scheme outside dedicated English. Korean has no local modes. */
    public static boolean active(int scheme, boolean dedicatedEnglish) {
        return scheme == KOREAN_SCHEME && !dedicatedEnglish;
    }

    /**
     * The text to mark inline in the editor.
     *
     * <p>For Korean that is the composing Hangul (`View.reading`, equal to `View.preedit`): `editing_text` only holds the key letters of the open syllable, so drawing it would put `gks` in the document where 한 belongs. `editing_text` still decides whether anything is composing, because it is non-empty exactly while a syllable is open. Korean never holds a phrase prefix. Every other scheme keeps the shared rule.
     */
    public static String composing(boolean korean, String phrasePrefix, String editingText,
                                   String reading) {
        if (!korean) return PhrasePreeditPolicy.composing(phrasePrefix, editingText);
        if (editingText == null || editingText.isEmpty() || reading == null) return "";
        return reading;
    }

    /**
     * The character a hardware key sends in Korean.
     *
     * <p>Shift decides the case and Caps Lock does not: the case of a Korean letter is not capitalisation but a different jamo (Shift+R is ㄲ), so a latched Caps Lock must not turn every ㄱ into ㄲ. Anything that is not an ASCII letter is passed through unchanged.
     */
    public static int hardwareCharacter(int unicode, boolean shift) {
        boolean letter = (unicode >= 'a' && unicode <= 'z') || (unicode >= 'A' && unicode <= 'Z');
        if (!letter) return unicode;
        return shift ? Character.toUpperCase(unicode) : Character.toLowerCase(unicode);
    }
}
