package app.msime.android.test;

import android.accessibilityservice.AccessibilityService;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.graphics.Bitmap;
import android.os.SystemClock;
import android.system.Os;
import android.view.accessibility.AccessibilityNodeInfo;
import app.msime.ocr.fixture.SyntheticOcrFixture;
import java.io.File;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.RandomAccessFile;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.List;

/** 只操作专用模拟器中的合成模块、相机图和编辑器，不导出任意用户内容。 */
public final class OcrProductDeviceSmoke extends DeviceSmoke {
    private Class<?> runtime;
    private Method open;
    private Method install;
    private File source;

    @Override protected String successDescription() { return "pinned OCR import, readonly isolated loading, camera capture and editor insertion"; }

    @Override protected void runChecks() throws Exception {
        Context context = getTargetContext();
        long offlineDeadline = SystemClock.uptimeMillis() + 10000;
        android.net.ConnectivityManager connectivity = context.getSystemService(android.net.ConnectivityManager.class);
        while (connectivity.getActiveNetwork() != null && SystemClock.uptimeMillis() < offlineDeadline) SystemClock.sleep(100);
        check(connectivity.getActiveNetwork() == null, "product OCR test runs without an active network");
        ClassLoader loader = context.getClassLoader();
        runtime = loader.loadClass("app.msime.android.OcrRuntime");
        open = runtime.getMethod("open", Context.class);
        Class<?> progress = loader.loadClass("app.msime.android.NativeClient$ResourcePackProgress");
        install = loader.loadClass("app.msime.android.ResourcePacks").getMethod("install",
            Context.class, String.class, List.class, List.class, progress);
        source = new File(context.getCacheDir(), "synthetic-ocr-import.apk");
        try (InputStream input = getContext().getAssets().open("ocr-product-module.apk");
             FileOutputStream output = new FileOutputStream(source)) {
            byte[] buffer = new byte[8192];
            int count;
            while ((count = input.read(buffer)) != -1) output.write(buffer, 0, count);
        }
        rejectOpen("missing module");
        corrupt(source);
        rejectImport("corrupt module");
        copySource();
        File directory = importModule();
        File apk = new File(directory, "msime-ocr-v1.apk");
        check((Os.stat(apk.getAbsolutePath()).st_mode & 0222) == 0, "import seals executable readonly");
        recognizeFixture();
        recognizeFixture();
        // 同长度错误导入不能替换已经能用的模块。
        corrupt(source);
        rejectImport("bad replacement");
        recognizeFixture();
        check(apk.setWritable(true, true), "test opens synthetic module permissions");
        rejectOpen("writable executable");
        check(apk.setReadOnly(), "test reseals synthetic module");
        corrupt(apk);
        rejectOpen("tampered published bytes");
        copySource();
        importModule();
        recognizeFixture();
        source.delete();

        automation.grantRuntimePermission(context.getPackageName(), "android.permission.CAMERA");
        editor();
        camera();
        tap(node -> equalsText("拍照并识别", node.getText()) && node.isEnabled());
        awaitAny(node -> equalsText("上屏勾选行", node.getText()));
        tap(node -> equalsText("上屏勾选行", node.getText()));
        dropStaleNodes();
        awaitAny(field("msime-test-plain").and(node -> node.refresh() && node.getText() != null
            && node.getText().toString().replaceAll("\\s", "").contains("SYNTHETICOCR2468")));
        // 识别后离开输入框不能把待选结果带入下一个编辑器。
        camera();
        tap(node -> equalsText("拍照并识别", node.getText()) && node.isEnabled());
        automation.performGlobalAction(AccessibilityService.GLOBAL_ACTION_HOME);
        editor();
        SystemClock.sleep(2000);
        dropStaleNodes();
        AccessibilityNodeInfo target = awaitAny(field("msime-test-plain"));
        check(target.getText() == null || target.getText().length() == 0, "late recognition cannot insert in another editor");
    }

    private void copySource() throws Exception {
        try (InputStream input = getContext().getAssets().open("ocr-product-module.apk");
             FileOutputStream output = new FileOutputStream(source)) {
            byte[] buffer = new byte[8192];
            int count;
            while ((count = input.read(buffer)) != -1) output.write(buffer, 0, count);
        }
    }

    private File importModule() throws Exception {
        return (File) install.invoke(null, getTargetContext(), "ocr-module-android", List.of(), List.of(source.getAbsolutePath()), null);
    }

    private void rejectImport(String purpose) throws Exception {
        try { importModule(); }
        catch (InvocationTargetException rejected) { return; }
        throw new AssertionError("Import accepted " + purpose);
    }

    private void rejectOpen(String purpose) throws Exception {
        try {
            Object module = open.invoke(null, getTargetContext());
            runtime.getMethod("close").invoke(module);
        } catch (InvocationTargetException rejected) { return; }
        throw new AssertionError("Runtime accepted " + purpose);
    }

    private void recognizeFixture() throws Exception {
        Object module = open.invoke(null, getTargetContext());
        Bitmap image = SyntheticOcrFixture.render(1);
        try {
            String[] lines = (String[]) runtime.getMethod("recognize", Bitmap.class).invoke(module, image);
            check(SyntheticOcrFixture.matches(lines), "isolated product runtime recognizes complete Chinese and Latin fixture");
        } finally {
            runtime.getMethod("close").invoke(module);
            image.recycle();
        }
    }

    private static void corrupt(File file) throws Exception {
        boolean sealed = (Os.stat(file.getAbsolutePath()).st_mode & 0222) == 0;
        if (sealed) check(file.setWritable(true, true), "test unseals synthetic bytes");
        try (RandomAccessFile output = new RandomAccessFile(file, "rw")) {
            int value = output.read();
            output.seek(0);
            output.write(value ^ 0xff);
        }
        if (sealed) check(file.setReadOnly(), "test reseals corrupt bytes");
    }

    private void editor() throws Exception {
        getContext().startActivity(new Intent().setComponent(new ComponentName(
            "app.msime.android.test", "app.msime.android.test.EditorActivity"))
            .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TASK));
        dropStaleNodes();
        tap(field("msime-test-plain"));
        awaitAny(key("更多"));
    }

    private void camera() throws Exception {
        dropStaleNodes();
        tap(key("更多"));
        showTool("拍照识字");
        tap(tool("拍照识字"));
        awaitAny(node -> equalsText("拍照并识别", node.getText()) && node.isEnabled());
    }

    private static void check(boolean value, String purpose) { if (!value) throw new AssertionError(purpose); }
}
