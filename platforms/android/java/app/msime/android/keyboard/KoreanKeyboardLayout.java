package app.msime.android;

import java.util.Map;

/**
 * Dubeolsik keycaps for the 26 QWERTY keys.
 *
 * <p>These are labels only. Each key still sends its ASCII letter, and the Engine turns letters into jamo and jamo into syllables; the keycap is what the user needs to see to know which letter to press. Shift shows the five double consonants and ㅒ ㅖ, and sends those seven keys in uppercase; every other key keeps its jamo under Shift and sends its lowercase letter, which types the same jamo.
 */
public final class KoreanKeyboardLayout {
    private static final Map<Character, String> JAMO = Map.ofEntries(
        Map.entry('q', "ㅂ"), Map.entry('w', "ㅈ"), Map.entry('e', "ㄷ"), Map.entry('r', "ㄱ"),
        Map.entry('t', "ㅅ"), Map.entry('y', "ㅛ"), Map.entry('u', "ㅕ"), Map.entry('i', "ㅑ"),
        Map.entry('o', "ㅐ"), Map.entry('p', "ㅔ"), Map.entry('a', "ㅁ"), Map.entry('s', "ㄴ"),
        Map.entry('d', "ㅇ"), Map.entry('f', "ㄹ"), Map.entry('g', "ㅎ"), Map.entry('h', "ㅗ"),
        Map.entry('j', "ㅓ"), Map.entry('k', "ㅏ"), Map.entry('l', "ㅣ"), Map.entry('z', "ㅋ"),
        Map.entry('x', "ㅌ"), Map.entry('c', "ㅊ"), Map.entry('v', "ㅍ"), Map.entry('b', "ㅠ"),
        Map.entry('n', "ㅜ"), Map.entry('m', "ㅡ"));
    private static final Map<Character, String> SHIFTED_JAMO = Map.of(
        'q', "ㅃ", 'w', "ㅉ", 'e', "ㄸ", 'r', "ㄲ", 't', "ㅆ", 'o', "ㅒ", 'p', "ㅖ");

    /** Accessibility label for the Shift key while the Korean keycaps are on screen. */
    public static final String SHIFT_LABEL = "双辅音";

    private KoreanKeyboardLayout() {}

    /** The jamo drawn on a letter key; anything that is not a lowercase ASCII letter keeps its own face. */
    public static String face(String letter, boolean shifted) {
        if (letter == null || letter.length() != 1) return letter == null ? "" : letter;
        char key = letter.charAt(0);
        String doubled = shifted ? SHIFTED_JAMO.get(key) : null;
        if (doubled != null) return doubled;
        return JAMO.getOrDefault(key, letter);
    }

    /** The ASCII letter a key sends: uppercase only for the seven keys whose shifted jamo differs. */
    public static char input(char letter, boolean shifted) {
        char key = Character.toLowerCase(letter);
        return shifted && SHIFTED_JAMO.containsKey(key) ? Character.toUpperCase(key) : key;
    }

    /** Spoken label: the jamo the key types now. */
    public static String accessibilityLabel(String letter, boolean shifted) {
        String face = face(letter, shifted);
        return face.isEmpty() ? "韩文字母" : "韩文 " + face;
    }
}
