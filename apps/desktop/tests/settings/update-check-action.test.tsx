// @vitest-environment jsdom
import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { useUpdateCheck } from "@msime/ui";

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((accept) => {
    resolve = accept;
  });
  return { promise, resolve };
}

test("an update check already in progress ignores a second trigger", async () => {
  const request = deferred<Response>();
  const fetch = vi.fn().mockReturnValue(request.promise);
  vi.stubGlobal("fetch", fetch);
  const { result } = renderHook(() =>
    useUpdateCheck({
      clientHostedPlatform: false,
      releasePlatform: null,
      releasePageUrl: "https://updates.example.test/releases",
      currentAppVersion: "1.0.0",
    }),
  );

  let first!: Promise<void>;
  act(() => {
    first = result.current.checkForUpdate();
  });
  await waitFor(() => expect(result.current.busy).toBe(true));

  let second!: Promise<void>;
  act(() => {
    second = result.current.checkForUpdate();
  });
  expect(fetch).toHaveBeenCalledOnce();

  request.resolve({ ok: false, status: 503 } as Response);
  await act(async () => {
    await first;
    await second;
  });
  expect(result.current.status).toBe("检查失败，请稍后重试");
});

test("a check started for an older app version is ignored after the version changes", async () => {
  const request = deferred<Response>();
  vi.stubGlobal("fetch", vi.fn().mockReturnValue(request.promise));
  const { result, rerender } = renderHook(
    ({ currentAppVersion }) =>
      useUpdateCheck({
        clientHostedPlatform: false,
        releasePlatform: null,
        releasePageUrl: "https://updates.example.test/releases",
        currentAppVersion,
      }),
    { initialProps: { currentAppVersion: "1.0.0" } },
  );

  let pending!: Promise<void>;
  act(() => {
    pending = result.current.checkForUpdate();
  });
  await waitFor(() => expect(result.current.busy).toBe(true));
  rerender({ currentAppVersion: "2.0.0" });
  request.resolve({
    ok: true,
    json: async () => ({
      version: "1.5.0",
      releaseUrl: "https://updates.example.test/releases/v1.5.0",
    }),
  } as Response);
  await act(async () => pending);

  expect(result.current.available).toBeNull();
  expect(result.current.status).toBe("");
  expect(result.current.busy).toBe(false);
});
