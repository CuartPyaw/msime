package app.msime.android.core;

public final class TelemetryHandlerSmoke {
    private static void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    public static void main(String[] args) {
        Thread.UncaughtExceptionHandler previous = Thread.getDefaultUncaughtExceptionHandler();
        try {
            check(Telemetry.installCrashHandler(null), "first installation must succeed");
            Thread.UncaughtExceptionHandler installed = Thread.getDefaultUncaughtExceptionHandler();
            check(installed != previous, "installation must replace the prior handler");
            check(!Telemetry.installCrashHandler(null), "reinstallation must be ignored");
            check(Thread.getDefaultUncaughtExceptionHandler() == installed,
                "reinstallation must not wrap the handler again");
            System.out.println("Android telemetry crash handler installation is idempotent");
        } finally {
            Thread.setDefaultUncaughtExceptionHandler(previous);
        }
    }
}
