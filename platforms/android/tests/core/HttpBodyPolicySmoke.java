import app.msime.android.HttpBodyPolicy;
import java.io.ByteArrayInputStream;
import java.nio.charset.StandardCharsets;

public final class HttpBodyPolicySmoke {
    static void check(boolean condition) { if (!condition) throw new AssertionError(); }

    public static void main(String[] args) throws Exception {
        check(new String(HttpBodyPolicy.readBounded(
            new ByteArrayInputStream("hello".getBytes(StandardCharsets.UTF_8)), 5),
            StandardCharsets.UTF_8).equals("hello"));
        check(HttpBodyPolicy.readBounded(
            new ByteArrayInputStream("hello".getBytes(StandardCharsets.UTF_8)), 4) == null);
        check(HttpBodyPolicy.readBounded(new ByteArrayInputStream(new byte[0]), 4).length == 0);
        System.out.println("Android bounded HTTP body policy passed");
    }
}
