package app.msime.android;

import android.graphics.Bitmap;

/** Shared bitmap geometry for host image-processing flows. */
public final class BitmapPolicy {
    private BitmapPolicy() {}

    /** Scale a bitmap down proportionally when its longest edge exceeds the requested limit. */
    public static Bitmap scaleToEdge(Bitmap bitmap, int maximumEdge) {
        if (bitmap == null || maximumEdge <= 0) return bitmap;
        int edge = Math.max(bitmap.getWidth(), bitmap.getHeight());
        if (edge <= maximumEdge) return bitmap;
        float scale = maximumEdge / (float) edge;
        Bitmap scaled = Bitmap.createScaledBitmap(bitmap,
            Math.max(1, Math.round(bitmap.getWidth() * scale)),
            Math.max(1, Math.round(bitmap.getHeight() * scale)), true);
        if (scaled != bitmap) bitmap.recycle();
        return scaled;
    }
}
