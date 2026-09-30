import { expect, test } from "vitest";
import { availableSettingsPages, type AvailablePageCapabilities } from "@msime/ui";

const capabilities: AvailablePageCapabilities = {
  home: false,
  typingStatistics: false,
  vocabularyReview: false,
  account: false,
  chat: false,
  community: false,
  floatingToolbar: false,
  plugins: false,
  mobile: false,
};

test("keeps common settings pages while hiding unavailable host entries", () => {
  const ids = availableSettingsPages(capabilities).map((page) => page.id);

  expect(ids).toContain("appearance");
  expect(ids).toContain("input");
  expect(ids).not.toContain("home");
  expect(ids).not.toContain("vocabulary");
  expect(ids).not.toContain("community");
  expect(ids).not.toContain("floating-toolbar");
  expect(ids).not.toContain("plugins");
  expect(ids).not.toContain("more");
  expect(
    availableSettingsPages({ ...capabilities, plugins: true }).map((page) => page.id),
  ).toContain("plugins");
});

test("exposes only the host-backed and mobile entries that are available", () => {
  const ids = availableSettingsPages({
    ...capabilities,
    home: true,
    account: true,
    vocabularyReview: true,
    mobile: true,
  }).map((page) => page.id);

  expect(ids).toContain("home");
  expect(ids).toContain("account");
  expect(ids).toContain("vocabulary");
  expect(ids).toContain("more");
  expect(ids).not.toContain("chat");
  expect(ids).not.toContain("typing-statistics");
});
