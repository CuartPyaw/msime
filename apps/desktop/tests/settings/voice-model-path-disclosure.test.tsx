// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import { VoiceModelPathDisclosure } from "@msime/ui";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

test.each([true, false])("renders the model path editor with disclosure=%s", (disclosure) => {
  render(
    <VoiceModelPathDisclosure
      disclosure={disclosure}
      path="/synthetic/voice-models/sense-voice"
      onChange={vi.fn()}
    />,
  );

  expect(screen.getByLabelText("本地模型目录")).toBeTruthy();
  if (disclosure) {
    expect(screen.getByText("高级：手动指定本地模型目录")).toBeTruthy();
  } else {
    expect(screen.queryByText("高级：手动指定本地模型目录")).toBeNull();
  }
});
