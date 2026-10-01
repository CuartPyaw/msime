// @vitest-environment jsdom
import { act, renderHook, waitFor } from "@testing-library/react";
import { expect, test, vi } from "vitest";
import { useDataDirectory, type DataDirectoryClient } from "@msime/ui";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((accept) => {
    resolve = accept;
  });
  return { promise, resolve };
}

test("a move response from a replaced data directory client is ignored", async () => {
  const pendingMove = deferred<Awaited<ReturnType<DataDirectoryClient["move"]>>>();
  const oldClient: DataDirectoryClient = {
    status: vi.fn().mockResolvedValue({ path: "/old", isDefault: true }),
    pick: vi.fn().mockResolvedValue("/target"),
    move: vi.fn().mockReturnValue(pendingMove.promise),
  };
  const nextClient: DataDirectoryClient = {
    status: vi.fn().mockResolvedValue({ path: "/new", isDefault: false }),
    pick: vi.fn(),
    move: vi.fn(),
  };
  const confirm = vi.fn().mockResolvedValue(true);
  const { result, rerender } = renderHook(
    ({ client }) => useDataDirectory({ client, enabled: true, confirm }),
    { initialProps: { client: oldClient } },
  );
  await waitFor(() => expect(result.current.dataDirectory?.path).toBe("/old"));

  let pending!: Promise<void>;
  act(() => {
    pending = result.current.choose();
  });
  await waitFor(() => expect(oldClient.move).toHaveBeenCalledOnce());
  rerender({ client: nextClient });
  await waitFor(() => expect(result.current.dataDirectory?.path).toBe("/new"));

  pendingMove.resolve({
    path: "/old-result",
    isDefault: false,
    retainedOldData: false,
  });
  await act(async () => pending);
  expect(result.current.dataDirectory?.path).toBe("/new");
});
