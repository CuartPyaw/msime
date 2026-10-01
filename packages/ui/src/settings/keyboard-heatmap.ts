import {
  ansiHeatmapKeyboardRows,
  nineKeyHeatmapRows,
  touchKeyboardRows,
  type PreviewKey,
} from "../keyboard/keyboard-layouts";

/** Per-key press counts per local day, as the statistics store keeps them: `{ "YYYY-MM-DD": { keyId: count } }`. */
export type DailyKeyCounts = Record<string, Record<string, number>>;

export type KeyboardHeatmapLayout = "ansi" | "touch";

/** A drawn key with its count: `label` is printed on the cap, `name` is what it is announced as. A spacer has no `code` and a count of zero. */
export type KeyboardHeatmapKey = {
  code?: string;
  label: string;
  name: string;
  weight: number;
  count: number;
};

export type KeyCount = { code: string; label: string; count: number };

export type KeyboardHeatmapModel = {
  layout: KeyboardHeatmapLayout;
  rows: KeyboardHeatmapKey[][];
  /** The nine-key grid, present only on the touch layout and only when a nine-key cell has a count. */
  nineRows: KeyboardHeatmapKey[][] | null;
  /** Counted keys the drawn layout has no place for, most pressed first. */
  others: KeyCount[];
  /** The five most pressed keys, drawn or not. */
  top: KeyCount[];
  /** The largest single-key count, which the shade levels are relative to; at least 1. */
  maximum: number;
  total: number;
};

/**
 * Sums the per-key counts of the days in scope; `null` is the whole retained history, matching how `scopedBreakdown` reads the page's scope.
 *
 * Only positive finite counts are kept, so a key the result holds was pressed at least once in scope.
 */
export function scopedKeyCounts(
  dailyKeys: DailyKeyCounts | undefined,
  scopeKeys: readonly string[] | null,
): Record<string, number> {
  const result: Record<string, number> = {};
  if (!dailyKeys) return result;
  const days = scopeKeys ?? Object.keys(dailyKeys);
  for (const day of days) {
    const keys = dailyKeys[day];
    if (!keys) continue;
    for (const [code, count] of Object.entries(keys)) {
      if (!Number.isFinite(count) || count <= 0) continue;
      result[code] = (result[code] ?? 0) + count;
    }
  }
  return result;
}

/** Whether `code` exists only on an on-screen keyboard. */
function softKey(code: string): boolean {
  return code.startsWith("Nine") || code.startsWith("Soft");
}

/**
 * Which keyboard to draw: the phone's 26-key layout on a mobile page or whenever a soft-keyboard-only key was counted, otherwise the physical ANSI board.
 */
export function keyboardHeatmapLayout(
  counts: Record<string, number>,
  mobile: boolean,
): KeyboardHeatmapLayout {
  return mobile || Object.keys(counts).some(softKey) ? "touch" : "ansi";
}

const mac = (platform: string | undefined) => platform === "macos";

function metaName(platform: string | undefined): string {
  if (mac(platform)) return "Command";
  if (platform === "linux") return "Super";
  return "Win";
}

const fixedLabels: Record<string, string> = {
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
  IntlBackslash: "ISO \\",
  IntlRo: "ろ",
  IntlYen: "¥",
  Lang1: "Lang1",
  Lang2: "Lang2",
  Convert: "変換",
  NonConvert: "無変換",
  KanaMode: "かな",
  Space: "空格",
  Enter: "回车",
  Backspace: "退格",
  Tab: "Tab",
  Escape: "Esc",
  Delete: "Delete",
  Insert: "Insert",
  Home: "Home",
  End: "End",
  PageUp: "Page Up",
  PageDown: "Page Down",
  ArrowUp: "上箭头",
  ArrowDown: "下箭头",
  ArrowLeft: "左箭头",
  ArrowRight: "右箭头",
  CapsLock: "Caps Lock",
  ShiftLeft: "左 Shift",
  ShiftRight: "右 Shift",
  Fn: "Fn",
  ContextMenu: "菜单键",
  NumpadDecimal: "小键盘 .",
  NumpadEnter: "小键盘回车",
  NumpadAdd: "小键盘 +",
  NumpadSubtract: "小键盘 -",
  NumpadMultiply: "小键盘 *",
  NumpadDivide: "小键盘 /",
  NumLock: "Num Lock",
  Nine1: "九宫格 1（标点）",
  SoftPunctuation: "九宫格侧栏标点",
  SoftSymbol: "符号",
  SoftLayer: "123",
  SoftLanguage: "中/英",
  SoftGlobe: "地球键",
  SoftEmoji: "表情",
  SoftVoice: "语音",
};

/** The name a key is announced and listed under, e.g. `A`, `空格`, `左 Shift`; an id this page does not know is shown as itself. */
export function keyLabel(code: string, platform?: string): string {
  const fixed = fixedLabels[code];
  if (fixed) return fixed;
  let match =
    /^Key([A-Z])$/.exec(code) ?? /^Digit([0-9])$/.exec(code) ?? /^(F[0-9]{1,2})$/.exec(code);
  if (match) return match[1];
  match = /^Numpad([0-9])$/.exec(code);
  if (match) return `小键盘 ${match[1]}`;
  match = /^Nine([0-9])$/.exec(code);
  if (match) return `九宫格 ${match[1]}`;
  match = /^(Control|Alt|Meta)(Left|Right)$/.exec(code);
  if (match) {
    const side = match[2] === "Left" ? "左" : "右";
    const name =
      match[1] === "Meta"
        ? metaName(platform)
        : match[1] === "Alt"
          ? mac(platform)
            ? "Option"
            : "Alt"
          : mac(platform)
            ? "Control"
            : "Ctrl";
    return `${side} ${name}`;
  }
  return code;
}

/** The text printed on a drawn key cap: the layout's own label, upper-case letters, and the platform's names for the modifier keys. */
function capLabel(key: PreviewKey, platform: string | undefined): string {
  const code = key.code;
  if (!code) return key.label;
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (code.startsWith("Meta")) return mac(platform) ? "⌘" : metaName(platform);
  if (code.startsWith("Alt")) return mac(platform) ? "⌥" : "Alt";
  if (code.startsWith("Control")) return mac(platform) ? "⌃" : "Ctrl";
  return key.label;
}

/** Most pressed first; equal counts fall back to the id so the order does not depend on how the host serialized the map. */
function byCount(left: KeyCount, right: KeyCount): number {
  return right.count - left.count || (left.code < right.code ? -1 : left.code > right.code ? 1 : 0);
}

/**
 * Places scoped counts on the chosen keyboard: the drawn rows with each key's count, the nine-key grid when one of its cells was pressed, the keys the layout cannot draw, and the top five.
 */
export function keyboardHeatmapModel(
  counts: Record<string, number>,
  mobile: boolean,
  platform?: string,
): KeyboardHeatmapModel {
  const layout = keyboardHeatmapLayout(counts, mobile);
  const drawn = new Set<string>();
  const place = (rows: PreviewKey[][]) =>
    rows.map((row) =>
      row.map((key) => {
        if (key.code) drawn.add(key.code);
        return {
          ...(key.code ? { code: key.code } : {}),
          label: capLabel(key, platform),
          name: key.code ? keyLabel(key.code, platform) : "",
          weight: key.weight,
          count: key.code ? (counts[key.code] ?? 0) : 0,
        };
      }),
    );
  const rows = place(layout === "touch" ? touchKeyboardRows : ansiHeatmapKeyboardRows);
  const nineRows =
    layout === "touch" && Object.keys(counts).some((code) => code.startsWith("Nine"))
      ? place(nineKeyHeatmapRows)
      : null;
  const listed = Object.entries(counts)
    .filter(([, count]) => count > 0)
    .map(([code, count]) => ({ code, label: keyLabel(code, platform), count }))
    .sort(byCount);
  const total = listed.reduce((sum, key) => sum + key.count, 0);
  return {
    layout,
    rows,
    nineRows,
    others: listed.filter((key) => !drawn.has(key.code)),
    top: listed.slice(0, 5),
    maximum: Math.max(1, ...listed.map((key) => key.count)),
    total,
  };
}

/** The shade a count takes, 0 (never pressed) to 4, on the same scale as the calendar heatmap. */
export function keyHeatLevel(count: number, maximum: number): number {
  return count <= 0 ? 0 : Math.min(4, Math.max(1, Math.ceil((count / maximum) * 4)));
}
