// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { InputModeSection } from "@msime/ui";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

test("switching to Chinese restores the remembered Chinese scheme", () => {
  const onChange = vi.fn();
  render(<InputModeSection scheme="japanese" lastChineseScheme="shuangpin" onChange={onChange} />);

  fireEvent.click(screen.getByLabelText("中文"));
  expect(onChange).toHaveBeenCalledWith({ scheme: "shuangpin" });
});

test("switching to Japanese remembers the current Chinese scheme", () => {
  const onChange = vi.fn();
  render(<InputModeSection scheme="wubi" lastChineseScheme="wubi" onChange={onChange} />);

  fireEvent.click(screen.getByLabelText("日文"));
  expect(onChange).toHaveBeenCalledWith({ last_chinese_scheme: "wubi", scheme: "japanese" });
});

test("switching to Korean remembers the current Chinese scheme", () => {
  const onChange = vi.fn();
  render(<InputModeSection scheme="shuangpin" lastChineseScheme="quanpin" onChange={onChange} />);

  fireEvent.click(screen.getByLabelText("韩文"));
  expect(onChange).toHaveBeenCalledWith({ last_chinese_scheme: "shuangpin", scheme: "korean" });
});

test("switching between Japanese and Korean keeps the remembered Chinese scheme", () => {
  const onChange = vi.fn();
  render(<InputModeSection scheme="japanese" lastChineseScheme="wubi" onChange={onChange} />);

  fireEvent.click(screen.getByLabelText("韩文"));
  expect(onChange).toHaveBeenCalledWith({ last_chinese_scheme: "wubi", scheme: "korean" });
});

test("switching from Korean to Chinese restores the remembered Chinese scheme", () => {
  const onChange = vi.fn();
  render(<InputModeSection scheme="korean" lastChineseScheme="wubi" onChange={onChange} />);

  expect((screen.getByLabelText("韩文") as HTMLInputElement).checked).toBe(true);
  fireEvent.click(screen.getByLabelText("中文"));
  expect(onChange).toHaveBeenCalledWith({ scheme: "wubi" });
});
