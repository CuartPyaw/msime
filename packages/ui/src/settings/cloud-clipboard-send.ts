import { errorCode } from "../core/error-code";
import type { CloudClipboardAction, CloudClipboardItem } from "../keyboard/panels";

/** One cloud clipboard request, the same actions the 云剪贴板 panel sends, issued by a host for the settings window itself. */
export type CloudClipboardRequest = (
  action: CloudClipboardAction,
) => Promise<{ items?: CloudClipboardItem[]; enabled?: boolean }>;

/** The longest text the cloud clipboard stores, counted in UTF-16 code units the way the server counts it. */
export const CLOUD_CLIPBOARD_MAX_UTF16 = 4000;
export const CLOUD_CLIPBOARD_SIGNED_OUT = "登录水杉账号后可在设备间同步剪贴板";
export const CLOUD_CLIPBOARD_DISABLED = "云剪贴板未开启";
export const CLOUD_CLIPBOARD_UNAVAILABLE = "云剪贴板服务暂不可用，请稍后重试";

/** Whether this window can send to the cloud clipboard right now: `ready` only when the account is signed in and the server reports the clipboard enabled. */
export type CloudClipboardAvailability =
  | "checking"
  | "ready"
  | "signed_out"
  | "disabled"
  | "unavailable";

/** Reads a failed request: an unauthorized account means signed out (or a session that has expired), anything else means the service could not be reached. */
export function cloudClipboardFailure(error: unknown): "signed_out" | "unavailable" {
  const code = errorCode(error);
  return code === "unauthorized" || code?.endsWith("_unauthorized") ? "signed_out" : "unavailable";
}

/** The note shown for an availability that blocks sending; `undefined` when there is nothing to explain. */
export function cloudClipboardAvailabilityNote(
  availability: CloudClipboardAvailability,
): string | undefined {
  switch (availability) {
    case "signed_out":
      return CLOUD_CLIPBOARD_SIGNED_OUT;
    case "disabled":
      return CLOUD_CLIPBOARD_DISABLED;
    case "unavailable":
      return CLOUD_CLIPBOARD_UNAVAILABLE;
  }
  return undefined;
}
