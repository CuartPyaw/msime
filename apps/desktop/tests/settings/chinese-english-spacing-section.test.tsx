// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { ChineseEnglishSpacingSection } from "../../../../packages/ui/src/settings/chinese-english-spacing-section";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

test("Chinese-English spacing switch reports the changed value", () => {
  const onChange = vi.fn();
  render(<ChineseEnglishSpacingSection value={false} onChange={onChange} />);

  fireEvent.click(screen.getByLabelText("中英文之间加空格"));
  expect(onChange).toHaveBeenCalledWith(true);
});

test("Chinese-English spacing switch is off when the document has no key", () => {
  render(<ChineseEnglishSpacingSection value={undefined} onChange={vi.fn()} />);

  expect((screen.getByLabelText("中英文之间加空格") as HTMLInputElement).checked).toBe(false);
});

test("Chinese-English spacing says what it leaves alone", () => {
  render(<ChineseEnglishSpacingSection value={false} onChange={vi.fn()} />);

  expect(
    screen.getByText(
      /两次上屏之间不加；网址、日期等快捷模式和全角输入不受影响，译文和释义上屏、常用语、剪贴板和语音结果也不加/,
    ),
  ).toBeTruthy();
});

test("跨次上屏独立开关保留原有开关", () => {
  const changed = vi.fn();
  render(<ChineseEnglishSpacingSection onChange={vi.fn()} onCrossCommitChange={changed} />);
  expect((screen.getByLabelText("跨次上屏中英文空格") as HTMLInputElement).checked).toBe(false);
  fireEvent.click(screen.getByLabelText("跨次上屏中英文空格"));
  expect(changed).toHaveBeenCalledWith({ enabled: true, excluded_apps: [] });
  expect((screen.getByLabelText("中英文之间加空格") as HTMLInputElement).checked).toBe(false);
});

test("跨次空格排除应用可以添加和移除", () => {
  const changed = vi.fn();
  const { rerender } = render(
    <ChineseEnglishSpacingSection
      onChange={vi.fn()}
      crossCommit={{ enabled: true, excluded_apps: [] }}
      onCrossCommitChange={changed}
    />,
  );
  fireEvent.change(screen.getByLabelText("应用 Bundle ID"), {
    target: { value: "com.example.Editor" },
  });
  fireEvent.click(screen.getByRole("button", { name: "添加" }));
  expect(changed).toHaveBeenCalledWith({ enabled: true, excluded_apps: ["com.example.Editor"] });
  rerender(
    <ChineseEnglishSpacingSection
      onChange={vi.fn()}
      crossCommit={{ enabled: true, excluded_apps: ["com.example.Editor"] }}
      onCrossCommitChange={changed}
    />,
  );
  fireEvent.click(
    screen.getByRole("button", { name: "把 com.example.Editor 从跨次空格排除名单移除" }),
  );
  expect(changed).toHaveBeenLastCalledWith({ enabled: true, excluded_apps: [] });
});
