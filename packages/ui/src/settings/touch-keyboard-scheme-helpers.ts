import type { Preferences } from "../index";
import { isChineseScheme } from "./input-scheme-options";

export type TouchKeyboardScheme =
  | "quanpin"
  | "nine_key"
  | "xiaohe"
  | "ziranma"
  | "microsoft"
  | "shoudao"
  | "wubi"
  | "japanese_nine_key"
  | "japanese"
  | "handwriting"
  | "thoughtful_reply"
  | "korean";
export type TouchKeyboardSchemePreferences = {
  enabled: TouchKeyboardScheme[];
  selected?: TouchKeyboardScheme;
};

export function touchKeyboardSchemeTitle(preferences: Preferences): string {
  const selected = preferences.touch_keyboard_schemes?.selected;
  if (selected) {
    return {
      quanpin: "全拼 26 键",
      nine_key: "全拼 9 键",
      xiaohe: "小鹤双拼",
      ziranma: "自然码双拼",
      microsoft: "微软双拼",
      shoudao: "首道双拼",
      wubi: "86 五笔",
      japanese_nine_key: "日语 9 键",
      japanese: "日语 26 键",
      handwriting: "手写",
      thoughtful_reply: "高情商回复",
      korean: "韩语 26 键",
    }[selected];
  }
  // Korean has only the 26-key Dubeolsik keyboard, whatever layout the document carries.
  if (preferences.scheme === "korean") return "韩语 26 键";
  if (preferences.touch_keyboard_layout === "handwriting") return "手写";
  if (preferences.touch_keyboard_layout === "nine_key")
    return preferences.scheme === "japanese" ? "日语 9 键" : "全拼 9 键";
  if (preferences.scheme === "japanese") return "日语 26 键";
  if (preferences.scheme === "wubi") return "86 五笔";
  if (preferences.scheme === "shuangpin") return `${preferences.shuangpin_profile} 双拼`;
  return "全拼 26 键";
}

export const touchKeyboardSchemeOptions: [TouchKeyboardScheme, string][] = [
  ["quanpin", "全拼 26 键"],
  ["nine_key", "全拼 9 键"],
  ["xiaohe", "小鹤双拼"],
  ["ziranma", "自然码双拼"],
  ["microsoft", "微软双拼"],
  ["shoudao", "首道双拼"],
  ["wubi", "86 五笔"],
  ["japanese_nine_key", "日语 9 键"],
  ["japanese", "日语 26 键"],
  ["handwriting", "手写"],
  ["thoughtful_reply", "高情商回复"],
  ["korean", "韩语 26 键"],
];
export const allTouchKeyboardSchemes = touchKeyboardSchemeOptions.map(([scheme]) => scheme);

export function inferredTouchKeyboardScheme(preferences: Preferences): TouchKeyboardScheme {
  const enabled = preferences.touch_keyboard_schemes?.enabled ?? allTouchKeyboardSchemes;
  const selected = preferences.touch_keyboard_schemes?.selected;
  if (selected && enabled.includes(selected)) return selected;
  let inferred = touchSchemeOf(preferences, preferences.scheme);
  if (preferences.touch_keyboard_layout === "handwriting" && preferences.scheme === "quanpin")
    inferred = "handwriting";
  else if (preferences.touch_keyboard_layout === "nine_key" && preferences.scheme !== "korean")
    inferred = preferences.scheme === "japanese" ? "japanese_nine_key" : "nine_key";
  return enabled.includes(inferred) ? inferred : (enabled[0] ?? "quanpin");
}

/** The touch scheme for a document scheme. Cantonese, Zhuyin and Vietnamese have no touch keyboard, so they map to the remembered Chinese scheme's touch scheme, or 全拼 when that has none either. */
function touchSchemeOf(
  preferences: Preferences,
  scheme: Preferences["scheme"],
): TouchKeyboardScheme {
  switch (scheme) {
    case "shuangpin":
      return preferences.shuangpin_profile;
    case "quanpin":
    case "wubi":
    case "japanese":
    case "korean":
      return scheme;
    case "cantonese":
    case "zhuyin":
    case "vietnamese": {
      const remembered = preferences.last_chinese_scheme;
      return remembered === "quanpin" || remembered === "shuangpin" || remembered === "wubi"
        ? touchSchemeOf(preferences, remembered)
        : "quanpin";
    }
  }
}

/** The Chinese scheme a Japanese, Korean or Vietnamese selection returns to; switching among those keeps the one already remembered. */
function rememberedChineseScheme(preferences: Preferences): Preferences["last_chinese_scheme"] {
  return isChineseScheme(preferences.scheme) ? preferences.scheme : preferences.last_chinese_scheme;
}

export function selectTouchKeyboardScheme(
  preferences: Preferences,
  selected: TouchKeyboardScheme,
): Preferences {
  const touch_keyboard_schemes = {
    enabled: preferences.touch_keyboard_schemes?.enabled ?? allTouchKeyboardSchemes,
    selected,
  };
  if (["xiaohe", "ziranma", "microsoft", "shoudao"].includes(selected))
    return {
      ...preferences,
      scheme: "shuangpin",
      last_chinese_scheme: "shuangpin",
      shuangpin_profile: selected as Preferences["shuangpin_profile"],
      touch_keyboard_layout: "twenty_six_key",
      touch_keyboard_schemes,
    };
  if (selected === "japanese" || selected === "japanese_nine_key")
    return {
      ...preferences,
      scheme: "japanese",
      last_chinese_scheme: rememberedChineseScheme(preferences),
      touch_keyboard_layout: selected === "japanese_nine_key" ? "nine_key" : "twenty_six_key",
      touch_keyboard_schemes,
    };
  if (selected === "korean")
    return {
      ...preferences,
      scheme: "korean",
      last_chinese_scheme: rememberedChineseScheme(preferences),
      touch_keyboard_layout: "twenty_six_key",
      touch_keyboard_schemes,
    };
  if (selected === "wubi")
    return {
      ...preferences,
      scheme: "wubi",
      last_chinese_scheme: "wubi",
      touch_keyboard_layout: "twenty_six_key",
      touch_keyboard_schemes,
    };
  return {
    ...preferences,
    scheme: "quanpin",
    last_chinese_scheme: "quanpin",
    touch_keyboard_layout:
      selected === "nine_key"
        ? "nine_key"
        : selected === "handwriting"
          ? "handwriting"
          : "twenty_six_key",
    touch_keyboard_schemes,
  };
}

/** Toggle a touch scheme while keeping one visible and a valid selected scheme. */
export function updateTouchKeyboardSchemeEnabled(
  preferences: Preferences,
  scheme: TouchKeyboardScheme,
  enabled: boolean,
  selectedTouchKeyboardScheme = inferredTouchKeyboardScheme(preferences),
): Preferences | null {
  const visible = new Set(preferences.touch_keyboard_schemes?.enabled ?? allTouchKeyboardSchemes);
  if (enabled) visible.add(scheme);
  else visible.delete(scheme);
  if (visible.size === 0) return null;
  const ordered = allTouchKeyboardSchemes.filter((value) => visible.has(value));
  const selected = visible.has(selectedTouchKeyboardScheme)
    ? selectedTouchKeyboardScheme
    : ordered[0];
  const next = selectTouchKeyboardScheme(preferences, selected);
  return { ...next, touch_keyboard_schemes: { enabled: ordered, selected } };
}

/** Select a scheme from the touch keyboard home page, enabling it if necessary. */
export function selectHomeTouchKeyboardScheme(
  preferences: Preferences,
  scheme: TouchKeyboardScheme,
): Preferences {
  const visible = new Set(preferences.touch_keyboard_schemes?.enabled ?? allTouchKeyboardSchemes);
  visible.add(scheme);
  const enabled = allTouchKeyboardSchemes.filter((value) => visible.has(value));
  const next = selectTouchKeyboardScheme(preferences, scheme);
  return { ...next, touch_keyboard_schemes: { enabled, selected: scheme } };
}
