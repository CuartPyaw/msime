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
    private final java.util.List<String> stages = new java.util.concurrent.CopyOnWriteArrayList<>();

    OfflineVoiceDeviceCases(Instrumentation instrumentation, String scenario) {
        this.instrumentation = instrumentation;
        this.scenario = scenario;
    }

    void onCreated(Activity created) {
        if (!created.getClass().getName().equals("app.msime.android.OfflineVoiceRecognitionActivity")) return;
        activity = created;
        if (scenario.equals("offline-asr")) {
            try {
                var field = created.getClass().getDeclaredField("recognizer");
                field.setAccessible(true);
                Object recognizer = field.get(created);
                java.util.function.Consumer<Object> diagnostics = kind -> stages.add(kind.toString());
                recognizer.getClass().getMethod("diagnostics", java.util.function.Consumer.class)
                    .invoke(recognizer, diagnostics);
            } catch (ReflectiveOperationException error) {
                failure = error;
            }
        }
        if (!java.util.Set.of("offline-result", "offline-cancel", "offline-activity-result",
                "offline-activity-cancel").contains(scenario)) return;
        try {
            if (scenario.startsWith("offline-activity-")) {
                if (created.getCallingActivity() == null) throw new AssertionError("Caller identity missing");
                if (created.isFinishing()) throw new AssertionError("Standard Activity request rejected");
            }
            var deliver = created.getClass().getDeclaredMethod("returnResult", String.class);
            deliver.setAccessible(true);
            String text = scenario.equals("offline-result") || scenario.equals("offline-activity-result")
                ? "synthetic external transcript" : null;
            deliver.invoke(created, text);
            deliver.invoke(created, "synthetic late transcript");
        } catch (ReflectiveOperationException error) {
            failure = error;
        }
    }

    void run() {
        Bundle result = new Bundle();
        try {
            if (scenario.equals("offline-prepare")) prepareLocalModel();
            else if (scenario.equals("offline-capture-cancel")) captureCancellation();
            else if (scenario.startsWith("offline-activity-")) activityResult();
            else externalResult();
            result.putString("stream", "Offline voice boundary passed: " + scenario);
            instrumentation.finish(Activity.RESULT_OK, result);
        } catch (Exception | AssertionError error) {
            result.putString("stream", "Offline voice boundary failed: " + error.getClass().getSimpleName()
                + ": " + error.getMessage());
            instrumentation.finish(Activity.RESULT_CANCELED, result);
        }
    }

    /** 不同 UID 的测试应用必须通过 onActivityResult 接到单次成功或不带文字的取消。 */
    private void activityResult() throws Exception {
        Context context = instrumentation.getTargetContext();
        var caller = context.getPackageManager().getApplicationInfo("app.msime.android.test", 0);
        if (caller.uid == context.getApplicationInfo().uid) throw new AssertionError("Caller is not isolated");
        AtomicInteger reports = new AtomicInteger();
        AtomicInteger code = new AtomicInteger();
        AtomicReference<Intent> returned = new AtomicReference<>();
        String action = context.getPackageName() + ".OFFLINE_CALLER." + java.util.UUID.randomUUID();
        BroadcastReceiver receiver = new BroadcastReceiver() {
            @Override public void onReceive(Context ignored, Intent intent) {
                code.set(getResultCode());
                returned.set(intent);
                reports.incrementAndGet();
            }
        };
        if (Build.VERSION.SDK_INT >= 33) context.registerReceiver(receiver, new IntentFilter(action), Context.RECEIVER_NOT_EXPORTED);
        else context.registerReceiver(receiver, new IntentFilter(action));
        int flags = PendingIntent.FLAG_CANCEL_CURRENT;
        if (Build.VERSION.SDK_INT >= 31) flags |= PendingIntent.FLAG_MUTABLE;
        PendingIntent report = PendingIntent.getBroadcast(context, 9159,
            new Intent(action).setPackage(context.getPackageName()), flags);
        Intent launch = new Intent().setClassName("app.msime.android.test",
            "app.msime.android.test.OfflineVoiceCallerActivity")
            .putExtra(OfflineVoiceCallerActivity.REPORT, report)
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK);
        try {
            instrumentation.runOnMainSync(() -> context.startActivity(launch));
            await(() -> activity != null, "External activity was not called by the fixture");
            await(() -> reports.get() > 0, "Standard Activity result did not reach the separate caller");
            if (failure != null) throw new AssertionError("Activity callback failed", failure);
            boolean success = scenario.equals("offline-activity-result");
            Intent observed = returned.get();
            if (reports.get() != 1 || observed.getIntExtra("caller_deliveries", 0) != 1)
                throw new AssertionError("Caller observed repeated Activity results");
            if (code.get() != (success ? Activity.RESULT_OK : Activity.RESULT_CANCELED))
                throw new AssertionError("Caller observed an incorrect Activity result code");
            if (success) {
                if (!java.util.List.of("synthetic external transcript").equals(
                        observed.getStringArrayListExtra(RecognizerIntent.EXTRA_RESULTS)))
                    throw new AssertionError("Caller observed an incorrect transcript");
            } else if (observed.hasExtra(RecognizerIntent.EXTRA_RESULTS)) {
                throw new AssertionError("Cancelled Activity result contains text");
            }
            if (!finished()) throw new AssertionError("Completed request is still visible");
        } finally {
            context.unregisterReceiver(receiver);
            report.cancel();
            if (activity != null) instrumentation.runOnMainSync(activity::finish);
            try (var input = instrumentation.getUiAutomation().executeShellCommand(
                    "am force-stop app.msime.android.test")) {
                try (var stream = new java.io.FileInputStream(input.getFileDescriptor())) {
                    byte[] discard = new byte[4096];
                    while (stream.read(discard) != -1) { }
                }
            }
        }
    }

    /** 专用模拟器只导入预先校验的合成验收资源，安装与配置保存仍走产品共享接口。 */
    private void prepareLocalModel() throws Exception {
        Context context = instrumentation.getTargetContext();
        Activity home = instrumentation.startActivitySync(new Intent().setClassName(context,
            "app.msime.android.home.HomeActivity").addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
        java.io.File options = new java.io.File(context.getFilesDir(), "runtime-options.json");
        try {
            await(options::isFile, "Native host did not prepare runtime options");
            Class<?> client = context.getClassLoader().loadClass("app.msime.android.NativeClient");
            Class<?> progress = context.getClassLoader().loadClass("app.msime.android.NativeClient$LocalModelProgress");
            java.io.File archive = new java.io.File(context.getCacheDir(), "synthetic-offline-model.tar.bz2");
            try {
                try (var descriptor = instrumentation.getUiAutomation().executeShellCommand(
                        "cat /data/local/tmp/msime-synthetic-offline-model.tar.bz2");
                        var input = new java.io.FileInputStream(descriptor.getFileDescriptor());
                        var output = new java.io.FileOutputStream(archive)) {
                    byte[] buffer = new byte[64 * 1024];
                    int count;
                    while ((count = input.read(buffer)) != -1) output.write(buffer, 0, count);
                }
                String request = new org.json.JSONObject()
                    .put("root", new java.io.File(context.getFilesDir(), "voice-models").getAbsolutePath())
                    .put("id", "x-asr-zh-en-streaming")
                    .put("files", new org.json.JSONArray().put(archive.getAbsolutePath())).toString();
                Object installed = client.getMethod("voiceLocalModelInstall", String.class, progress)
                    .invoke(null, request, null);
                org.json.JSONObject model = new org.json.JSONObject((String) installed);
                if (!model.optBoolean("ok")) throw new AssertionError("Pinned model import failed: " + model.optString("error"));
                String directory = new org.json.JSONObject(new String(java.nio.file.Files.readAllBytes(options.toPath()),
                    java.nio.charset.StandardCharsets.UTF_8))
                    .getString("preferences_directory");
                org.json.JSONObject loaded = new org.json.JSONObject((String) client
                    .getMethod("loadPreferences", String.class).invoke(null, directory));
                if (!loaded.optBoolean("ok")) throw new AssertionError("Shared preferences unavailable");
                org.json.JSONObject snapshot = loaded.getJSONObject("value");
                org.json.JSONObject preferences = snapshot.getJSONObject("preferences");
                org.json.JSONObject voice = preferences.optJSONObject("voice_input");
                if (voice == null) preferences.put("voice_input", voice = new org.json.JSONObject());
                voice.put("asr_provider", "local").put("asr_model_path", model.getJSONObject("value").getString("path"));
                org.json.JSONObject saved = new org.json.JSONObject((String) client
                    .getMethod("savePreferences", String.class, long.class, String.class)
                    .invoke(null, directory, snapshot.getLong("revision"), snapshot.toString()));
                if (!saved.optBoolean("ok")) throw new AssertionError("Shared local model selection failed");
            } finally {
                java.nio.file.Files.deleteIfExists(archive.toPath());
            }
        } finally {
            instrumentation.runOnMainSync(home::finish);
        }
    }

    private void externalResult() throws Exception {
        Context context = instrumentation.getTargetContext();
        if (scenario.equals("offline-permission-home") && context.checkSelfPermission(
                android.Manifest.permission.RECORD_AUDIO) == android.content.pm.PackageManager.PERMISSION_GRANTED) {
            throw new AssertionError("Revoke RECORD_AUDIO before testing permission-stage Home");
        }
        if (scenario.equals("offline-permission-home")) {
            var automation = instrumentation.getUiAutomation();
            var info = automation.getServiceInfo();
            info.flags |= android.accessibilityservice.AccessibilityServiceInfo.FLAG_RETRIEVE_INTERACTIVE_WINDOWS;
            automation.setServiceInfo(info);
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
                .putExtra(RecognizerIntent.EXTRA_LANGUAGE, "en-US")
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
                    try (var stream = new java.io.FileInputStream(input.getFileDescriptor())) {
                        byte[] discard = new byte[4096];
                        while (stream.read(discard) != -1) { }
                    }
                }
                await(this::finished, "Home did not cancel permission-stage request");
            } else if (scenario.equals("offline-asr")) {
                finishSyntheticSpeech();
            } else if (!scenario.equals("offline-result") && !scenario.equals("offline-cancel")) {
                throw new AssertionError("Unknown offline scenario");
            }
            try {
                await(() -> deliveries.get() > 0, "No PendingIntent result");
            } catch (AssertionError timeout) {
                if (!scenario.equals("offline-asr")) throw timeout;
                throw new AssertionError(timeout.getMessage() + "; active methods=" + asrWorkerMethods());
            }
            instrumentation.waitForIdleSync();
            if (failure != null) throw new AssertionError("Activity callback failed", failure);
            if (deliveries.get() != 1) throw new AssertionError("Result delivered more than once");
            Intent data = returned.get();
            boolean success = scenario.equals("offline-result") || scenario.equals("offline-asr");
            if (code.get() != (success ? Activity.RESULT_OK : Activity.RESULT_CANCELED)) {
                throw new AssertionError("Unexpected result code");
            }
            if (!"preserved".equals(data.getStringExtra("synthetic_marker"))) throw new AssertionError("Caller bundle lost");
            if (scenario.equals("offline-asr")) {
                var results = data.getStringArrayListExtra(RecognizerIntent.EXTRA_RESULTS);
                if (results == null || results.size() != 1 || !results.get(0).toLowerCase(java.util.Locale.ROOT).contains("hello")) {
                    throw new AssertionError("Real local model did not return the synthetic greeting");
                }
            } else if (success) {
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

    /** 等待外部注入的合成音频经过实际麦克风、模型和界面，再点击产品完成按钮。 */
    private void finishSyntheticSpeech() throws Exception {
        var listening = activity.getClass().getDeclaredField("listening");
        listening.setAccessible(true);
        Object view = listening.get(activity);
        var level = view.getClass().getDeclaredField("level");
        level.setAccessible(true);
        var status = activity.getClass().getDeclaredField("status");
        status.setAccessible(true);
        var failed = activity.getClass().getDeclaredField("failed");
        failed.setAccessible(true);
        AtomicBoolean audible = new AtomicBoolean();
        AtomicBoolean greeting = new AtomicBoolean();
        AtomicBoolean refused = new AtomicBoolean();
        long deadline = SystemClock.uptimeMillis() + 50000;
        while (!greeting.get() && !refused.get() && SystemClock.uptimeMillis() < deadline && !finished()) {
            instrumentation.runOnMainSync(() -> {
                try {
                    refused.set(failed.getBoolean(activity));
                    if (level.getFloat(view) > .02f) audible.set(true);
                    String text = ((android.widget.TextView) status.get(activity)).getText().toString();
                    if (text.toLowerCase(java.util.Locale.ROOT).contains("hello")) greeting.set(true);
                } catch (ReflectiveOperationException error) {
                    throw new AssertionError(error);
                }
            });
            SystemClock.sleep(25);
        }
        if (failure != null) throw new AssertionError("ASR diagnostics unavailable", failure);
        if (refused.get()) throw new AssertionError("Product rejected local ASR at stages " + stages);
        if (!audible.get()) throw new AssertionError("Synthetic microphone audio did not drive the listening signal; stages " + stages);
        if (!greeting.get()) throw new AssertionError("Real model did not produce the synthetic partial greeting");
        var done = activity.getClass().getDeclaredField("done");
        done.setAccessible(true);
        instrumentation.runOnMainSync(() -> {
            try {
                android.widget.Button button = (android.widget.Button) done.get(activity);
                if (!button.isEnabled()) throw new AssertionError("Listening request cannot finish");
                button.performClick();
            } catch (ReflectiveOperationException error) {
                throw new AssertionError(error);
            }
        });
    }

    private boolean finished() {
        return activity != null && (activity.isFinishing() || activity.isDestroyed());
    }

    /** 仅记录固定类的方法名，区分录音阻塞与原生解码，不记录线程名或运行时参数。 */
    private static java.util.Set<String> asrWorkerMethods() {
        var methods = new java.util.TreeSet<String>();
        for (StackTraceElement[] trace : Thread.getAllStackTraces().values()) {
            for (StackTraceElement frame : trace) {
                if (java.util.Set.of("app.msime.android.LocalAsrRecognizer", "app.msime.android.NativeClient",
                        "android.media.AudioRecord").contains(frame.getClassName())) {
                    methods.add(frame.getClassName() + "." + frame.getMethodName());
                }
            }
        }
        return methods;
    }

    @SuppressWarnings("deprecation") // API 28 的辅助功能节点仍需显式回收。
    private boolean permissionVisible() {
        var automation = instrumentation.getUiAutomation();
        boolean visible = false;
        // 权限弹窗不一定成为辅助功能的活动窗口，按实际可交互窗口检查系统包。
        for (var window : automation.getWindows()) {
            try {
                visible |= permissionRoot(window.getRoot());
            } finally {
                window.recycle();
            }
        }
        return visible || permissionRoot(automation.getRootInActiveWindow());
    }

    @SuppressWarnings("deprecation") // API 28 的辅助功能节点仍需显式回收。
    private static boolean permissionRoot(android.view.accessibility.AccessibilityNodeInfo root) {
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
