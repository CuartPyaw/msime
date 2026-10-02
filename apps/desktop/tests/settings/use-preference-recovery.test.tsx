// @vitest-environment jsdom
import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { usePreferenceRecovery, type Preferences, type Snapshot } from "@msime/ui";

afterEach(cleanup);

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((accept) => {
    resolve = accept;
  });
  return { promise, resolve };
}

const preferences = {} as Preferences;
const snapshot = { revision: 1, preferences } as Snapshot;

test("ignores a same-tick duplicate restore-defaults action", async () => {
  const pending = deferred<Preferences>();
  const loadDefaultPreferences = vi.fn().mockReturnValue(pending.promise);
  const { result } = renderHook(() =>
    usePreferenceRecovery({
      client: { loadDefaultPreferences },
      busy: false,
      snapshotRef: { current: snapshot },
      draftRef: { current: preferences },
      setSnapshot: vi.fn(),
      setDraft: vi.fn(),
      setBusy: vi.fn(),
      setError: vi.fn(),
      setNotice: vi.fn(),
      setRecoveredBackup: vi.fn(),
      confirm: vi.fn().mockResolvedValue(true),
    }),
  );

  let first!: Promise<void>;
  let second!: Promise<void>;
  act(() => {
    first = result.current.restoreDefaults();
    second = result.current.restoreDefaults();
  });
  await act(async () => {
    await Promise.resolve();
  });
  expect(loadDefaultPreferences).toHaveBeenCalledOnce();
  pending.resolve(preferences);
  await act(async () => {
    await first;
    await second;
  });
});
