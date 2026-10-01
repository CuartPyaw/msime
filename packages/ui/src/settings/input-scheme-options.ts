import type { ChineseScheme, HostCapabilities, InputScheme, VietnamesePreferences } from "../index";

export type ChineseInputScheme = ChineseScheme;

export const chineseInputSchemeOptions = [
  { value: "quanpin", label: "全拼" },
  { value: "shuangpin", label: "双拼" },
  { value: "wubi", label: "五笔" },
  { value: "cantonese", label: "粤拼" },
  { value: "zhuyin", label: "注音" },
] as const satisfies readonly { value: ChineseInputScheme; label: string }[];

export const japaneseInputSchemeOptions = [{ value: "romaji", label: "罗马音" }] as const;

export const koreanInputSchemeOptions = [{ value: "dubeolsik", label: "两套式" }] as const;

export const cantoneseInputSchemeOptions = [{ value: "jyutping", label: "粤拼" }] as const;

export const zhuyinLayoutOptions = [{ value: "dachen", label: "大千" }] as const;

export const vietnameseInputMethodOptions = [
  { value: "telex", label: "Telex" },
  { value: "vni", label: "VNI" },
] as const satisfies readonly {
  value: NonNullable<VietnamesePreferences["input_method"]>;
  label: string;
}[];

export const vietnameseToneStyleOptions = [
  { value: "modern", label: "新式 hoà" },
  { value: "classic", label: "旧式 hòa" },
] as const satisfies readonly {
  value: NonNullable<VietnamesePreferences["tone_style"]>;
  label: string;
}[];

/** The input modes that are not Chinese: selecting one remembers the Chinese scheme in `last_chinese_scheme`, and 中文 returns to it. */
export const nonChineseSchemes = ["japanese", "korean", "vietnamese"] as const;

export function isChineseScheme(scheme: InputScheme): scheme is ChineseScheme {
  return !(nonChineseSchemes as readonly InputScheme[]).includes(scheme);
}

/** Mirrors `client-core::host_surface::base_input_schemes`: what a host older than `HostCapabilities.input_schemes` offers. */
export const baseInputSchemes: readonly InputScheme[] = [
  "quanpin",
  "shuangpin",
  "wubi",
  "japanese",
  "korean",
];

const knownInputSchemes: readonly InputScheme[] = [
  ...baseInputSchemes,
  "cantonese",
  "zhuyin",
  "vietnamese",
];

/** The schemes the host offers. A value this build does not know is dropped, so a newer host never puts an unlabelled option on the page. */
export function supportedInputSchemes(host?: HostCapabilities): readonly InputScheme[] {
  return (
    host?.input_schemes?.filter((scheme) => knownInputSchemes.includes(scheme)) ?? baseInputSchemes
  );
}

/** The scheme host-api runs when the document names one the host does not offer: the remembered Chinese scheme when it is offered, else 全拼. */
export function fallbackChineseScheme(
  lastChineseScheme: ChineseScheme | null | undefined,
  supported: readonly InputScheme[],
): ChineseScheme {
  return lastChineseScheme && supported.includes(lastChineseScheme) ? lastChineseScheme : "quanpin";
}
