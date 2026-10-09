package app.msime.android;

import java.util.concurrent.ThreadFactory;

/** Android 宿主共用的线程创建策略。 */
public final class ThreadPolicy {
    private ThreadPolicy() {}

    /** 创建使用固定名称且不会阻止进程退出的未启动线程。 */
    public static Thread namedDaemonThread(String name, Runnable runnable) {
        Thread thread = new Thread(runnable, name);
        thread.setDaemon(true);
        return thread;
    }

    /** 创建使用固定名称且不会阻止进程退出的线程工厂。 */
    public static ThreadFactory namedDaemonFactory(String name) {
        return runnable -> namedDaemonThread(name, runnable);
    }
}
