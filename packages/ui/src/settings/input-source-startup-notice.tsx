import { useState } from "react";
import { actionRow, primary } from "../account/account-style";

export type InputSourceStartupStatus = {
  /** `login_required`: the input method is installed, but this login session's input source list only picks it up after the user logs in again. */
  action: "installed" | "updated" | "up_to_date" | "login_required" | "failed";
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
  /** Adds the installed input method to the input source list; absent when the host cannot, which leaves only the manual route. */
  onEnable?: () => Promise<void>;
  onDismiss: () => void;
  onError: (message: string) => void;
}

const settingsPath = "「系统设置 › 键盘 › 文字输入 › 输入法」";
const openSettingsError = `无法打开系统设置，请手动前往${settingsPath}。`;

function enableErrorMessage(error: unknown) {
  const code =
    typeof error === "object" && error !== null ? (error as { code?: unknown }).code : undefined;
  if (code === "not_installed") {
    return "没有找到已安装的水杉输入法，请在「快捷键」页的「输入法服务」中点「安装 / 更新」。";
  }
  return `系统没有接受自动添加，请在${settingsPath}中点「编辑…」手动添加水杉输入法。`;
}

/** Shows the macOS input-source installation result reported during startup, and what is left for the user to do. */
export function InputSourceStartupNotice({
  status,
  onOpenSettings,
  onEnable,
  onDismiss,
  onError,
}: InputSourceStartupNoticeProps) {
  const [enabling, setEnabling] = useState(false);
  const [enabled, setEnabled] = useState(false);
  const failed = status.action === "failed";
  const loginRequired = status.action === "login_required";
  const needsEnabling = status.enabled === false && !failed && !loginRequired && !enabled;
  const version = status.installed_version;

  const enable = () => {
    if (!onEnable || enabling) return;
    setEnabling(true);
    void onEnable()
      .then(() => setEnabled(true))
      .catch((error: unknown) => onError(enableErrorMessage(error)))
      .finally(() => setEnabling(false));
  };
  const openSettings = () => {
    void Promise.resolve(onOpenSettings()).catch(() => onError(openSettingsError));
  };

  let title: string;
  let detail: string | null = null;
  if (failed) {
    title = "水杉输入法没能自动安装或更新";
    detail = "请在「快捷键」页的「输入法服务」中点「安装 / 更新」重试。";
  } else if (loginRequired) {
    title = "重新登录后才能添加水杉输入法";
    detail = `水杉输入法已装到本机，但这次登录的输入法列表还看不到它。请注销并重新登录，然后在${settingsPath}中点「编辑…」添加水杉输入法。`;
  } else if (enabled) {
    title = "水杉输入法已启用";
    detail = "在菜单栏的输入法菜单里选择「水杉输入法」，就可以开始输入了。";
  } else if (needsEnabling) {
    title = "水杉输入法还没有加入输入法列表";
    detail = onEnable
      ? `加入后才能在菜单栏的输入法菜单里切换到它。点「启用水杉输入法」自动加入，也可以在${settingsPath}中点「编辑…」手动添加。`
      : `加入后才能在菜单栏的输入法菜单里切换到它。请在${settingsPath}中点「编辑…」添加水杉输入法。`;
  } else if (status.action === "installed") {
    title = version ? `水杉输入法 ${version} 已安装` : "水杉输入法已安装";
  } else if (status.action === "updated") {
    title = version ? `水杉输入法已更新到 ${version}` : "水杉输入法已更新";
  } else {
    title = "系统目录里多了一份水杉输入法";
  }
  // The install result still matters when the headline is about enabling: it says which version is now in place.
  const installResult =
    needsEnabling && (status.action === "installed" || status.action === "updated")
      ? status.action === "installed"
        ? `已安装${version ? ` ${version}` : ""}。`
        : `已更新${version ? `到 ${version}` : ""}。`
      : null;
  const systemBundles = status.system_bundles?.length
    ? `「/Library/Input Methods」里还有一份水杉输入法（${status.system_bundles.join("、")}），它会让输入法列表出现重复项，或用上较旧的版本。请在「访达」中按 Shift+Command+G 前往该文件夹，把它移到废纸篓（需要管理员密码），然后注销并重新登录。`
    : null;

  return (
    <div
      role={failed ? "alert" : "status"}
      className="section flex flex-col gap-2"
      aria-label="水杉输入法安装状态"
    >
      <p className={`section-title m-0${failed ? " text-[var(--danger-text)]" : ""}`}>{title}</p>
      {detail && (
        <p className="notice m-0">
          {installResult}
          {detail}
        </p>
      )}
      {systemBundles && <p className="notice m-0">{systemBundles}</p>}
      <div className={`${actionRow} mt-1`}>
        {needsEnabling && onEnable && (
          <button type="button" className={primary} disabled={enabling} onClick={enable}>
            {enabling ? "正在启用…" : "启用水杉输入法"}
          </button>
        )}
        {needsEnabling && (
          <button type="button" className="secondary" onClick={openSettings}>
            打开键盘设置
          </button>
        )}
        <button type="button" className="secondary" onClick={onDismiss}>
          知道了
        </button>
      </div>
    </div>
  );
}
