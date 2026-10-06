import app.msime.android.KeyboardLayout;
import app.msime.android.KoreanKeyboardLayout;
import java.util.List;

public final class KoreanKeyboardLayoutSmoke {
    private static void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    public static void main(String[] args) {
        // The standard Dubeolsik layout over the same QWERTY rows the other schemes use.
        List<List<String>> rows = KeyboardLayout.rows(KeyboardLayout.Layer.LETTERS);
        check(faces(rows.get(0), false).equals(List.of(
                "ㅂ", "ㅈ", "ㄷ", "ㄱ", "ㅅ", "ㅛ", "ㅕ", "ㅑ", "ㅐ", "ㅔ")),
            "top row jamo");
        check(faces(rows.get(1), false).equals(List.of(
                "ㅁ", "ㄴ", "ㅇ", "ㄹ", "ㅎ", "ㅗ", "ㅓ", "ㅏ", "ㅣ")),
            "home row jamo");
        check(faces(rows.get(2), false).equals(List.of("ㅋ", "ㅌ", "ㅊ", "ㅍ", "ㅠ", "ㅜ", "ㅡ")),
            "bottom row jamo");
        check(faces(rows.get(0), true).equals(List.of(
                "ㅃ", "ㅉ", "ㄸ", "ㄲ", "ㅆ", "ㅛ", "ㅕ", "ㅑ", "ㅒ", "ㅖ")),
            "Shift shows the five double consonants and ㅒ ㅖ, and leaves the rest of the row alone");
        check(faces(rows.get(1), true).equals(faces(rows.get(1), false))
                && faces(rows.get(2), true).equals(faces(rows.get(2), false)),
            "Shift changes nothing below the top row");

        // A key sends its ASCII letter; uppercase only where Shift changes the jamo.
        for (List<String> row : rows) {
            for (String key : row) {
                char letter = key.charAt(0);
                check(KoreanKeyboardLayout.input(letter, false) == letter,
                    "an unshifted key sends its lowercase letter");
            }
        }
        for (char letter : "qwertop".toCharArray()) {
            check(KoreanKeyboardLayout.input(letter, true) == Character.toUpperCase(letter),
                "a shifted double-consonant key sends its uppercase letter");
        }
        for (char letter : "yuiasdfghjklzxcvbnm".toCharArray()) {
            check(KoreanKeyboardLayout.input(letter, true) == letter,
                "a shifted key without a shifted jamo sends its lowercase letter");
        }
        check(KoreanKeyboardLayout.input('R', false) == 'r',
            "the letter a key carries is canonicalised before Shift is applied");

        check("韩文 ㄲ".equals(KoreanKeyboardLayout.accessibilityLabel("r", true))
                && "韩文 ㄱ".equals(KoreanKeyboardLayout.accessibilityLabel("r", false)),
            "the spoken label is the jamo the key types now");
        check(";".equals(KoreanKeyboardLayout.face(";", false))
                && "".equals(KoreanKeyboardLayout.face(null, false)),
            "keys without a jamo keep their own face");
        check(!KoreanKeyboardLayout.SHIFT_LABEL.contains("英文"),
            "Shift on the Korean keycaps is not described as the way into English");
        System.out.println("Android Korean keyboard: Dubeolsik keycaps, Shift faces and sent letters passed");
    }

    private static List<String> faces(List<String> keys, boolean shifted) {
        return keys.stream().map(key -> KoreanKeyboardLayout.face(key, shifted)).toList();
    }
}
