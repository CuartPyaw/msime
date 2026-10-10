package app.msime.android;


import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.view.ViewGroup;

/** Apple-compatible touch-keyboard spacing contract; input algorithms remain in Engine. */
public final class KeyboardGeometry {
    public static final int DEFAULT_HEIGHT_ADJUSTMENT_DP = 0;
    public static final int MIN_HEIGHT_ADJUSTMENT_DP = -12;
    public static final int MAX_HEIGHT_ADJUSTMENT_DP = 48;
    /** 所有布局的每一行键高，包括底行（123、中/英、空格、换行那一行）：26 键字母行、九键网格、笔画、手写区、大千四行整块和底行都按它排，每行另加一份行距，彼此切换时键盘总高不变。默认行距下四行共 4 × (52 + 7) = 236 dp，与原来三行 56 dp 键行加 46 dp 不带行距的底栏（235 dp）差 1 dp；底行和上面的键一样高，与 iOS、HarmonyOS 相同（#6354、#6472）。 */
    public static final int KEY_ROW_HEIGHT_DP = 52;
    /** 键区的行数：三行键（或一块占三行高的键块）加底行。键盘高度调整由这四行均分，见 {@link #bottomRowAdjustment}。 */
    public static final int KEYBOARD_ROW_COUNT = 4;

    /**
     * 底行分到的键盘高度调整（dp）：底行和上面三行一起均分调整量，余数按 {@link #adjustedRowHeight} 的规则给前面的行，底行是第四行，所以是 `floorDiv(调整量, 4)`。调整量先按 {@link #designHeightAdjustment} 钳制。
     *
     * <p>和 {@link #keyRowsAdjustment} 相加恰好是整份调整量，键盘总高与百分比的换算不变。
     */
    public static int bottomRowAdjustment(int adjustment) {
        return Math.floorDiv(designHeightAdjustment(adjustment), KEYBOARD_ROW_COUNT);
    }

    /** 底行以上的键行（三行字母、数字层的前三行、九键 / 笔画 / 手写这类三行高的键块）合起来分到的高度调整：整份调整量减去底行那一份（{@link #bottomRowAdjustment}）。 */
    public static int keyRowsAdjustment(int adjustment) {
        int value = designHeightAdjustment(adjustment);
        return value - bottomRowAdjustment(value);
    }

    /** Fixed candidate/shortcut row; swapping its contents must not move the key rows. */
    public static final int CANDIDATE_ROW_HEIGHT_DP = 48;
    public static final int NINE_KEY_HEIGHT_DP = 180;
    public static final int HANDWRITING_BODY_HEIGHT_DP = 220;
    /** 默认键距 6 dp、行距 7 dp，与共享偏好 client-core 的 `touch_key_spacing_tenths` / `touch_row_spacing_tenths` 默认值一致：共享层总会把这两个值写进偏好，这里另取一套会让新安装和「恢复默认」先画一种间距再跳回共享的那种。要改默认值得在共享层改，各平台一起变。 */
    public static final int DEFAULT_KEY_SPACING_TENTHS = 60;
    public static final int DEFAULT_ROW_SPACING_TENTHS = 70;
    public static final int MIN_KEY_SPACING_TENTHS = 30;
    public static final int MAX_KEY_SPACING_TENTHS = 60;
    public static final int MIN_ROW_SPACING_TENTHS = 40;
    public static final int MAX_ROW_SPACING_TENTHS = 100;

    // ---- 新设计的键盘几何（N/design-tokens.md §5） ----

    /** 新设计的标准键高（100% 时）。 */
    public static final int DESIGN_KEY_HEIGHT_DP = 46;
    /** 新设计的工具栏 / 候选行高度。 */
    public static final int DESIGN_TOOLBAR_ROW_HEIGHT_DP = 50;
    /** 新设计键盘外边距：上 8、左右 6、下 6。 */
    public static final int DESIGN_PADDING_TOP_DP = 8;
    public static final int DESIGN_PADDING_HORIZONTAL_DP = 6;
    public static final int DESIGN_PADDING_BOTTOM_DP = 6;
    /** 新设计的键距 5 dp、行距 8 dp。 */
    public static final int DESIGN_KEY_GAP_DP = 5;
    public static final int DESIGN_ROW_GAP_DP = 8;
    /** 键盘高度百分比的范围与默认值（内联高度条与设置页滑块 75%–160%）。上限原是 130%，竖屏手机上整块键盘只到屏幕的四成多，单手握持时拇指够不舒服（#5564），放宽到 160%，竖屏手机能调到屏幕一半以上；超出 130% 的部分另受窗口高度约束，见 {@link #windowHeightAdjustment}。 */
    public static final int MIN_HEIGHT_PERCENT = 75;
    public static final int MAX_HEIGHT_PERCENT = 160;
    public static final int DEFAULT_HEIGHT_PERCENT = 100;
    /** 百分比换算的基准：四行设计键高 4 × 46 dp。这是换算用的常数，不是此刻画出的键区高度（键行是 {@link #KEY_ROW_HEIGHT_DP} 另加行距）；调整量按它换算，所以 1% 仍是 1.84 dp。 */
    public static final int HEIGHT_PERCENT_BASE_DP = 184;
    /** 触屏键盘高度调整在新设计下的范围（dp）：75%–160% 换算为 −46…110。存在 Android 本地设置里，共享偏好的 `touch_keyboard_height_adjustment` 仍是 −12…48。 */
    public static final int MIN_DESIGN_HEIGHT_ADJUSTMENT_DP = -46;
    public static final int MAX_DESIGN_HEIGHT_ADJUSTMENT_DP = 110;
    /** 放宽前的上限（130% 对应的 55 dp）：不超过它的调整量在任何窗口里都照画，与放宽前相同。 */
    public static final int LEGACY_MAX_DESIGN_HEIGHT_ADJUSTMENT_DP = 55;
    /** 超出 {@link #LEGACY_MAX_DESIGN_HEIGHT_ADJUSTMENT_DP} 的调整量最多占窗口高度的这个百分比：竖屏手机（约 800 dp 高）能用满 160%，横屏手机（约 400 dp 高）止于 130%，不会把工具栏顶出窗口。 */
    public static final int WINDOW_HEIGHT_ADJUSTMENT_PERCENT = 14;

    private KeyboardGeometry() { }

    /** 键盘高度百分比对应的高度调整 dp：`round(184 × (p − 100) / 100)`，范围外先钳到 75–160（向远离零的方向取整）。 */
    public static int heightPercentToAdjustment(int percent) {
        int clamped = BoundsPolicy.bounded(percent, MIN_HEIGHT_PERCENT, MAX_HEIGHT_PERCENT);
        int scaled = HEIGHT_PERCENT_BASE_DP * (clamped - DEFAULT_HEIGHT_PERCENT);
        return (scaled + Integer.signum(scaled) * 50) / 100;
    }

    /** 高度调整 dp 对应的百分比：`round(100 + adjustment × 100 / 184)`；调整先钳到 −46…110。 */
    public static int heightAdjustmentToPercent(int adjustment) {
        int clamped = designHeightAdjustment(adjustment);
        int scaled = clamped * 100;
        return DEFAULT_HEIGHT_PERCENT
            + (scaled + Integer.signum(scaled) * HEIGHT_PERCENT_BASE_DP / 2) / HEIGHT_PERCENT_BASE_DP;
    }

    /** 新设计下的高度调整：缺省（{@link Integer#MIN_VALUE}）为 0，其余钳到 −46…110。 */
    public static int designHeightAdjustment(int value) {
        if (value == Integer.MIN_VALUE) return DEFAULT_HEIGHT_ADJUSTMENT_DP;
        return BoundsPolicy.bounded(value, MIN_DESIGN_HEIGHT_ADJUSTMENT_DP, MAX_DESIGN_HEIGHT_ADJUSTMENT_DP);
    }

    /**
     * 当前窗口里实际画出的高度调整：不超过 55 dp（130%）的照画；更高的部分最多到窗口高度的 {@link #WINDOW_HEIGHT_ADJUSTMENT_PERCENT}%，但不低于 55 dp。
     *
     * <p>存着的设置不变，旋转回竖屏或换到更高的窗口时自动恢复；窗口高度未知（0 或负数）时只画到 55 dp。
     *
     * @param adjustment 设置里的调整量（dp）
     * @param windowHeightDp 键盘所在窗口的可用高度（`Configuration.screenHeightDp`）
     */
    public static int windowHeightAdjustment(int adjustment, int windowHeightDp) {
        int value = designHeightAdjustment(adjustment);
        if (value <= LEGACY_MAX_DESIGN_HEIGHT_ADJUSTMENT_DP) return value;
        int windowLimit = windowHeightDp <= 0 ? 0
            : (int) ((long) windowHeightDp * WINDOW_HEIGHT_ADJUSTMENT_PERCENT / 100);
        return BoundsPolicy.atMost(value,
            BoundsPolicy.atLeast(windowLimit, LEGACY_MAX_DESIGN_HEIGHT_ADJUSTMENT_DP));
    }

    /** 某个高度百分比下的键高：`round(46 × p / 100)`。 */
    public static int designKeyHeight(int percent) {
        int clamped = BoundsPolicy.bounded(percent, MIN_HEIGHT_PERCENT, MAX_HEIGHT_PERCENT);
        return (DESIGN_KEY_HEIGHT_DP * clamped + 50) / 100;
    }

    /** 内联高度条上显示的百分比文字，如「100%」。 */
    public static String displayPercent(int percent) {
        return BoundsPolicy.bounded(percent, MIN_HEIGHT_PERCENT, MAX_HEIGHT_PERCENT) + "%";
    }

    public static int keySpacing(int value) {
        return clamp(value, MIN_KEY_SPACING_TENTHS, MAX_KEY_SPACING_TENTHS,
            DEFAULT_KEY_SPACING_TENTHS);
    }

    /** Widest key spacing the Dachen keyboard takes: eleven columns leave each key a tenth less width than the 26-key rows, and a 6 dp gap would take that out of the keycap itself. */
    public static final int ZHUYIN_MAX_KEY_SPACING_TENTHS = 40;

    /** The key spacing a touch layout draws with; the Dachen rows cap the setting rather than replace it, so a narrower gap the user picked still applies. */
    public static int layoutKeySpacing(int value, int touchLayout) {
        int spacing = keySpacing(value);
        return touchLayout == KeyboardLayout.ZHUYIN_LAYOUT
            ? BoundsPolicy.atMost(spacing, ZHUYIN_MAX_KEY_SPACING_TENTHS) : spacing;
    }

    public static int rowSpacing(int value) {
        return clamp(value, MIN_ROW_SPACING_TENTHS, MAX_ROW_SPACING_TENTHS,
            DEFAULT_ROW_SPACING_TENTHS);
    }

    public static int heightAdjustment(int value) {
        if (value == Integer.MIN_VALUE) return DEFAULT_HEIGHT_ADJUSTMENT_DP;
        return BoundsPolicy.bounded(value, MIN_HEIGHT_ADJUSTMENT_DP, MAX_HEIGHT_ADJUSTMENT_DP);
    }

    /**
     * Divide the total adjustment across rows without losing a density-independent pixel.
     *
     * <p>调整量按新设计的范围（−46…110）钳制。这里曾用共享偏好的 −12…48 钳制，设置里 126%–130% 与 75%–93% 画出来都一样高，滑块的两端是死区。
     */
    public static int adjustedRowHeight(int baseHeight, int adjustment, int rowCount, int rowIndex) {
        if (baseHeight <= 0 || rowCount <= 0 || rowIndex < 0 || rowIndex >= rowCount)
            throw new IllegalArgumentException("Invalid keyboard height geometry");
        int total = baseHeight * rowCount + designHeightAdjustment(adjustment);
        return total / rowCount + (rowIndex < total % rowCount ? 1 : 0);
    }

    public static String display(int tenths) {
        return NumberPolicy.decimal1(tenths / 10.0);
    }

    public static String displayHeight(int adjustment) {
        int value = heightAdjustment(adjustment);
        return (value > 0 ? "+" : "") + value;
    }

    public static int halfGapPixels(int tenths, float density) {
        if (!Float.isFinite(density) || density <= 0) return 0;
        return BoundsPolicy.nonNegative(Math.round(tenths * density / 20f));
    }

    private static int clamp(int value, int minimum, int maximum, int fallback) {
        if (value < 0) return fallback;
        return BoundsPolicy.bounded(value, minimum, maximum);
    }
}
