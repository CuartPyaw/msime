package app.msime.android.home;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.graphics.Rect;
import android.graphics.RectF;
import android.os.Bundle;
import android.util.AttributeSet;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.View;
import androidx.annotation.NonNull;
import androidx.annotation.Nullable;
import androidx.core.content.ContextCompat;
import androidx.core.view.ViewCompat;
import androidx.core.view.accessibility.AccessibilityNodeInfoCompat;
import androidx.customview.widget.ExploreByTouchHelper;
import app.msime.android.KeyPressIds;
import app.msime.android.R;
import app.msime.android.TypingStatisticsModel;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;

/**
 * 按键热力图：each key of the soft keyboard filled by how often it was pressed in the page's scope.
 *
 * <p>The soft 26-key layout is always drawn, since it is the keyboard most presses go through; the nine-key grid joins it only when a nine-key cell was pressed, so a 26-key user is not shown an empty second keyboard. Keys that have no drawn position (a hardware keyboard's Tab or F1, or the nine-key cells when the grid is not drawn) are listed under 其他键 in the same colours, and the five most pressed keys close the card with their counts.
 *
 * <p>The colours are the calendar's: the hairline tone for a key never pressed, then the accent at 30, 50, 75 and 100 percent by the key's share of the busiest key, under the same 少…多 legend (see {@link HeatmapView}). Everything is drawn, so each key, 其他键 chip and top-five row is exposed to TalkBack as its own virtual node read as "A，123 次", and the view itself only carries a one-line summary.
 */
public final class KeyHeatmapView extends View {
    /** Accent opacity of each step above empty, as in {@link HeatmapView}. */
    private static final float[] LEVELS = {.3f, .5f, .75f, 1f};
    /** Width of each soft key in units of a letter key, row by row, matching {@link KeyPressIds#SOFT_ROWS}. */
    private static final float[][] SOFT_WEIGHTS = {
        {1, 1, 1, 1, 1, 1, 1, 1, 1, 1},
        {1, 1, 1, 1, 1, 1, 1, 1, 1},
        {1.5f, 1, 1, 1, 1, 1, 1, 1, 1.5f},
        {1, 1, 1, 4, 1.25f, 1.75f},
    };
    private static final int TOP_KEYS = 5;

    /** One drawn key or 其他键 chip, or one row of the top five: where it sits, what it shows, and what TalkBack reads for it. */
    private record Placed(String id, RectF bounds, String face, String spoken) {}

    private final Paint fill = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint face = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint label = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint rank = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Paint count = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final RectF box = new RectF();
    private final int accent;
    private final int empty;
    private final int ink;
    private final int onAccent;
    private Map<String, Long> counts = Map.of();
    private List<TypingStatisticsModel.Slice> top = List.of();
    private List<String> others = List.of();
    private boolean nineGrid;
    private long peak;
    /** The drawn keys and chips, then the top-five rows, in reading order; their index is their virtual view id. */
    private List<Placed> placed = List.of();
    private int placedKeys;
    private int placedWidth = -1;
    private final KeyNodes nodes;

    public KeyHeatmapView(Context context, AttributeSet attributes) {
        super(context, attributes);
        accent = ContextCompat.getColor(context, R.color.forest);
        empty = ContextCompat.getColor(context, R.color.hairline);
        ink = ContextCompat.getColor(context, R.color.ink);
        onAccent = ContextCompat.getColor(context, R.color.on_accent);
        face.setTextAlign(Paint.Align.CENTER);
        label.setColor(ContextCompat.getColor(context, R.color.text_secondary));
        label.setTextSize(dp(12f));
        rank.setColor(ink);
        rank.setTextSize(dp(15f));
        count.setColor(ContextCompat.getColor(context, R.color.text_secondary));
        count.setTextSize(dp(13f));
        nodes = new KeyNodes(this);
        ViewCompat.setAccessibilityDelegate(this, nodes);
    }

    /**
     * Show one scope's key counts.
     *
     * @param ranked every pressed key, most pressed first, as {@link TypingStatisticsModel#slices} gives them for {@link TypingStatisticsModel.Section#KEYS}
     */
    public void setKeys(List<TypingStatisticsModel.Slice> ranked) {
        List<TypingStatisticsModel.Slice> values = ranked == null ? List.of() : ranked;
        Map<String, Long> next = new LinkedHashMap<>();
        long highest = 0;
        for (TypingStatisticsModel.Slice slice : values) {
            if (slice.count() <= 0) continue;
            next.put(slice.id(), slice.count());
            highest = Math.max(highest, slice.count());
        }
        counts = java.util.Collections.unmodifiableMap(next);
        peak = highest;
        top = List.copyOf(values.subList(0, Math.min(TOP_KEYS, values.size())));
        nineGrid = KeyPressIds.hasNineKey(counts);
        others = KeyPressIds.others(counts, nineGrid);
        placedWidth = -1;
        setContentDescription(counts.isEmpty() ? "按键热力图，这一段时间还没有按键记录"
            : "按键热力图，" + counts.size() + " 个键有按键记录");
        requestLayout();
        invalidate();
        nodes.invalidateRoot();
    }

    /** The placed keys for the current width, laid out again only when the width or the counts change. */
    private List<Placed> placed() {
        int width = getWidth();
        if (width != placedWidth) {
            placed = layout(width);
            placedWidth = width;
        }
        return placed;
    }

    private List<Placed> layout(float width) {
        List<Placed> result = new ArrayList<>();
        float gap = gap();
        float unit = (width - gap * 9) / 10f;
        for (int row = 0; row < SOFT_WEIGHTS.length; row++) {
            List<String> ids = KeyPressIds.SOFT_ROWS.get(row);
            float[] weights = SOFT_WEIGHTS[row];
            float total = 0f;
            for (float weight : weights) total += weight;
            // The home row keeps the letter width and sits centred, offset half a key the way the keyboard staggers it; the other rows fill the width.
            float rowUnit = total < 10f ? unit : (width - gap * (weights.length - 1)) / total;
            float x = (width - (total * rowUnit + gap * (weights.length - 1))) / 2f;
            float top = row * (keyHeight() + gap);
            for (int index = 0; index < ids.size(); index++) {
                float keyWidth = weights[index] * rowUnit;
                result.add(key(ids.get(index), x, top, keyWidth, keyHeight(), face(ids.get(index))));
                x += keyWidth + gap;
            }
        }
        float y = softHeight();
        if (nineGrid) {
            float gridWidth = nineWidth(width);
            float cell = (gridWidth - gap * 2) / 3f;
            float left = (width - gridWidth) / 2f;
            float top = y + heading();
            for (int row = 0; row < KeyPressIds.NINE_ROWS.size(); row++) {
                List<String> ids = KeyPressIds.NINE_ROWS.get(row);
                for (int column = 0; column < ids.size(); column++) {
                    String id = ids.get(column);
                    result.add(key(id, left + column * (cell + gap), top + row * (keyHeight() + gap), cell,
                        keyHeight(), face(id)));
                }
            }
            y += nineHeight();
        }
        if (!others.isEmpty()) {
            float top = y + heading();
            List<float[]> chips = chips(width);
            for (int index = 0; index < others.size(); index++) {
                float[] chip = chips.get(index);
                String id = others.get(index);
                result.add(key(id, chip[0], top + chip[1], chip[2], chipHeight(), chipText(id)));
            }
            y += othersHeight(width);
        }
        placedKeys = result.size();
        float start = y + legendHeight() + heading();
        for (int index = 0; index < top.size(); index++) {
            TypingStatisticsModel.Slice slice = top.get(index);
            float rowTop = start + index * dp(28f);
            result.add(new Placed(slice.id(), new RectF(0f, rowTop, width, rowTop + dp(28f)),
                (index + 1) + "  " + slice.title(),
                "第 " + (index + 1) + " 名，" + slice.title() + "，" + slice.count() + " 次"));
        }
        return List.copyOf(result);
    }

    private Placed key(String id, float x, float y, float width, float height, String text) {
        return new Placed(id, new RectF(x, y, x + width, y + height), text,
            KeyPressIds.label(id) + "，" + counts.getOrDefault(id, 0L) + " 次");
    }

    private float dp(float value) { return value * getResources().getDisplayMetrics().density; }

    private float gap() { return dp(5f); }

    private float keyHeight() { return dp(40f); }

    private float softHeight() { return SOFT_WEIGHTS.length * (keyHeight() + gap()) - gap(); }

    private float heading() { return dp(30f); }

    private float nineWidth(float width) { return Math.min(width, dp(240f)); }

    private float nineHeight() {
        return nineGrid ? heading() + KeyPressIds.NINE_ROWS.size() * (keyHeight() + gap()) - gap() : 0f;
    }

    private float chipHeight() { return dp(30f); }

    /** Where each 其他键 chip starts, laid out left to right and wrapped at the view's width; the last entry is the height they take. */
    private List<float[]> chips(float width) {
        List<float[]> result = new ArrayList<>();
        float x = 0f;
        float y = 0f;
        face.setTextSize(dp(13f));
        for (String id : others) {
            float chip = chipWidth(id);
            if (x > 0f && x + chip > width) {
                x = 0f;
                y += chipHeight() + gap();
            }
            result.add(new float[] {x, y, chip});
            x += chip + gap();
        }
        return result;
    }

    private float chipWidth(String id) {
        return face.measureText(chipText(id)) + dp(20f);
    }

    private String chipText(String id) {
        return KeyPressIds.label(id) + "  " + counts.getOrDefault(id, 0L);
    }

    private float othersHeight(float width) {
        if (others.isEmpty()) return 0f;
        List<float[]> placed = chips(width);
        float[] last = placed.get(placed.size() - 1);
        return heading() + last[1] + chipHeight();
    }

    private float legendHeight() { return dp(28f); }

    private float topHeight() {
        return heading() + Math.max(1, top.size()) * dp(28f);
    }

    @Override protected void onMeasure(int widthSpec, int heightSpec) {
        int width = resolveSize((int) dp(300f), widthSpec);
        float height = softHeight() + nineHeight() + othersHeight(width) + legendHeight() + topHeight();
        setMeasuredDimension(width, resolveSize((int) Math.ceil(height), heightSpec));
    }

    @Override protected void onDraw(Canvas canvas) {
        float width = getWidth();
        List<Placed> keys = placed();
        for (int index = 0; index < placedKeys; index++) drawKey(canvas, keys.get(index));
        float y = softHeight();
        if (nineGrid) {
            canvas.drawText("九键", 0, y + heading() - dp(10f), label);
            y += nineHeight();
        }
        if (!others.isEmpty()) {
            canvas.drawText("其他键", 0, y + heading() - dp(10f), label);
            y += othersHeight(width);
        }
        y = drawLegend(canvas, width, y);
        drawTop(canvas, width, y);
    }

    @Override protected boolean dispatchHoverEvent(MotionEvent event) {
        return nodes.dispatchHoverEvent(event) || super.dispatchHoverEvent(event);
    }

    @Override public boolean dispatchKeyEvent(KeyEvent event) {
        return nodes.dispatchKeyEvent(event) || super.dispatchKeyEvent(event);
    }

    @Override protected void onFocusChanged(boolean gainFocus, int direction, @Nullable Rect previouslyFocusedRect) {
        super.onFocusChanged(gainFocus, direction, previouslyFocusedRect);
        nodes.onFocusChanged(gainFocus, direction, previouslyFocusedRect);
    }

    /** 少, the five steps, 多; right-aligned under the keys, as under the calendar. */
    private float drawLegend(Canvas canvas, float width, float y) {
        float swatch = dp(11f);
        float gap = dp(3f);
        float bottom = y + legendHeight() - dp(6f);
        float more = label.measureText("多");
        canvas.drawText("多", width - more, bottom - dp(1f), label);
        float x = width - more - dp(6f) - swatch;
        for (int level = LEVELS.length; level >= 0; level--) {
            fill.setColor(colour(level));
            box.set(x, bottom - swatch, x + swatch, bottom);
            canvas.drawRoundRect(box, dp(2.5f), dp(2.5f), fill);
            x -= swatch + gap;
        }
        float less = label.measureText("少");
        canvas.drawText("少", x + swatch + gap - dp(6f) - less, bottom - dp(1f), label);
        return y + legendHeight();
    }

    private void drawTop(Canvas canvas, float width, float y) {
        canvas.drawText("最常按的键", 0, y + heading() - dp(10f), label);
        float start = y + heading();
        if (top.isEmpty()) {
            canvas.drawText("这一段时间还没有按键记录", 0, start + dp(18f), count);
            return;
        }
        for (int index = 0; index < top.size(); index++) {
            TypingStatisticsModel.Slice slice = top.get(index);
            float baseline = start + index * dp(28f) + dp(18f);
            canvas.drawText((index + 1) + "  " + slice.title(), 0, baseline, rank);
            String times = slice.count() + " 次";
            canvas.drawText(times, width - count.measureText(times), baseline, count);
        }
    }

    private void drawKey(Canvas canvas, Placed key) {
        int level = level(counts.getOrDefault(key.id(), 0L));
        fill.setColor(colour(level));
        box.set(key.bounds());
        float width = box.width();
        String text = key.face();
        canvas.drawRoundRect(box, dp(7f), dp(7f), fill);
        // The two strongest steps are dark enough that ink on them stops reading; they take the accent's own foreground.
        face.setColor(level >= 3 ? onAccent : ink);
        face.setTextSize(dp(text.length() > 3 ? 11f : 14f));
        float textWidth = face.measureText(text);
        if (textWidth > width - dp(4f)) face.setTextSize(face.getTextSize() * (width - dp(4f)) / textWidth);
        canvas.drawText(text, box.centerX(), box.centerY() + face.getTextSize() * 0.36f, face);
    }

    /** What a drawn key shows: its label, with the nine-key cells reduced to the digit printed on them. */
    private static String face(String id) {
        if (id.startsWith("Nine")) return id.substring(4);
        return KeyPressIds.label(id);
    }

    /** 0 for a key never pressed, otherwise 1 to 4 by its share of the busiest key; a key pressed once still gets the first step. */
    private int level(long value) {
        if (peak <= 0 || value <= 0) return 0;
        return Math.max(1, Math.min(LEVELS.length, (int) Math.ceil(LEVELS.length * value / (double) peak)));
    }

    private int colour(int level) {
        if (level <= 0) return empty;
        return Color.argb(Math.round(255 * LEVELS[level - 1]), Color.red(accent), Color.green(accent),
            Color.blue(accent));
    }

    /** The virtual accessibility nodes: one per drawn key, 其他键 chip and top-five row, so TalkBack can step through them one at a time. */
    private static final class KeyNodes extends ExploreByTouchHelper {
        private final KeyHeatmapView view;
        private final Rect bounds = new Rect();

        KeyNodes(KeyHeatmapView view) {
            super(view);
            this.view = view;
        }

        @Override protected int getVirtualViewAt(float x, float y) {
            List<Placed> keys = view.placed();
            for (int index = 0; index < keys.size(); index++) {
                if (keys.get(index).bounds().contains(x, y)) return index;
            }
            return HOST_ID;
        }

        @Override protected void getVisibleVirtualViews(List<Integer> ids) {
            int size = view.placed().size();
            for (int index = 0; index < size; index++) ids.add(index);
        }

        @SuppressWarnings("deprecation")
        @Override protected void onPopulateNodeForVirtualView(int id, @NonNull AccessibilityNodeInfoCompat node) {
            List<Placed> keys = view.placed();
            // A stale id from before the counts changed still has to get bounds, or the helper throws; it reads as nothing until TalkBack refreshes.
            if (id < 0 || id >= keys.size()) {
                node.setContentDescription("");
                node.setBoundsInParent(new Rect());
                return;
            }
            Placed key = keys.get(id);
            node.setContentDescription(key.spoken());
            key.bounds().roundOut(bounds);
            node.setBoundsInParent(bounds);
        }

        @Override protected boolean onPerformActionForVirtualView(int id, int action, @Nullable Bundle arguments) {
            return false;
        }
    }
}
