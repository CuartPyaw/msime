import { useEffect, useState } from "react";
import { logo, privacyUrl } from "../settings/app-resources";
import type { InputSourceStartupStatus } from "../settings/input-source-startup-notice";
import { SetupStep } from "./onboarding-page";
import * as onboarding from "./onboarding-style";

export interface MacosInputSourceGuideClient {
  /** The start-time install result, with whether the source is in the System Settings list read afresh on every call. */
  status(): Promise<InputSourceStartupStatus | null>;
  /** Opens System Settings > 键盘. */
  openSettings(): Promise<void>;
  /** Opens the privacy policy in the browser; without it the system-notice explanation carries no link. */
  openExternalUrl?: (url: string) => Promise<void>;
}

/** How often the guide asks again while it waits for the user to add the source; focus changes ask immediately. */
export const INPUT_SOURCE_RECHECK_MS = 3000;
const COMPLETED_KEY = "msime.macos.input-source-guide.completed";

/** Whether the start-time result leaves the user without a usable input method: a first install waiting for the next login, a source missing from the System Settings list, a copy in `/Library/Input Methods` competing with the user's, or a failed install with no working copy behind it. An unreadable list alone does not count: the guide would then have nothing to wait for. */
export function macosInputSourceGuideNeeded(status: InputSourceStartupStatus | null): boolean {
  if (!status) return false;
  if (status.action === "login_required" || status.enabled === false) return true;
  if (status.system_bundles?.length) return true;
  return status.action === "failed" && status.enabled !== true;
}

// Storage is the webview's, which a locked-down profile can refuse; the guide then only loses its "added before" wording.
function readCompleted(): boolean {
  try {
    return window.localStorage.getItem(COMPLETED_KEY) === "1";
  } catch {
    return false;
  }
}

function rememberCompleted() {
  try {
    window.localStorage.setItem(COMPLETED_KEY, "1");
  } catch {
    /* See readCompleted. */
  }
}

type Stage = "failed" | "system" | "login" | "add" | "ready";

function stageOf(status: InputSourceStartupStatus): Stage {
  if (status.action === "failed" && status.enabled !== true) return "failed";
  // Before the login step: removing the system-wide copy also needs a new login, so one covers both.
  if (status.system_bundles?.length) return "system";
  if (status.action === "login_required") return "login";
  return status.enabled === true ? "ready" : "add";
}

const titles: Record<Stage, string> = {
  failed: "水杉输入法未能安装",
  system: "移除多余的水杉输入法副本",
  login: "注销并重新登录",
  add: "把水杉输入法添加到系统",
  ready: "已添加水杉输入法",
};

/** The macOS first-run page for the one step the settings app cannot do itself: adding the input method in System Settings. It follows the start-time install result and moves on by itself once the source shows up in the list. */
export function MacosInputSourceGuide({
  status: initial,
  client,
  onComplete,
}: {
  status: InputSourceStartupStatus;
  client: MacosInputSourceGuideClient;
  onComplete: () => void;
}) {
  const [status, setStatus] = useState(initial);
  const [error, setError] = useState("");
  // Read once: a source that was added before and is missing now was removed, which the lead says rather than repeating first-run instructions.
  const [returning] = useState(readCompleted);
  const stage = stageOf(status);

  // Only the add and system stages can change while the page is open: a login-bound install needs a new session, which restarts this app, and a failed one is retried from the settings page.
  useEffect(() => {
    if (stage !== "add" && stage !== "system") return;
    let active = true;
    const recheck = () =>
      void client
        .status()
        .then((next) => {
          if (active && next) setStatus(next);
        })
        .catch(() => {});
    const timer = window.setInterval(recheck, INPUT_SOURCE_RECHECK_MS);
    window.addEventListener("focus", recheck);
    return () => {
      active = false;
      window.clearInterval(timer);
      window.removeEventListener("focus", recheck);
    };
  }, [client, stage]);

  useEffect(() => {
    if (stage === "ready") rememberCompleted();
  }, [stage]);

  const openSettings = () => {
    setError("");
    void client
      .openSettings()
      .catch(() => setError("无法打开系统设置，请手动打开「系统设置」→「键盘」。"));
  };

  const addSteps = (
    <div className={onboarding.setupCard}>
      <SetupStep number={1} title="打开键盘设置">
        打开「系统设置」→「键盘」，在「文字输入」下的「输入法」一行点「编辑…」。
      </SetupStep>
      <SetupStep number={2} title="添加水杉输入法">
        点列表左下角的「+」，在左侧选「简体中文」，再选「水杉输入法」，然后点「添加」。
      </SetupStep>
      <SetupStep number={3} title="切换并开始输入" last>
        在菜单栏的输入法菜单中选「水杉输入法」，或按 Control+空格（或地球键）切换过去。
      </SetupStep>
    </div>
  );

  const systemNotice = (
    <p className={onboarding.note}>
      添加时 macOS
      会提示「开发者可以访问你通过此输入法键入的任何内容…」。这是系统对所有第三方输入法都会显示的标准提示，并不是水杉额外申请的权限。
      {client.openExternalUrl && (
        <>
          {" "}
          水杉如何处理数据见
          <button
            type="button"
            className="m-0 border-0 bg-transparent p-0 text-accent underline"
            onClick={() => void client.openExternalUrl?.(privacyUrl).catch(() => {})}
          >
            隐私政策
          </button>
          。
        </>
      )}
    </p>
  );

  return (
    <main
      className={`${onboarding.page} ${onboarding.buttons}`}
      aria-label="添加水杉输入法"
      data-onboarding-shell=""
      data-platform="macos"
    >
      <header className={onboarding.header}>
        <img src={logo} alt="" />
        <h1>水杉输入法</h1>
      </header>
      <div className={onboarding.body}>
        <section className={onboarding.section}>
          <h2 className={onboarding.sectionTitle}>{titles[stage]}</h2>
          {stage === "failed" && (
            <>
              <p className={onboarding.lead}>
                水杉输入法未能自动安装或更新。请进入设置，在「快捷键」页的「输入法服务」中点「安装 /
                更新」重试。
              </p>
              <p className={onboarding.note}>
                应用随附版本：{status.bundled_version ?? "无法读取"}；本机已安装版本：
                {status.installed_version ?? "未安装"}。
              </p>
            </>
          )}
          {stage === "system" && (
            <>
              <p className={onboarding.lead}>
                「/Library/Input
                Methods」里还有一份水杉输入法。它和你现在用的是同一个输入法，会让系统的输入法列表出现重复项，或用上其中较旧的版本。这个文件夹属于整台
                Mac，移除需要管理员密码，MSIME 不会替你操作。
              </p>
              <ul className={onboarding.note}>
                {status.system_bundles?.map((path) => (
                  <li key={path}>{path}</li>
                ))}
              </ul>
              <div className={onboarding.setupCard}>
                <SetupStep number={1} title="打开旧版所在的文件夹">
                  在「访达」中按 Shift+Command+G，输入 /Library/Input Methods 后前往。
                </SetupStep>
                <SetupStep number={2} title="移到废纸篓">
                  把上面列出的项目拖到废纸篓，按提示输入管理员密码。
                </SetupStep>
                <SetupStep number={3} title="注销并重新登录" last>
                  点屏幕左上角的苹果菜单，选「退出登录」，重新登录后再打开 MSIME，它会接着引导你。
                </SetupStep>
              </div>
            </>
          )}
          {stage === "login" && (
            <>
              <p className={onboarding.lead}>
                水杉输入法已经装到这台 Mac 上，但 macOS
                只在登录时读取新安装的输入法，本次登录的输入法列表里还找不到它。重新登录一次之后，再按下面的步骤添加；以后更新水杉不需要再重新登录。
              </p>
              <div className={onboarding.setupCard}>
                <SetupStep number={1} title="注销并重新登录">
                  点屏幕左上角的苹果菜单，选「退出登录」，然后重新登录这个账户。
                </SetupStep>
                <SetupStep number={2} title="重新打开 MSIME">
                  打开「应用程序」里的 MSIME，它会接着引导你完成添加。
                </SetupStep>
                <SetupStep number={3} title="在系统设置中添加" last>
                  在「系统设置」→「键盘」→「文字输入」→「输入法」中点「编辑…」，添加「简体中文」下的「水杉输入法」。
                </SetupStep>
              </div>
            </>
          )}
          {stage === "add" && (
            <>
              <p className={onboarding.lead}>
                {returning
                  ? "水杉输入法不在系统的输入法列表中了，可能是在系统设置里被移除了。按下面的步骤重新添加即可。"
                  : "水杉输入法已经装好。macOS 要求由你在系统设置里亲自添加第三方输入法，添加好后这里会自动进入下一步。"}
              </p>
              {systemNotice}
              {addSteps}
              <p className={onboarding.note} role="status">
                正在等待你在系统设置中添加水杉输入法…
              </p>
            </>
          )}
          {stage === "ready" && (
            <>
              <p className={onboarding.lead} role="status">
                已添加。按 Control+空格 或在菜单栏切换到水杉输入法，就可以开始输入了。
              </p>
              {addSteps}
            </>
          )}
        </section>
      </div>
      {error && (
        <p className={onboarding.error} role="alert">
          {error}
        </p>
      )}
      <footer className={onboarding.deskFooter}>
        {stage === "add" ? (
          <>
            <button type="button" className={onboarding.deskLink} onClick={onComplete}>
              稍后再说
            </button>
            <span className={onboarding.deskSpacer} />
            <button type="button" className="primary" onClick={openSettings}>
              打开键盘设置
            </button>
          </>
        ) : (
          <>
            <span className={onboarding.deskSpacer} />
            <button type="button" className="primary" onClick={onComplete}>
              进入设置
            </button>
          </>
        )}
      </footer>
    </main>
  );
}
