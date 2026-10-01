export type InputSourceStartupStatus = {
  /** `login_required`: the input method is installed, but this login session's input source list only picks it up after the user logs in again. `not_installed`: a first install, left for the user to start from the install window. */
  action: "installed" | "updated" | "up_to_date" | "not_installed" | "login_required" | "failed";
  /** Whether the input source is in the System Settings list; `null` when that list could not be read. */
  enabled: boolean | null;
  bundled_version: string | null;
  installed_version: string | null;
  /** Copies of the input method in `/Library/Input Methods`, which compete with the user's copy and need an administrator to remove; read afresh on every request. */
  system_bundles?: string[];
};

export interface InputSourceStartupNoticeProps {
  status: InputSourceStartupStatus;
  onOpenSettings: () => void | Promise<void>;
  onDismiss: () => void;
  onError: (message: string) => void;
}

/** Shows the macOS input-source installation result reported during startup. */
export function InputSourceStartupNotice({
  status,
  onOpenSettings,
  onDismiss,
  onError,
}: InputSourceStartupNoticeProps) {
  return (
    <div
      role={status.action === "failed" ? "alert" : "status"}
      className={status.action === "failed" ? "error" : "notice"}
      aria-label="水杉输入法安装状态"
    >
      {status.action === "installed" && (
        <p>水杉输入法已安装{status.installed_version ? `：${status.installed_version}` : ""}。</p>
      )}
      {status.action === "updated" && (
        <p>水杉输入法已更新{status.installed_version ? `到 ${status.installed_version}` : ""}。</p>
      )}
      {status.action === "login_required" && (
        <p>
          水杉输入法已安装到本机，但本次登录的输入法列表还看不到它。请注销并重新登录，然后在
          系统设置 &gt; 键盘 &gt; 文字输入 &gt; 输入法 中点「编辑…」添加水杉输入法。
        </p>
      )}
      {status.action === "not_installed" && (
        <p>水杉输入法还没有安装到本机。请在「快捷键」页的「输入法服务」中点「安装 / 更新」。</p>
      )}
      {status.action === "failed" && (
        <p>
          水杉输入法未能自动安装或更新。请在「快捷键」页的「输入法服务」中点「安装 / 更新」重试。
        </p>
      )}
      {status.enabled === false &&
        status.action !== "login_required" &&
        status.action !== "not_installed" && (
          <p>
            请在 系统设置 &gt; 键盘 &gt; 文字输入 &gt; 输入法 中点「编辑…」添加水杉输入法。
            <button
              type="button"
              className="secondary"
              onClick={() => {
                void Promise.resolve(onOpenSettings()).catch(() =>
                  onError("无法打开系统设置，请手动前往 系统设置 > 键盘 > 文字输入 > 输入法。"),
                );
              }}
            >
              打开键盘设置
            </button>
          </p>
        )}
      {status.system_bundles?.length ? (
        <p>
          「/Library/Input
          Methods」里还有一份水杉输入法，会让输入法列表出现重复项，或用上较旧的版本。请在「访达」中按
          Shift+Command+G 前往该文件夹，把 {status.system_bundles.join("、")}{" "}
          移到废纸篓（需要管理员密码），然后注销并重新登录。
        </p>
      ) : null}
      <button type="button" className="secondary" onClick={onDismiss}>
        知道了
      </button>
    </div>
  );
}
