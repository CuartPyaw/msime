// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { ClipboardHistorySection } from "@msime/ui";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

test("clipboard history loads entries and toggles a pin", async () => {
  const setPinned = vi.fn().mockResolvedValue(undefined);
  const clear = vi.fn().mockResolvedValue(undefined);
  const list = vi
    .fn()
    .mockResolvedValueOnce([{ text: "合成测试", timestampMs: 1_700_000_000_000, pinned: false }])
    .mockResolvedValueOnce([{ text: "合成测试", timestampMs: 1_700_000_000_000, pinned: true }]);

  render(
    <ClipboardHistorySection
      client={{ clear, list, setPinned }}
      historyEnabled
      persistedHistoryEnabled
      revision={1}
      ios={false}
      onToggle={vi.fn()}
      onError={vi.fn()}
    />,
  );

  expect(await screen.findByText("合成测试")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "固定剪贴板记录" }));

  await waitFor(() => expect(setPinned).toHaveBeenCalledWith("合成测试", true));
  expect(await screen.findByRole("button", { name: "取消固定剪贴板记录" })).toBeTruthy();
});

test("a clipboard mutation from a replaced client cannot restore stale entries", async () => {
  let resolvePin!: () => void;
  const oldClient = {
    clear: vi.fn().mockResolvedValue(undefined),
    list: vi
      .fn()
      .mockResolvedValue([{ text: "旧记录", timestampMs: 1_700_000_000_000, pinned: false }]),
    setPinned: vi.fn(
      () =>
        new Promise<void>((resolve) => {
          resolvePin = resolve;
        }),
    ),
  };
  const nextClient = {
    clear: vi.fn().mockResolvedValue(undefined),
    list: vi
      .fn()
      .mockResolvedValue([{ text: "新记录", timestampMs: 1_700_000_000_001, pinned: false }]),
    setPinned: vi.fn().mockResolvedValue(undefined),
  };
  const view = render(
    <ClipboardHistorySection
      client={oldClient}
      historyEnabled
      persistedHistoryEnabled
      revision={1}
      ios={false}
      onToggle={vi.fn()}
      onError={vi.fn()}
    />,
  );
  await screen.findByText("旧记录");
  fireEvent.click(screen.getByRole("button", { name: "固定剪贴板记录" }));
  await waitFor(() => expect(oldClient.setPinned).toHaveBeenCalledWith("旧记录", true));

  view.rerender(
    <ClipboardHistorySection
      client={nextClient}
      historyEnabled
      persistedHistoryEnabled
      revision={1}
      ios={false}
      onToggle={vi.fn()}
      onError={vi.fn()}
    />,
  );
  await screen.findByText("新记录");
  await act(async () => {
    resolvePin();
    await Promise.resolve();
    await Promise.resolve();
  });
  expect(screen.queryByText("旧记录")).toBeNull();
  expect(screen.getByText("新记录")).toBeTruthy();
});
