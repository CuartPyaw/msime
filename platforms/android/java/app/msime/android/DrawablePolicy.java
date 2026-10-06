package app.msime.android;

import android.graphics.Color;
import android.graphics.drawable.GradientDrawable;

/** Shared drawable factories for host UI surfaces. */
public final class DrawablePolicy {
    private DrawablePolicy() {}

    public static GradientDrawable circle(int color) {
        GradientDrawable shape = new GradientDrawable();
        shape.setShape(GradientDrawable.OVAL);
        shape.setColor(color);
        return shape;
    }

    public static GradientDrawable rounded(int color, float radiusPx) {
        GradientDrawable shape = new GradientDrawable();
        shape.setShape(GradientDrawable.RECTANGLE);
        shape.setColor(color);
        shape.setCornerRadius(radiusPx);
        return shape;
    }

    public static GradientDrawable rounded(int color, float[] radii) {
        GradientDrawable shape = new GradientDrawable();
        shape.setShape(GradientDrawable.RECTANGLE);
        shape.setColor(color);
        shape.setCornerRadii(radii);
        return shape;
    }

    public static GradientDrawable outlined(float radiusPx, int strokeWidth, int strokeColor) {
        GradientDrawable shape = rounded(Color.TRANSPARENT, radiusPx);
        shape.setStroke(Math.max(1, strokeWidth), strokeColor);
        return shape;
    }
}
