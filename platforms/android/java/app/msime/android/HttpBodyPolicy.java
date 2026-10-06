package app.msime.android;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;

/** Shared bounded reader for HTTP response bodies. */
public final class HttpBodyPolicy {
    private HttpBodyPolicy() {}

    /** Reads at most {@code limit} bytes, returning {@code null} when the body is larger. */
    public static byte[] readBounded(InputStream input, int limit) throws IOException {
        if (input == null || limit < 0) return null;
        ByteArrayOutputStream output = new ByteArrayOutputStream(Math.min(limit, 8192));
        byte[] buffer = new byte[8192];
        int count;
        while ((count = input.read(buffer)) != -1) {
            if (output.size() + count > limit) return null;
            output.write(buffer, 0, count);
        }
        return output.toByteArray();
    }
}
