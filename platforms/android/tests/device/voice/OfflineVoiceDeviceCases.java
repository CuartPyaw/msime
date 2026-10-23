package app.msime.android.test;

import android.app.Activity;
import android.app.Instrumentation;
import android.app.PendingIntent;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.media.AudioFormat;
import android.media.AudioRecord;
import android.media.MediaRecorder;
import android.os.Build;
import android.os.Bundle;
import android.os.SystemClock;
import android.speech.RecognizerIntent;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;

/** 实际产品进程中的启动取消边界与标准结果通道；合成结果不代表模型识别验收。 */
final class OfflineVoiceDeviceCases {
    private final Instrumentation instrumentation;
    private final String scenario;
    private volatile Activity activity;
    private volatile Throwable failure;

    OfflineVoiceDeviceCases(Instrumentation instrumentation, String scenario) {
        this.instrumentation = instrumentation;
        this.scenario = scenario;
    }

    void onCreated(Activity created) {
        if (!created.getClass().getName().equals("app.msime.android.OfflineVoiceRecognitionActivity")) return;
        activity = created;
        if (!scenario.equals("offline-result") && !scenario.equals("offline-cancel")) return;
        try {
            var deliver = created.getClass().getDeclaredMethod("returnResult", String.class);
            deliver.setAccessible(true);
            String text = scenario.equals("offline-result") ? "synthetic external transcript" : null;
            deliver.invoke(created, text);
            deliver.invoke(created, "synthetic late transcript");
        } catch (ReflectiveOperationException error) {
            failure = error;
        }
    }

    void run() {
        Bundle result = new Bundle();
        try {
            if (scenario.equals("offline-capture-cancel")) captureCancellation();
            else externalResult();
            result.putString("stream", "Offline voice boundary passed: " + scenario);
            instrumentation.finish(Activity.RESULT_OK, result);
        } catch (Exception | AssertionError error) {
            result.putString("stream", "Offline voice boundary failed: " + error.getClass().getSimpleName()
                + ": " + error.getMessage());
            instrumentation.finish(Activity.RESULT_CANCELED, result);
        }
    }

    private void externalResult() throws Exception {
        Context context = instrumentation.getTargetContext();
        if (scenario.equals("offline-permission-home") && context.checkSelfPermission(
                android.Manifest.permission.RECORD_AUDIO) == android.content.pm.PackageManager.PERMISSION_GRANTED) {
            throw new AssertionError("Revoke RECORD_AUDIO before testing permission-stage Home");
        }
        AtomicInteger deliveries = new AtomicInteger();
        AtomicReference<Intent> returned = new AtomicReference<>();
        AtomicInteger code = new AtomicInteger();
        String action = context.getPackageName() + ".OFFLINE_SYNTHETIC_RESULT";
        BroadcastReceiver receiver = new BroadcastReceiver() {
            @Override public void onReceive(Context ignored, Intent intent) {
                code.set(getResultCode());
                returned.set(intent);
                deliveries.incrementAndGet();
            }
        };
        if (Build.VERSION.SDK_INT >= 33) context.registerReceiver(receiver, new IntentFilter(action), Context.RECEIVER_NOT_EXPORTED);
        else context.registerReceiver(receiver, new IntentFilter(action));
        int flags = PendingIntent.FLAG_CANCEL_CURRENT;
        if (Build.VERSION.SDK_INT >= 31) flags |= PendingIntent.FLAG_MUTABLE;
        PendingIntent pending = PendingIntent.getBroadcast(context, 9159,
            new Intent(action).setPackage(context.getPackageName()), flags);
        try {
            Bundle extras = new Bundle();
            extras.putString("synthetic_marker", "preserved");
            extras.putStringArrayList(RecognizerIntent.EXTRA_RESULTS,
                new java.util.ArrayList<>(java.util.List.of("synthetic stale transcript")));
            Intent request = new Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH)
                .setPackage(context.getPackageName())
                .putExtra(RecognizerIntent.EXTRA_RESULTS_PENDINGINTENT, pending)
                .putExtra(RecognizerIntent.EXTRA_RESULTS_PENDINGINTENT_BUNDLE, extras)
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
            if (context.getPackageManager().resolveActivity(request, 0) == null) {
                throw new AssertionError("Standard request is not discoverable");
            }
            instrumentation.runOnMainSync(() -> context.startActivity(request));
            await(() -> activity != null, "External activity did not start");
            if (scenario.equals("offline-permission-home")) {
                await(this::permissionVisible, "Microphone permission dialog did not appear");
                instrumentation.waitForIdleSync();
                if (finished()) throw new AssertionError("Permission dialog cancelled a visible request");
                var started = activity.getClass().getDeclaredField("started");
                started.setAccessible(true);
                if (started.getBoolean(activity)) throw new AssertionError("Recording started before permission");
                try (var input = instrumentation.getUiAutomation().executeShellCommand("input keyevent 3")) {
                    new java.io.FileInputStream(input.getFileDescriptor()).readAllBytes();
                }
                await(this::finished, "Home did not cancel permission-stage request");
            } else if (!scenario.equals("offline-result") && !scenario.equals("offline-cancel")) {
                throw new AssertionError("Unknown offline scenario");
            }
            await(() -> deliveries.get() > 0, "No PendingIntent result");
            instrumentation.waitForIdleSync();
            if (failure != null) throw new AssertionError("Activity callback failed", failure);
            if (deliveries.get() != 1) throw new AssertionError("Result delivered more than once");
            Intent data = returned.get();
            boolean success = scenario.equals("offline-result");
            if (code.get() != (success ? Activity.RESULT_OK : Activity.RESULT_CANCELED)) {
                throw new AssertionError("Unexpected result code");
            }
            if (!"preserved".equals(data.getStringExtra("synthetic_marker"))) throw new AssertionError("Caller bundle lost");
            if (success) {
                if (!java.util.List.of("synthetic external transcript").equals(
                        data.getStringArrayListExtra(RecognizerIntent.EXTRA_RESULTS))) {
                    throw new AssertionError("Unexpected synthetic result");
                }
            } else if (data.hasExtra(RecognizerIntent.EXTRA_RESULTS)) {
                throw new AssertionError("Cancelled request retained caller-supplied results");
            }
        } finally {
            pending.cancel();
            context.unregisterReceiver(receiver);
            if (activity != null) instrumentation.runOnMainSync(activity::finish);
        }
    }

    private boolean finished() {
        return activity != null && (activity.isFinishing() || activity.isDestroyed());
    }

    @SuppressWarnings("deprecation") // API 28 的辅助功能节点仍需显式回收。
    private boolean permissionVisible() {
        var root = instrumentation.getUiAutomation().getRootInActiveWindow();
        if (root == null) return false;
        try {
            String owner = String.valueOf(root.getPackageName());
            return owner.contains("permissioncontroller") || owner.contains("packageinstaller");
        } finally {
            root.recycle();
        }
    }

    private void captureCancellation() throws Exception {
        ClassLoader loader = instrumentation.getTargetContext().getClassLoader();
        Class<?> type = loader.loadClass("app.msime.android.LocalAsrRecognizer");
        Class<?> listener = loader.loadClass("app.msime.android.LocalAsrRecognizer$Listener");
        var capture = type.getDeclaredMethod("capture", AudioRecord.class,
            java.util.concurrent.BlockingQueue.class, AtomicBoolean.class, listener);
        capture.setAccessible(true);
        Object recognizer = type.getConstructor().newInstance();
        CountDownLatch preparing = new CountDownLatch(1);
        CountDownLatch resume = new CountDownLatch(1);
        java.util.function.Consumer<Object> diagnostics = kind -> {
            if (!kind.toString().equals("VOICE_LOCAL_CAPTURE_START")) return;
            preparing.countDown();
            try {
                if (!resume.await(5, TimeUnit.SECONDS)) throw new AssertionError("Capture barrier timed out");
            } catch (InterruptedException error) {
                Thread.currentThread().interrupt();
                throw new AssertionError(error);
            }
        };
        type.getMethod("diagnostics", java.util.function.Consumer.class).invoke(recognizer, diagnostics);
        CountingRecorder recorder = new CountingRecorder();
        AtomicReference<Throwable> error = new AtomicReference<>();
        Thread worker = new Thread(() -> {
            try {
                capture.invoke(recognizer, recorder, new LinkedBlockingQueue<short[]>(), new AtomicBoolean(), null);
            } catch (ReflectiveOperationException exception) {
                error.set(exception);
            }
        });
        try {
            worker.start();
            if (!preparing.await(5, TimeUnit.SECONDS)) throw new AssertionError("Capture did not reach start boundary");
            type.getMethod("cancel").invoke(recognizer);
        } finally {
            resume.countDown();
            worker.join(5000);
        }
        if (worker.isAlive()) throw new AssertionError("Capture did not release after cancellation");
        if (error.get() != null) throw new AssertionError("Capture failed", error.get());
        if (recorder.starts.get() != 0) throw new AssertionError("Microphone started after cancellation");
    }

    /** 真实 AudioRecord 资源配合启动计数，断言取消后连一次启动调用也没有。 */
    private static final class CountingRecorder extends AudioRecord {
        final AtomicInteger starts = new AtomicInteger();

        CountingRecorder() {
            super(MediaRecorder.AudioSource.VOICE_RECOGNITION, 16000,
                AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT,
                Math.max(32000, AudioRecord.getMinBufferSize(16000,
                    AudioFormat.CHANNEL_IN_MONO, AudioFormat.ENCODING_PCM_16BIT)));
            if (getState() != STATE_INITIALIZED) throw new AssertionError("Recorder needs microphone permission");
        }

        @Override public void startRecording() {
            starts.incrementAndGet();
        }
    }

    private static void await(java.util.function.BooleanSupplier condition, String message) {
        long deadline = SystemClock.uptimeMillis() + 10000;
        while (!condition.getAsBoolean() && SystemClock.uptimeMillis() < deadline) SystemClock.sleep(25);
        if (!condition.getAsBoolean()) throw new AssertionError(message);
    }
}
