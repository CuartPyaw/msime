/**
 * A key drawn in a keyboard preview. `code` is the W3C `KeyboardEvent.code` (or soft-keyboard id) the statistics store counts the key under; it is absent on spacers and on keys the store has no id for.
 */
export type PreviewKey = { label: string; weight: number; code?: string };

const key = (label: string, weight = 1, code?: string): PreviewKey =>
  code === undefined ? { label, weight } : { label, weight, code };

const punctuationCodes: Record<string, string> = {
  "`": "Backquote",
  "-": "Minus",
  "=": "Equal",
  "[": "BracketLeft",
  "]": "BracketRight",
  "\\": "Backslash",
  ";": "Semicolon",
  "'": "Quote",
  ",": "Comma",
  ".": "Period",
  "/": "Slash",
};

/** The `KeyboardEvent.code` of the key that types `character` unshifted on an ANSI layout. */
function characterCode(character: string): string | undefined {
  if (/^[a-z]$/.test(character)) return `Key${character.toUpperCase()}`;
  if (/^[0-9]$/.test(character)) return `Digit${character}`;
  return punctuationCodes[character];
}

const letters = (text: string) => [...text].map((label) => key(label, 1, characterCode(label)));

export const desktopKeyboardRows: PreviewKey[][] = [
  [...letters("`1234567890-="), key("Backspace", 1.9, "Backspace")],
  [key("Tab", 1.5, "Tab"), ...letters("qwertyuiop[]"), key("\\", 1.4, "Backslash")],
  [key("Caps Lock", 1.85, "CapsLock"), ...letters("asdfghjkl;'"), key("Enter", 2, "Enter")],
  [key("Shift", 2.35, "ShiftLeft"), ...letters("zxcvbnm,./"), key("Shift", 2.15, "ShiftRight")],
  [
    key("Ctrl", 1.25, "ControlLeft"),
    key("Win", 1.25, "MetaLeft"),
    key("Alt", 1.25, "AltLeft"),
    key("Space", 6.7, "Space"),
    key("Alt", 1.25, "AltRight"),
    key("Win", 1.25, "MetaRight"),
    key("Del", 1.25, "Delete"),
    key("Ctrl", 1.25, "ControlRight"),
  ],
];

const functionKeys = (from: number, to: number) =>
  Array.from({ length: to - from + 1 }, (_, index) =>
    key(`F${from + index}`, 1, `F${from + index}`),
  );

/**
 * The physical ANSI keyboard the typing statistics heatmap draws: the escape and function row over the main block of `desktopKeyboardRows`. The bottom row differs from the preview's, which puts Del there for the on-screen keyboard; a real ANSI board has the context-menu key in that place, and Delete is drawn nowhere rather than somewhere it is not.
 */
export const ansiHeatmapKeyboardRows: PreviewKey[][] = [
  [
    key("Esc", 1, "Escape"),
    key("", 0.9),
    ...functionKeys(1, 4),
    key("", 0.5),
    ...functionKeys(5, 8),
    key("", 0.5),
    ...functionKeys(9, 12),
  ],
  ...desktopKeyboardRows.slice(0, 4),
  [
    key("Ctrl", 1.25, "ControlLeft"),
    key("Win", 1.25, "MetaLeft"),
    key("Alt", 1.25, "AltLeft"),
    key("Space", 6.7, "Space"),
    key("Alt", 1.25, "AltRight"),
    key("Win", 1.25, "MetaRight"),
    key("Menu", 1.25, "ContextMenu"),
    key("Ctrl", 1.25, "ControlRight"),
  ],
];

export const touchKeyboardRows: PreviewKey[][] = [
  letters("qwertyuiop"),
  [key("", 0.5), ...letters("asdfghjkl"), key("", 0.5)],
  [key("⇧", 1.5, "ShiftLeft"), ...letters("zxcvbnm"), key("⌫", 1.5, "Backspace")],
  [
    key("符号", 1.5, "SoftSymbol"),
    key("中/英", 1.5, "SoftLanguage"),
    key("空格", 4.5, "Space"),
    key("，", 1, "Comma"),
    key("↵", 1.5, "Enter"),
  ],
];

/** The nine-key pinyin grid, each cell named by the digit printed on it; `Nine1` is the punctuation and separator cell. */
export const nineKeyHeatmapRows: PreviewKey[][] = [
  [key("1 ，。", 1, "Nine1"), key("2 ABC", 1, "Nine2"), key("3 DEF", 1, "Nine3")],
  [key("4 GHI", 1, "Nine4"), key("5 JKL", 1, "Nine5"), key("6 MNO", 1, "Nine6")],
  [key("7 PQRS", 1, "Nine7"), key("8 TUV", 1, "Nine8"), key("9 WXYZ", 1, "Nine9")],
  [key("", 1), key("0", 1, "Nine0"), key("", 1)],
];

export const actionKeyboardLabels = new Set(["Backspace", "Enter", "Shift", "Del", "⇧", "⌫", "↵"]);
