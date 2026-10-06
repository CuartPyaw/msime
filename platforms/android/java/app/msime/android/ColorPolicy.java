package app.msime.android;

import android.graphics.Color;

/** Shared parsing for optional Android theme and skin colours. */
public final class ColorPolicy {
    private ColorPolicy() {}

    /** Multiply a colour's existing alpha by the supplied factor. */
    public static int withAlpha(int color, float alpha) {
        int base = Color.alpha(color);
        return (color & 0x00FFFFFF) | (Math.round(base * alpha) << 24);
    }

    /** Return the fallback when the value is missing, empty, or not a valid Android colour. */
    public static int parse(String value, int fallback) {
        if (value == null || value.isEmpty()) return fallback;
        try {
            return Color.parseColor(value);
        } catch (IllegalArgumentException error) {
            return fallback;
        }
    }
}
