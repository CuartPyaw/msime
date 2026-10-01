// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { InputSchemeSelectorSection } from "@msime/ui";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

test("shows the selected scheme and forwards radio changes", () => {
  const onChange = vi.fn();
  render(<InputSchemeSelectorSection value="quanpin" onChange={onChange} />);

  expect((screen.getByRole("radio", { name: "全拼" }) as HTMLInputElement).checked).toBe(true);
  fireEvent.click(screen.getByRole("radio", { name: "双拼" }));
  expect(onChange).toHaveBeenCalledWith("shuangpin");
  expect(screen.getByRole("group", { name: "输入方案" })).toBeTruthy();
});

test("grouped mode renders the selector in a platform row and honors hidden state", () => {
  const onChange = vi.fn();
  const { rerender } = render(
    <InputSchemeSelectorSection grouped value="quanpin" onChange={onChange} />,
  );

  expect(screen.getByRole("radiogroup")).toBeTruthy();
  fireEvent.click(screen.getByRole("radio", { name: "双拼" }));
  expect(onChange).toHaveBeenCalledWith("shuangpin");

  rerender(<InputSchemeSelectorSection grouped hidden value="quanpin" onChange={onChange} />);
  expect(screen.getByText("输入方案").closest("div")?.hasAttribute("hidden")).toBe(true);
});

const allSchemes = [
  "quanpin",
  "shuangpin",
  "wubi",
  "japanese",
  "korean",
  "cantonese",
  "zhuyin",
  "vietnamese",
] as const;

test("offers 粤拼 and 注音 where the host supports them", () => {
  const onChange = vi.fn();
  render(
    <InputSchemeSelectorSection
      grouped
      value="quanpin"
      supportedSchemes={allSchemes}
      onChange={onChange}
    />,
  );

  expect(screen.getAllByRole("radio").map((radio) => radio.closest("label")?.textContent)).toEqual([
    "全拼",
    "双拼",
    "五笔",
    "粤拼",
    "注音",
  ]);
  fireEvent.click(screen.getByRole("radio", { name: "粤拼" }));
  expect(onChange).toHaveBeenCalledWith("cantonese");
  fireEvent.click(screen.getByRole("radio", { name: "注音" }));
  expect(onChange).toHaveBeenCalledWith("zhuyin");
  expect(screen.queryByText(/此平台暂不支持/)).toBeNull();
});

test("schemes the host does not offer are disabled with the reason", () => {
  for (const grouped of [true, false]) {
    render(<InputSchemeSelectorSection grouped={grouped} value="wubi" onChange={vi.fn()} />);
    expect((screen.getByRole("radio", { name: "粤拼" }) as HTMLInputElement).disabled).toBe(true);
    expect((screen.getByRole("radio", { name: "注音" }) as HTMLInputElement).disabled).toBe(true);
    expect((screen.getByRole("radio", { name: "五笔" }) as HTMLInputElement).disabled).toBe(false);
    expect(screen.getByText("此平台暂不支持粤拼、注音")).toBeTruthy();
    cleanup();
  }
});

test("an unsupported selected scheme stays selected and names the fallback", () => {
  render(
    <InputSchemeSelectorSection
      grouped
      value="cantonese"
      lastChineseScheme="shuangpin"
      onChange={vi.fn()}
    />,
  );

  const cantonese = screen.getByRole("radio", { name: "粤拼" }) as HTMLInputElement;
  expect(cantonese.checked).toBe(true);
  expect(cantonese.disabled).toBe(true);
  expect(screen.getByText("此平台暂不支持粤拼、注音，已回退到双拼")).toBeTruthy();
});
