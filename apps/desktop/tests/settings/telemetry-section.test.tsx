// @vitest-environment jsdom
import { afterEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { TelemetrySection, usageReportingDescription, usageReportingSummary } from "@msime/ui";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

test("usage reporting switch reports the changed value", () => {
  const onChange = vi.fn();
  render(<TelemetrySection value={true} onChange={onChange} />);

  fireEvent.click(screen.getByLabelText("匿名使用统计"));
  expect(onChange).toHaveBeenCalledWith(false);
});

test("usage reporting is on when the preference is absent", () => {
  render(<TelemetrySection value={undefined} onChange={vi.fn()} />);

  expect((screen.getByLabelText("匿名使用统计") as HTMLInputElement).checked).toBe(true);
});

test("the switch carries one line and the full disclosure folds below it", () => {
  render(<TelemetrySection value={true} onChange={vi.fn()} />);

  expect(screen.getByText(usageReportingSummary)).toBeTruthy();
  const details = screen.getByText("发送哪些内容").closest("details");
  expect(details?.open).toBe(false);
  // The whole text stays in the page, word for word as PRIVACY.md has it, only folded away.
  expect(details?.textContent).toContain(usageReportingDescription);
  expect(usageReportingDescription).toContain("https://api.msime.app/v1/telemetry/events");
});

test("usage reporting shows off once the user turned it off", () => {
  render(<TelemetrySection value={false} onChange={vi.fn()} />);

  expect((screen.getByLabelText("匿名使用统计") as HTMLInputElement).checked).toBe(false);
});
