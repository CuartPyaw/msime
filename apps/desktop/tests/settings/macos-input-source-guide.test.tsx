// @vitest-environment jsdom
import { afterEach, beforeEach, expect, test, vi, type Mock } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import {
  INPUT_SOURCE_RECHECK_MS,
  MacosInputSourceGuide,
  macosInputSourceGuideNeeded,
  type InputSourceStartupStatus,
  type MacosInputSourceGuideClient,
} from "@msime/ui";

const COMPLETED_KEY = "msime.macos.input-source-guide.completed";

beforeEach(() => window.localStorage.clear());

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

const installed: InputSourceStartupStatus = {
  action: "installed",
  enabled: false,
  bundled_version: "0.51.0 (7300)",
  installed_version: "0.51.0 (7300)",
  system_bundles: [],
};

function guideClient(
  overrides: {
    status?: Mock<MacosInputSourceGuideClient["status"]>;
    openSettings?: Mock<MacosInputSourceGuideClient["openSettings"]>;
  } = {},
) {
  return {
    status:
      overrides.status ??
      vi.fn<MacosInputSourceGuideClient["status"]>().mockResolvedValue(installed),
    openSettings:
      overrides.openSettings ??
      vi.fn<MacosInputSourceGuideClient["openSettings"]>().mockResolvedValue(),
    openExternalUrl: vi
      .fn<NonNullable<MacosInputSourceGuideClient["openExternalUrl"]>>()
      .mockResolvedValue(),
  };
}

test("decides which start-time results need the guide", () => {
  expect(macosInputSourceGuideNeeded(null)).toBe(false);
  expect(macosInputSourceGuideNeeded({ ...installed, enabled: true })).toBe(false);
  // An unreadable list gives the guide nothing to wait for.
  expect(macosInputSourceGuideNeeded({ ...installed, enabled: null })).toBe(false);
  expect(macosInputSourceGuideNeeded(installed)).toBe(true);
  expect(
    macosInputSourceGuideNeeded({ ...installed, action: "login_required", enabled: null }),
  ).toBe(true);
  // Not installed: the install failed and nothing is in place.
  expect(
    macosInputSourceGuideNeeded({
      ...installed,
      action: "failed",
      enabled: null,
      installed_version: null,
    }),
  ).toBe(true);
  // A failed update behind a working copy is the settings page's to report.
  expect(macosInputSourceGuideNeeded({ ...installed, action: "failed", enabled: true })).toBe(
    false,
  );
  expect(
    macosInputSourceGuideNeeded({
      ...installed,
      enabled: true,
      system_bundles: ["/Library/Input Methods/MetasequoiaIME.app"],
    }),
  ).toBe(true);
});

test("not installed: shows the failure with both versions and lets the user continue", () => {
  const onComplete = vi.fn();
  const client = guideClient();
  render(
    <MacosInputSourceGuide
      status={{ ...installed, action: "failed", enabled: null, installed_version: null }}
      client={client}
      onComplete={onComplete}
    />,
  );

  expect(screen.getByRole("heading", { name: "水杉输入法未能安装" })).toBeTruthy();
  expect(screen.getByText(/「安装 \/\s*更新」重试/)).toBeTruthy();
  expect(
    screen.getByText(/应用随附版本：0\.51\.0 \(7300\)；本机已安装版本：\s*未安装/),
  ).toBeTruthy();
  expect(screen.queryByRole("button", { name: "打开键盘设置" })).toBeNull();
  fireEvent.click(screen.getByRole("button", { name: "进入设置" }));
  expect(onComplete).toHaveBeenCalledOnce();
  expect(client.status).not.toHaveBeenCalled();
});

test("failed: reports the version still installed", () => {
  render(
    <MacosInputSourceGuide
      status={{
        ...installed,
        action: "failed",
        enabled: false,
        installed_version: "0.50.0 (7000)",
      }}
      client={guideClient()}
      onComplete={vi.fn()}
    />,
  );
  expect(screen.getByText(/本机已安装版本：\s*0\.50\.0 \(7000\)/)).toBeTruthy();
});

test("login_required: asks for a new login first, with no settings button to press yet", async () => {
  vi.useFakeTimers();
  const client = guideClient();
  render(
    <MacosInputSourceGuide
      status={{ ...installed, action: "login_required", enabled: null }}
      client={client}
      onComplete={vi.fn()}
    />,
  );

  expect(screen.getByRole("heading", { name: "注销并重新登录" })).toBeTruthy();
  expect(screen.getByText(/只在登录时读取新安装的输入法/)).toBeTruthy();
  expect(screen.getByText(/选「退出登录」/)).toBeTruthy();
  expect(screen.queryByRole("button", { name: "打开键盘设置" })).toBeNull();
  await act(async () => vi.advanceTimersByTime(INPUT_SOURCE_RECHECK_MS * 2));
  expect(client.status).not.toHaveBeenCalled();
});

test("not enabled: walks through System Settings and moves on once a focus re-check finds the source", async () => {
  const onComplete = vi.fn();
  const client = guideClient();
  render(<MacosInputSourceGuide status={installed} client={client} onComplete={onComplete} />);

  expect(screen.getByRole("heading", { name: "把水杉输入法添加到系统" })).toBeTruthy();
  expect(screen.getByText(/在「文字输入」下的「输入法」一行点「编辑…」/)).toBeTruthy();
  expect(
    screen.getByText(/左下角的「\+」，在左侧选「简体中文」，再选「水杉输入法」，然后点「添加」/),
  ).toBeTruthy();
  expect(screen.getByText(/开发者可以访问你通过此输入法键入的任何内容/)).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "隐私政策" }));
  expect(client.openExternalUrl).toHaveBeenCalledWith("https://msime.app/privacy/");

  fireEvent.click(screen.getByRole("button", { name: "打开键盘设置" }));
  expect(client.openSettings).toHaveBeenCalledOnce();

  // Still missing on the first return from System Settings.
  await act(async () => window.dispatchEvent(new Event("focus")));
  expect(client.status).toHaveBeenCalledOnce();
  expect(screen.getByRole("heading", { name: "把水杉输入法添加到系统" })).toBeTruthy();

  client.status.mockResolvedValue({ ...installed, enabled: true });
  await act(async () => window.dispatchEvent(new Event("focus")));
  await waitFor(() =>
    expect(screen.getByRole("heading", { name: "已添加水杉输入法" })).toBeTruthy(),
  );
  expect(
    screen.getByText("已添加。按 Control+空格 或在菜单栏切换到水杉输入法，就可以开始输入了。"),
  ).toBeTruthy();
  expect(window.localStorage.getItem(COMPLETED_KEY)).toBe("1");

  fireEvent.click(screen.getByRole("button", { name: "进入设置" }));
  expect(onComplete).toHaveBeenCalledOnce();
});

test("not enabled: also re-checks on a timer while waiting, and stops once added", async () => {
  vi.useFakeTimers();
  const client = guideClient();
  client.status.mockResolvedValue({ ...installed, enabled: true });
  render(<MacosInputSourceGuide status={installed} client={client} onComplete={vi.fn()} />);

  await act(async () => vi.advanceTimersByTime(INPUT_SOURCE_RECHECK_MS - 1));
  expect(client.status).not.toHaveBeenCalled();
  await act(async () => vi.advanceTimersByTime(1));
  expect(client.status).toHaveBeenCalledOnce();
  expect(screen.getByRole("heading", { name: "已添加水杉输入法" })).toBeTruthy();
  await act(async () => vi.advanceTimersByTime(INPUT_SOURCE_RECHECK_MS * 3));
  expect(client.status).toHaveBeenCalledOnce();
});

test("not enabled: a failed re-check keeps waiting, and a failure to open System Settings is shown", async () => {
  const client = guideClient({
    status: vi
      .fn<MacosInputSourceGuideClient["status"]>()
      .mockRejectedValue(new Error("unavailable")),
    openSettings: vi
      .fn<MacosInputSourceGuideClient["openSettings"]>()
      .mockRejectedValue(new Error("unavailable")),
  });
  const onComplete = vi.fn();
  render(<MacosInputSourceGuide status={installed} client={client} onComplete={onComplete} />);

  await act(async () => window.dispatchEvent(new Event("focus")));
  expect(screen.getByRole("heading", { name: "把水杉输入法添加到系统" })).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "打开键盘设置" }));
  expect((await screen.findByRole("alert")).textContent).toBe(
    "无法打开系统设置，请手动打开「系统设置」→「键盘」。",
  );
  fireEvent.click(screen.getByRole("button", { name: "稍后再说" }));
  expect(onComplete).toHaveBeenCalledOnce();
});

test("a source removed after an earlier setup is described as removed", () => {
  window.localStorage.setItem(COMPLETED_KEY, "1");
  render(<MacosInputSourceGuide status={installed} client={guideClient()} onComplete={vi.fn()} />);
  expect(screen.getByText(/可能是在系统设置里被移除了/)).toBeTruthy();
});

test("a copy in /Library/Input Methods: asks the user to remove it and moves on once it is gone", async () => {
  const systemCopy = "/Library/Input Methods/MetasequoiaIME.app";
  const client = guideClient();
  client.status.mockResolvedValue({ ...installed, enabled: true });
  render(
    <MacosInputSourceGuide
      status={{ ...installed, enabled: true, system_bundles: [systemCopy] }}
      client={client}
      onComplete={vi.fn()}
    />,
  );

  expect(screen.getByRole("heading", { name: "移除多余的水杉输入法副本" })).toBeTruthy();
  expect(screen.getByText(systemCopy)).toBeTruthy();
  expect(screen.getByText(/Shift\+Command\+G/)).toBeTruthy();
  expect(screen.getByText(/选「退出登录」/)).toBeTruthy();

  await act(async () => window.dispatchEvent(new Event("focus")));
  await waitFor(() =>
    expect(screen.getByRole("heading", { name: "已添加水杉输入法" })).toBeTruthy(),
  );
});
