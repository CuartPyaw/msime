// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { VoiceModelPathSection } from "@msime/ui";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

test("edits the manual model path", () => {
  const onChange = vi.fn();
  render(<VoiceModelPathSection path="/old/voice-models/paraformer" onChange={onChange} />);

  fireEvent.change(screen.getByRole("textbox", { name: "本地模型目录" }), {
    target: { value: "/new/voice-models/sense-voice" },
  });
  expect(onChange).toHaveBeenCalledWith("/new/voice-models/sense-voice");
  expect(screen.queryByRole("button", { name: "选择…" })).toBeNull();
});

test("updates from a selected file and ignores cancellation", async () => {
  const onChange = vi.fn();
  const pickPath = vi
    .fn()
    .mockResolvedValueOnce("/picked/voice-models/sense-voice")
    .mockResolvedValueOnce(null);
  render(
    <VoiceModelPathSection
      path="/old/voice-models/paraformer"
      pickPath={pickPath}
      onChange={onChange}
    />,
  );

  fireEvent.click(screen.getByRole("button", { name: "选择…" }));
  await vi.waitFor(() => expect(onChange).toHaveBeenCalledWith("/picked/voice-models/sense-voice"));
  onChange.mockClear();
  fireEvent.click(screen.getByRole("button", { name: "选择…" }));
  await Promise.resolve();
  expect(onChange).not.toHaveBeenCalled();
});
