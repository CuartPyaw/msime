import { useEffect, useRef, useState } from "react";
import type {
  MobileKeyboardFeedback,
  MobileKeyboardFeedbackClient,
} from "./mobile-keyboard-feedback-section";

export interface UseMobileKeyboardFeedbackOptions {
  mobile: boolean;
  client?: MobileKeyboardFeedbackClient;
  onError: (message: string) => void;
}

/** Loads and persists native mobile keyboard sound, haptic, and suggestion preferences. */
export function useMobileKeyboardFeedback({
  mobile,
  client,
  onError,
}: UseMobileKeyboardFeedbackOptions) {
  const [value, setValue] = useState<MobileKeyboardFeedback>();
  const [busy, setBusy] = useState(false);
  const generation = useRef(0);

  useEffect(() => {
    const current = ++generation.current;
    setBusy(false);
    if (!mobile || !client) {
      setValue(undefined);
      return () => {
        if (generation.current === current) generation.current++;
      };
    }
    let active = true;
    void client
      .load()
      .then((next) => {
        if (active) setValue(next);
      })
      .catch(() => {
        if (active) onError("无法读取按键反馈设置，请重试。");
      });
    return () => {
      active = false;
      if (generation.current === current) generation.current++;
    };
  }, [client, mobile, onError]);

  async function save(next: MobileKeyboardFeedback) {
    if (!client) return;
    const current = generation.current;
    const previous = value;
    setValue(next);
    setBusy(true);
    onError("");
    try {
      const saved = await client.save(next);
      if (generation.current === current) setValue(saved);
    } catch {
      if (generation.current === current) {
        if (previous) setValue(previous);
        onError("无法保存按键反馈设置，请重试。");
      }
    } finally {
      if (generation.current === current) setBusy(false);
    }
  }

  async function preview() {
    if (!client?.preview || !value?.hapticsEnabled) return;
    onError("");
    try {
      await client.preview(value.hapticStrength);
    } catch {
      onError("无法预览按键振动，请重试。");
    }
  }

  return { value, busy, save, preview };
}
