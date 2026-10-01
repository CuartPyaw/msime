import type { HostPlatform } from "../index";
import {
  defaultCandidateFallbackFonts,
  defaultCandidateFontFamily,
  type CandidateFontPreferences,
} from "./candidate-font-family";

export type CandidateFontPresetId = "default" | "song" | "hei" | "kai" | "yuan";

type PresetFamily = {
  family: string;
  /** The hosts whose stock fonts include this family; absent for a family no host ships but any can install. */
  platforms?: readonly HostPlatform[];
};

type CandidateFontPreset = {
  id: CandidateFontPresetId;
  label: string;
  families: readonly PresetFamily[];
  /** Hosts with no face of this style at all, where the preset is offered disabled. */
  unavailable?: readonly HostPlatform[];
};

const apple = ["macos", "ios"] as const;

/** The 候选字体 presets: each is a style of Chinese face, named by the family each platform ships for it. No preference records the preset; it is read back from `candidate_font_family`. */
export const candidateFontPresets: readonly CandidateFontPreset[] = [
  {
    id: "default",
    label: "默认",
    // Only the default main font names 默认: its fallback list also carries Microsoft YaHei, which is 黑体's Windows face, so matching the whole list would read 黑体 as 默认 on any other host. The patch still restores the full default list.
    families: [{ family: defaultCandidateFontFamily }],
  },
  {
    id: "song",
    label: "宋体",
    families: [
      { family: "Songti SC", platforms: apple },
      { family: "SimSun", platforms: ["windows"] },
      { family: "Noto Serif CJK SC", platforms: ["linux"] },
      { family: "Noto Serif SC" },
    ],
  },
  {
    id: "hei",
    label: "黑体",
    families: [
      { family: "PingFang SC", platforms: apple },
      { family: "Microsoft YaHei", platforms: ["windows"] },
      { family: "Noto Sans CJK SC", platforms: ["linux"] },
      { family: "Noto Sans SC" },
    ],
  },
  {
    id: "kai",
    label: "楷体",
    families: [
      { family: "Kaiti SC", platforms: apple },
      { family: "KaiTi", platforms: ["windows"] },
      { family: "STKaiti", platforms: apple },
      { family: "AR PL UKai CN", platforms: ["linux"] },
    ],
  },
  {
    id: "yuan",
    label: "圆体",
    families: [{ family: "Yuanti SC", platforms: apple }, { family: "Noto Sans SC" }],
    // Windows ships no rounded Chinese face, so the preset is disabled there rather than quietly drawing 黑体.
    unavailable: ["windows"],
  },
];

const maxFallbackFonts = 32;

function preset(id: CandidateFontPresetId): CandidateFontPreset {
  return candidateFontPresets.find((entry) => entry.id === id) ?? candidateFontPresets[0];
}

/** The preset's families with the ones `platform` ships first, the rest in the preset's order; with no platform known, every family in the preset's order. */
function orderedFamilies(entry: CandidateFontPreset, platform: HostPlatform | undefined): string[] {
  const native = entry.families.filter(
    (family) => platform !== undefined && family.platforms?.includes(platform),
  );
  const rest = entry.families.filter((family) => !native.includes(family));
  return [...native, ...rest].map((family) => family.family);
}

/** Whether the preset can be chosen on `platform`; unknown platforms can choose every preset. */
export function candidateFontPresetAvailable(
  id: CandidateFontPresetId,
  platform: HostPlatform | undefined,
): boolean {
  return platform === undefined || !preset(id).unavailable?.includes(platform);
}

/** The font preferences for choosing `id` on `platform`: the first family the platform has becomes the main font, and the preset's families lead the fallback chain (which is all the Windows renderer reads), followed by the user's own list without duplicates. 默认 restores the core's defaults. */
export function candidateFontPresetPatch(
  id: CandidateFontPresetId,
  platform: HostPlatform | undefined,
  current: CandidateFontPreferences,
): CandidateFontPreferences {
  if (id === "default")
    return {
      candidate_font_family: defaultCandidateFontFamily,
      candidate_fallback_fonts: [...defaultCandidateFallbackFonts],
    };
  const families = orderedFamilies(preset(id), platform);
  const rest = (current.candidate_fallback_fonts ?? defaultCandidateFallbackFonts).filter(
    (family) => !families.includes(family),
  );
  return {
    candidate_font_family: families[0],
    candidate_fallback_fonts: [...families, ...rest].slice(0, maxFallbackFonts),
  };
}

/** The preset the font preferences show, or `null` for a font the user chose themselves. The main font names the preset; where two presets share it (黑体 and 圆体 both fall back to Noto Sans SC, as 默认 does), the one whose families lead the fallback chain on `platform` wins, and otherwise 默认. */
export function currentCandidateFontPreset(
  preferences: CandidateFontPreferences,
  platform: HostPlatform | undefined,
): CandidateFontPresetId | null {
  const family = preferences.candidate_font_family ?? defaultCandidateFontFamily;
  const fallback = preferences.candidate_fallback_fonts ?? defaultCandidateFallbackFonts;
  const matches = candidateFontPresets.filter((entry) =>
    entry.families.some((candidate) => candidate.family === family),
  );
  if (matches.length <= 1) return matches[0]?.id ?? null;
  const leading = matches.find((entry) => {
    const families =
      entry.id === "default"
        ? [...defaultCandidateFallbackFonts]
        : orderedFamilies(entry, platform);
    return families[0] === family && families.every((name, index) => fallback[index] === name);
  });
  if (leading) return leading.id;
  // 默认 is only ever matched by its own main font, so on a tie it is the one the font names.
  return family === defaultCandidateFontFamily ? "default" : matches[0].id;
}
