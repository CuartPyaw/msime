// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { InputSourceStartupNotice, type InputSourceStartupStatus } from "@msime/ui";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const installed: InputSourceStartupStatus = {
  action: "installed",
  enabled: true,
  bundled_version: "1.0.0",
  installed_version: "1.0.0",
};

test("shows the installation result and can be dismissed", () => {
  const onDismiss = vi.fn();
  render(
    <InputSourceStartupNotice
      status={installed}
      onOpenSettings={vi.fn()}
      onDismiss={onDismiss}
      onError={vi.fn()}
    />,
  );

  expect(screen.getByRole("status", { name: "水杉输入法安装状态" }).textContent).toContain(
    "水杉输入法 1.0.0 已安装",
  );
  fireEvent.click(screen.getByRole("button", { name: "知道了" }));
  expect(onDismiss).toHaveBeenCalledOnce();
});

test("reports a failure when opening keyboard settings fails", async () => {
  const onError = vi.fn();
  render(
    <InputSourceStartupNotice
      status={{ ...installed, enabled: false }}
      onOpenSettings={() => Promise.reject(new Error("settings unavailable"))}
      onDismiss={vi.fn()}
      onError={onError}
    />,
  );

  fireEvent.click(screen.getByRole("button", { name: "打开键盘设置" }));
  await vi.waitFor(() =>
    expect(onError).toHaveBeenCalledWith(
      "无法打开系统设置，请手动前往「系统设置 › 键盘 › 文字输入 › 输入法」。",
    ),
  );
});

test("does not offer enabling or keyboard settings after a failed install", () => {
  render(
    <InputSourceStartupNotice
      status={{ ...installed, action: "failed", enabled: false }}
      onOpenSettings={vi.fn()}
      onEnable={vi.fn()}
      onDismiss={vi.fn()}
      onError={vi.fn()}
    />,
  );

  const banner = screen.getByRole("alert", { name: "水杉输入法安装状态" });
  expect(banner.textContent).toContain("水杉输入法没能自动安装或更新");
  expect(screen.queryByRole("button", { name: "启用水杉输入法" })).toBeNull();
  expect(screen.queryByRole("button", { name: "打开键盘设置" })).toBeNull();
});

test("asks for an install when there is no installed copy to enable", async () => {
  const onError = vi.fn();
  render(
    <InputSourceStartupNotice
      status={{ ...installed, action: "up_to_date", enabled: false }}
      onOpenSettings={vi.fn()}
      onEnable={() => Promise.reject({ code: "not_installed" })}
      onDismiss={vi.fn()}
      onError={onError}
    />,
  );

  fireEvent.click(screen.getByRole("button", { name: "启用水杉输入法" }));
  await vi.waitFor(() =>
    expect(onError).toHaveBeenCalledWith(
      "没有找到已安装的水杉输入法，请在「快捷键」页的「输入法服务」中点「安装 / 更新」。",
    ),
  );
});
