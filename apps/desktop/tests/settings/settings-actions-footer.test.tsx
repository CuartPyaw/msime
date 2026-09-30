// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { SettingsActionsFooter } from "@msime/ui";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

test("renders dirty state and dispatches restore and save actions", () => {
  const onRestoreDefaults = vi.fn();
  const onSave = vi.fn();
  render(
    <form
      onSubmit={(event) => {
        event.preventDefault();
        onSave();
      }}
    >
      <SettingsActionsFooter
        busy={false}
        dirty
        canSave
        showRestoreDefaults
        onRestoreDefaults={onRestoreDefaults}
      />
    </form>,
  );

  expect(screen.getByText("有未保存的修改")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "恢复默认设置" }));
  fireEvent.click(screen.getByRole("button", { name: "保存设置" }));
  expect(onRestoreDefaults).toHaveBeenCalledTimes(1);
  expect(onSave).toHaveBeenCalledTimes(1);
});

test("draws 重新读取 in the action row and only 保存设置 as the primary button", () => {
  const onReload = vi.fn();
  render(
    <SettingsActionsFooter
      busy={false}
      dirty
      canSave
      showRestoreDefaults
      onRestoreDefaults={vi.fn()}
      onReload={onReload}
    />,
  );
  const footer = screen.getByRole("contentinfo");
  // Secondary actions first, then the status that takes the free space, then 保存设置.
  expect(Array.from(footer.children).map((child) => child.textContent)).toEqual([
    "重新读取",
    "恢复默认设置",
    "有未保存的修改",
    "保存设置",
  ]);
  fireEvent.click(screen.getByRole("button", { name: "重新读取" }));
  expect(onReload).toHaveBeenCalledTimes(1);
  // A bare `[&>button]` fill painted the secondary actions green over their `secondary` look.
  expect(footer.className).not.toMatch(/\[&>button\]:bg-/);
  expect(footer.className).toContain("[&>button[type=submit]]:bg-accent-strong");
  for (const name of ["重新读取", "恢复默认设置"])
    expect(screen.getByRole("button", { name }).className).toBe("secondary");
});

test("leaves 重新读取 out when the page cannot reload", () => {
  render(
    <SettingsActionsFooter
      busy={false}
      dirty={false}
      canSave
      showRestoreDefaults
      onRestoreDefaults={vi.fn()}
    />,
  );
  expect(screen.queryByRole("button", { name: "重新读取" })).toBeNull();
});

test("disables actions while busy or when there are no valid edits", () => {
  render(
    <SettingsActionsFooter
      busy
      dirty={false}
      canSave={false}
      showRestoreDefaults
      onRestoreDefaults={vi.fn()}
    />,
  );

  expect((screen.getByRole("button", { name: "恢复默认设置" }) as HTMLButtonElement).disabled).toBe(
    true,
  );
  expect((screen.getByRole("button", { name: "处理中…" }) as HTMLButtonElement).disabled).toBe(
    true,
  );
});
