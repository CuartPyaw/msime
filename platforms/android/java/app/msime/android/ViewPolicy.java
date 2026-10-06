package app.msime.android;

import android.view.Gravity;
import android.view.View;
import android.util.TypedValue;
import android.text.TextUtils;
import android.widget.Button;
import android.widget.LinearLayout;
import android.widget.TextView;

/** Shared view configuration for host controls whose widget defaults need resetting. */
public final class ViewPolicy {
    private ViewPolicy() {}

    /** Remove both legacy and platform minimum-height constraints from a view. */
    public static void clearMinimumHeight(View view) {
        view.setMinimumHeight(0);
    }

    /** Remove both minimum-height constraints from a text widget. */
    public static void clearMinimumHeight(TextView view) {
        view.setMinHeight(0);
        view.setMinimumHeight(0);
    }

    /** Apply a minimum height to a generic view. */
    public static void setMinimumHeight(View view, int height) {
        view.setMinimumHeight(height);
    }

    /** Apply a minimum height to both text-widget constraints. */
    public static void setMinimumHeight(TextView view, int height) {
        view.setMinHeight(height);
        view.setMinimumHeight(height);
    }

    /** Remove both legacy and platform minimum-width constraints from a view. */
    public static void clearMinimumWidth(View view) {
        view.setMinimumWidth(0);
    }

    /** Remove both minimum-width constraints from a text widget. */
    public static void clearMinimumWidth(TextView view) {
        view.setMinWidth(0);
        view.setMinimumWidth(0);
    }

    /** Apply a minimum width to a generic view. */
    public static void setMinimumWidth(View view, int width) {
        view.setMinimumWidth(width);
    }

    /** Apply a minimum width to both text-widget constraints. */
    public static void setMinimumWidth(TextView view, int width) {
        view.setMinWidth(width);
        view.setMinimumWidth(width);
    }

    /** Apply equal horizontal and vertical padding to a view. */
    public static void setSymmetricPadding(View view, int horizontal, int vertical) {
        view.setPadding(horizontal, vertical, horizontal, vertical);
    }

    /** Apply equal horizontal pixel padding while leaving vertical padding unset. */
    public static void setHorizontalPadding(View view, int horizontal) {
        view.setPadding(horizontal, 0, horizontal, 0);
    }

    /** Clear all view padding. */
    public static void clearPadding(View view) {
        view.setPadding(0, 0, 0, 0);
    }

    /** Clear vertical padding while preserving horizontal padding. */
    public static void clearVerticalPadding(View view) {
        view.setPadding(view.getPaddingLeft(), 0, view.getPaddingRight(), 0);
    }

    /** Set equal horizontal padding while preserving the current vertical padding. */
    public static void setHorizontalPaddingPreservingVertical(View view, int horizontal) {
        view.setPadding(horizontal, view.getPaddingTop(), horizontal, view.getPaddingBottom());
    }

    /** Keep a button label in its authored casing instead of applying the platform default. */
    public static void setAllCapsFalse(Button button) {
        button.setAllCaps(false);
    }

    /** Center a view's content on both axes. */
    public static void setCentered(View view) {
        ViewPolicy.setCentered(view);
    }

    /** Center a view's content along the vertical axis. */
    public static void setCenteredVertically(View view) {
        ViewPolicy.setCenteredVertically(view);
    }

    /** Center a view's content along the horizontal axis. */
    public static void setCenteredHorizontally(LinearLayout view) {
        view.setGravity(Gravity.CENTER_HORIZONTAL);
    }

    /** Align a view's content to the start edge and center it vertically. */
    public static void setStartCenteredVertically(LinearLayout view) {
        view.setGravity(Gravity.START | Gravity.CENTER_VERTICAL);
    }

    /** Align a view's content to the end edge and center it vertically. */
    public static void setEndCenteredVertically(View view) {
        view.setGravity(Gravity.END | Gravity.CENTER_VERTICAL);
    }

    /** Set a text view's size in scalable pixels. */
    public static void setTextSizeSp(TextView view, float sizeSp) {
        view.setTextSize(TypedValue.COMPLEX_UNIT_SP, sizeSp);
    }

    /** Remove a view's default background drawable. */
    public static void clearBackground(View view) {
        view.setBackground(null);
    }

    /** Limit a text view to a fixed number of lines and truncate at the end. */
    public static void setMaxLinesEllipsized(TextView view, int maxLines) {
        view.setMaxLines(maxLines);
        view.setEllipsize(TextUtils.TruncateAt.END);
    }

    /** Keep a text view on one line and truncate overflowing text at the end. */
    public static void setSingleLineEllipsized(TextView view) {
        view.setSingleLine(true);
        view.setEllipsize(TextUtils.TruncateAt.END);
    }

    /** Force a text view to occupy exactly the requested number of lines. */
    public static void setFixedLines(TextView view, int lines) {
        view.setMinLines(lines);
        view.setMaxLines(lines);
    }

    /** Configure uniform automatic text sizing with scalable-pixel bounds. */
    public static void setAutoSizeSp(TextView view, int minSp, int maxSp, int stepSp) {
        view.setAutoSizeTextTypeUniformWithConfiguration(minSp, maxSp, stepSp,
            TypedValue.COMPLEX_UNIT_SP);
    }

    /** Remove Android's extra font top and bottom padding from a text view. */
    public static void clearFontPadding(TextView view) {
        view.setIncludeFontPadding(false);
    }

    /** Make a view passive for touch and focus navigation. */
    public static void setNonInteractive(View view) {
        view.setClickable(false);
        view.setFocusable(false);
    }

    /** Remove the platform state-list animator from a view. */
    public static void clearStateListAnimator(View view) {
        view.setStateListAnimator(null);
    }

    /** Apply full opacity to an active view and a caller-selected opacity otherwise. */
    public static void setActiveAlpha(View view, boolean active, float inactiveAlpha) {
        view.setAlpha(active ? 1f : inactiveAlpha);
    }

    /** Apply minimum width and height constraints to a view. */
    public static void setMinimumSize(View view, int width, int height) {
        view.setMinimumWidth(width);
        view.setMinimumHeight(height);
    }

    /** Show a view in layout and rendering. */
    public static void show(View view) {
        view.setVisibility(View.VISIBLE);
    }

    /** Hide a view from layout and rendering. */
    public static void hide(View view) {
        view.setVisibility(View.GONE);
    }
}
