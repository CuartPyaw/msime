package app.msime.android;

/** Host-boundary Simplified/Traditional output policy without Android dependencies. */
public final class ChineseOutputPolicy {
    @FunctionalInterface
    public interface Converter {
        String convert(String text);
    }

    private ChineseOutputPolicy() {}

    /** Japanese (3) and Korean (4) text is not Chinese, so it is never converted. */
    public static boolean applies(boolean dedicatedEnglish, int scheme, String localMode) {
        return !dedicatedEnglish && scheme != 3 && scheme != KoreanInputPolicy.KOREAN_SCHEME
            && !"temporary_japanese".equals(localMode);
    }

    public static String output(String text, boolean traditional, boolean applies,
                                Converter converter) {
        if (!traditional || !applies || text.isEmpty()) return text;
        try {
            String converted = converter.convert(text);
            return converted == null ? text : converted;
        } catch (RuntimeException | LinkageError error) {
            return text;
        }
    }
}
