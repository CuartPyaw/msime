package app.msime.android;

import java.util.Map;
import java.util.TreeMap;

/**
 * Key presses held in memory until they are worth one write to the shared statistics store.
 *
 * <p>The store takes its file lock and rewrites the whole document on every call, which is fine once a commit and far too much once a key, so the input service counts here and hands a whole batch to its worker. The batch belongs to one local day: the counts are filed under the day the keys were pressed, never the day the write happens to run, so a batch that is still open at midnight is handed back for flushing under its own day before the first press of the next one is counted.
 *
 * <p>Holds only per-key counts and the day they belong to; no order, no timing, no text. Not thread-safe: the input service touches it on the main thread only and flushes immutable copies.
 */
public final class KeyPressBatch {
    /** Presses after which a batch is flushed without waiting for the timer. */
    public static final int FLUSH_PRESSES = 256;

    /** One batch ready for the store: the local day and each key's press count. */
    public record Flush(String day, Map<String, Long> keys) {
        public Flush {
            keys = Map.copyOf(keys);
        }

        /** The number of presses in this batch. */
        public long presses() {
            long total = 0;
            for (long count : keys.values()) total += count;
            return total;
        }
    }

    private final TreeMap<String, Long> keys = new TreeMap<>();
    private String day = "";
    private int presses;

    /**
     * Counts one press of {@code id} on the local day {@code today}.
     *
     * @return the batch of an earlier day that must be flushed before this press's day, or {@code null} when the open batch is still today's. An id the store does not know is ignored, since one would reject the whole batch.
     */
    public Flush add(String id, String today) {
        if (!KeyPressIds.isKnown(id) || today == null || today.isEmpty()) return null;
        Flush previous = day.equals(today) ? null : drain();
        day = today;
        keys.merge(id, 1L, Long::sum);
        presses++;
        return previous;
    }

    /** Whether the batch has reached {@link #FLUSH_PRESSES} and should be flushed now. */
    public boolean full() { return presses >= FLUSH_PRESSES; }

    public boolean isEmpty() { return presses == 0; }

    /** The open batch, which is then emptied; {@code null} when nothing was counted. */
    public Flush drain() {
        if (presses == 0) return null;
        Flush flush = new Flush(day, keys);
        keys.clear();
        presses = 0;
        day = "";
        return flush;
    }

    /** Drops the open batch unflushed, for when recording has been turned off. */
    public void clear() {
        keys.clear();
        presses = 0;
        day = "";
    }
}
