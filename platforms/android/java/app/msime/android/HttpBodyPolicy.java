package app.msime.android;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.util.function.BooleanSupplier;

/** Shared bounded reader for HTTP response bodies. */
public final class HttpBodyPolicy {
    private HttpBodyPolicy() {}

    /** Reads at most {@code limit} bytes, returning {@code null} when the body is larger. */
    public static byte[] readBounded(InputStream input, int limit) throws IOException {
        return readBounded(input, limit, () -> false);
    }

    /** Reads a bounded body while allowing a caller to stop between input chunks. */
    public static byte[] readBounded(InputStream input, int limit,
            BooleanSupplier cancelled) throws IOException {
        if (input == null || limit < 0) return null;
        if (cancelled == null || cancelled.getAsBoolean()) return null;
        ByteArrayOutputStream output = new ByteArrayOutputStream(Math.min(limit, 8192));
        byte[] buffer = new byte[8192];
        int count;
        while ((count = input.read(buffer)) != -1) {
            if (cancelled.getAsBoolean()) return null;
            if (output.size() + count > limit) return null;
            output.write(buffer, 0, count);
        }
        return output.toByteArray();
    }
}
