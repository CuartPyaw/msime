package app.msime.android;

/** Bounds transient Engine diagnostics before they reach the keyboard surface. */
public final class InputDiagnosticPolicy {
    public static final long DISMISS_DELAY_MILLIS = 4_000L;
    public static final int MAX_LENGTH = 1_024;

    private InputDiagnosticPolicy() {}

    public static String normalize(String value) {
        if (value == null) return "";
        String normalized = value.trim();
        if (normalized.isEmpty()) return "";
        if (normalized.length() <= MAX_LENGTH) return normalized;
        int end = MAX_LENGTH - 1;
        // 避免截断 emoji 时把孤立的高代理项带入诊断提示。
        if (end > 0 && Character.isHighSurrogate(normalized.charAt(end - 1))) end--;
        return normalized.substring(0, end) + "…";
    }

    public static boolean visible(String value) {
        return !normalize(value).isEmpty();
    }
}
