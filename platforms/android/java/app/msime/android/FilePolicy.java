package app.msime.android;

import java.io.File;

/** 共享文件操作策略。 */
public final class FilePolicy {
    private FilePolicy() {}

    /** 删除文件；删除失败时交给虚拟机退出钩子再次尝试。 */
    public static void deleteQuietly(File file) {
        if (file.exists() && !file.delete()) file.deleteOnExit();
    }
}
