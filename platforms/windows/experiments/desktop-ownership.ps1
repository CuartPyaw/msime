# 仅创建并销毁合成桌面与隐藏窗口；不切换输入桌面，不操作真实输入法。
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

public static class MsimeDesktop9439Probe {
    [DllImport("user32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern IntPtr CreateDesktop(string name, IntPtr device, IntPtr mode, uint flags, uint access, IntPtr security);
    [DllImport("user32.dll", SetLastError=true)] static extern bool CloseDesktop(IntPtr desktop);
    [DllImport("user32.dll", SetLastError=true)] static extern bool SetThreadDesktop(IntPtr desktop);
    [DllImport("user32.dll", SetLastError=true)] static extern IntPtr GetThreadDesktop(uint thread);
    [DllImport("kernel32.dll")] static extern uint GetCurrentThreadId();
    [DllImport("user32.dll", SetLastError=true)] static extern uint GetWindowThreadProcessId(IntPtr window, out uint pid);
    [DllImport("user32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern bool GetUserObjectInformation(IntPtr handle, int index, StringBuilder info, uint bytes, out uint needed);
    [DllImport("user32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern IntPtr CreateWindowEx(uint ex, string cls, string title, uint style, int x, int y, int w, int h,
        IntPtr parent, IntPtr menu, IntPtr instance, IntPtr parameter);
    [DllImport("user32.dll", SetLastError=true)] static extern bool DestroyWindow(IntPtr window);
    delegate bool EnumWindow(IntPtr window, IntPtr data);
    [DllImport("user32.dll", SetLastError=true)] static extern bool EnumDesktopWindows(IntPtr desktop, EnumWindow callback, IntPtr data);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern IntPtr CreateMutex(IntPtr security, bool owner, string name);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
    static void Check(bool condition, string message) {
        if (!condition) throw new InvalidOperationException(message + " (Win32 " + Marshal.GetLastWin32Error() + ")");
    }
    static string Name(IntPtr handle) {
        uint needed;
        StringBuilder text = new StringBuilder(1024);
        Check(GetUserObjectInformation(handle, 2, text, 2048, out needed), "object name query failed");
        return text.ToString();
    }
    static HashSet<IntPtr> Windows(IntPtr desktop) {
        HashSet<IntPtr> result = new HashSet<IntPtr>();
        EnumWindow callback = delegate(IntPtr window, IntPtr unused) { result.Add(window); return true; };
        Check(EnumDesktopWindows(desktop, callback, IntPtr.Zero), "desktop window enumeration failed");
        GC.KeepAlive(callback);
        return result;
    }
    public static string Run() {
        string token = "MSIME9439Probe" + Guid.NewGuid().ToString("N");
        string mutexName = "Local\\" + token;
        IntPtr[] desktops = new IntPtr[2];
        IntPtr[] windows = new IntPtr[2];
        Thread[] threads = new Thread[2];
        ManualResetEvent[] ready = {new ManualResetEvent(false),new ManualResetEvent(false)};
        ManualResetEvent release = new ManualResetEvent(false);
        Exception[] errors = new Exception[2];
        bool[] sameMutex = new bool[2];
        bool[] ownMutex = new bool[2];
        string[] names = new string[2];
        IntPtr shared = CreateMutex(IntPtr.Zero, false, mutexName);
        Check(shared != IntPtr.Zero, "probe mutex unavailable");
        try {
            for (int i=0; i<2; i++) {
                desktops[i] = CreateDesktop(token + i, IntPtr.Zero, IntPtr.Zero, 0, 0xC3, IntPtr.Zero);
                Check(desktops[i] != IntPtr.Zero, "private desktop creation failed");
                int index=i;
                threads[i] = new Thread(delegate() {
                    IntPtr original = GetThreadDesktop(GetCurrentThreadId());
                    IntPtr common = IntPtr.Zero;
                    IntPtr scoped = IntPtr.Zero;
                    try {
                        Check(SetThreadDesktop(desktops[index]), "desktop binding failed before window creation");
                        names[index] = Name(GetThreadDesktop(GetCurrentThreadId()));
                        windows[index] = CreateWindowEx(0,"STATIC","synthetic probe",0x80000000,0,0,120,60,
                            IntPtr.Zero,IntPtr.Zero,IntPtr.Zero,IntPtr.Zero);
                        Check(windows[index] != IntPtr.Zero, "private desktop window creation failed");
                        common=CreateMutex(IntPtr.Zero,false,mutexName);
                        int commonError=Marshal.GetLastWin32Error();
                        sameMutex[index]=common!=IntPtr.Zero && commonError==183;
                        scoped=CreateMutex(IntPtr.Zero,false,mutexName+"."+index);
                        int scopedError=Marshal.GetLastWin32Error();
                        ownMutex[index]=scoped!=IntPtr.Zero && scopedError!=183;
                        ready[index].Set();
                        release.WaitOne(15000);
                    } catch(Exception error) { errors[index]=error; ready[index].Set(); }
                    finally {
                        if(windows[index]!=IntPtr.Zero) DestroyWindow(windows[index]);
                        if(common!=IntPtr.Zero) CloseHandle(common);
                        if(scoped!=IntPtr.Zero) CloseHandle(scoped);
                        SetThreadDesktop(original);
                    }
                });
                threads[i].IsBackground=true;
                threads[i].Start();
            }
            for(int i=0;i<2;i++) {
                Check(ready[i].WaitOne(10000),"desktop worker timeout");
                if(errors[i]!=null) throw errors[i];
                Check(names[i]==token+i,"worker attached to wrong desktop");
            }
            for(int i=0;i<2;i++) {
                uint process;
                uint thread=GetWindowThreadProcessId(windows[i],out process);
                Check(thread!=0 && process==(uint)System.Diagnostics.Process.GetCurrentProcess().Id,"window owner query failed");
                Check(Name(GetThreadDesktop(thread))==names[i],"main thread cannot resolve another desktop worker");
            }
            HashSet<IntPtr> first=Windows(desktops[0]);
            HashSet<IntPtr> second=Windows(desktops[1]);
            Check(first.Contains(windows[0])&&!first.Contains(windows[1]),"first desktop contains wrong window");
            Check(second.Contains(windows[1])&&!second.Contains(windows[0]),"second desktop contains wrong window");
            Check(sameMutex[0]&&sameMutex[1],"unscoped instance mutex does not collide as expected");
            Check(ownMutex[0]&&ownMutex[1],"desktop-scoped mutexes collide");
            return "{\"ownerThreadDesktopResolvedFromMain\":true,\"separateDesktopWindows\":true,\"unscopedMutexCollidesAcrossDesktops\":true,\"scopedMutexesIndependent\":true,\"switchedInputDesktop\":false,\"productionInputMethodChanged\":false}";
        } finally {
            release.Set();
            for(int i=0;i<2;i++) if(threads[i]!=null) Check(threads[i].Join(5000),"desktop worker cleanup timeout");
            for(int i=0;i<2;i++) {
                if(desktops[i]!=IntPtr.Zero) Check(CloseDesktop(desktops[i]),"private desktop cleanup failed");
                ready[i].Dispose();
            }
            CloseHandle(shared);
            release.Dispose();
        }
    }
}
'@
[MsimeDesktop9439Probe]::Run()
