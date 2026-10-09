import app.msime.android.ThreadPolicy;
import java.util.concurrent.ThreadFactory;
import java.util.concurrent.atomic.AtomicBoolean;

public final class ThreadPolicySmoke {
    static void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    public static void main(String[] args) throws InterruptedException {
        AtomicBoolean ran = new AtomicBoolean(false);
        ThreadFactory factory = ThreadPolicy.namedDaemonFactory("msime-smoke-worker");
        Thread thread = factory.newThread(() -> ran.set(true));

        check("msime-smoke-worker".equals(thread.getName()), "thread name is preserved");
        check(thread.isDaemon(), "worker thread is a daemon");
        check(thread.getState() == Thread.State.NEW, "factory does not start the thread");

        thread.start();
        thread.join();
        check(ran.get(), "worker task runs");
        System.out.println("Android named daemon thread factory passed");
    }
}
