package app.msime.android;

import android.content.res.ColorStateList;
import android.content.Context;
import android.graphics.drawable.Drawable;
import android.view.View;
import android.widget.ImageView;
import androidx.annotation.DrawableRes;
import androidx.annotation.ColorInt;

/** Android 图片控件共用的状态与外观策略。 */
public final class ImageViewPolicy {
    private ImageViewPolicy() {}

    /** 通过平台颜色状态列表为图片应用单色着色。 */
    public static void setTint(ImageView view, @ColorInt int color) {
        view.setImageTintList(ColorStateList.valueOf(color));
    }

    /** Create an accessibility-hidden decorative image with a theme tint. */
    public static ImageView decorative(Context context, @DrawableRes int icon,
                                       @ColorInt int tint) {
        ImageView view = decorative(context, icon);
        setTint(view, tint);
        return view;
    }

    /** Create an accessibility-hidden decorative image from a resource. */
    public static ImageView decorative(Context context, @DrawableRes int icon) {
        ImageView view = new ImageView(context);
        view.setImageResource(icon);
        hideFromAccessibility(view);
        return view;
    }

    /** Create an accessibility-hidden decorative image from a runtime drawable. */
    public static ImageView decorative(Context context, Drawable icon) {
        ImageView view = new ImageView(context);
        view.setImageDrawable(icon);
        hideFromAccessibility(view);
        return view;
    }

    private static void hideFromAccessibility(View view) {
        view.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
    }
}
