import type { Preferences } from "../index";

/** Mirrors `client-core::preferences::KeySoundMode`. */
export type KeySoundMode = "keys" | "melody";

/** Mirrors `client-core::preferences::KeySoundPreferences`. */
export type KeySoundPreferences = {
  enabled: boolean;
  mode: KeySoundMode;
  /** The sound pack keys, commits and achievements are played from. */
  pack: string;
  /** 0-100, for every effect sound: keys, the melody, commits and achievements. */
  volume: number;
};

/** Mirrors `client-core::preferences::CommitSoundPreferences`. */
export type CommitSoundPreferences = { enabled: boolean };

/** Mirrors `client-core::preferences::MelodyPreferences`. */
export type MelodyPreferences = { pack: string };

/** Mirrors `client-core::preferences::MusicPreferences`. */
export type MusicPreferences = { enabled: boolean; pack: string; volume: number };

/** Mirrors `client-core::preferences::AchievementPreferences`. */
export type AchievementPreferences = { enabled: boolean };

/** Mirrors `client-core::preferences::PluginPreferences`. The document leaves the whole section out while it is at these defaults, so a page reading one merges it over `defaultPluginPreferences`. */
export type PluginPreferences = {
  key_sound: KeySoundPreferences;
  commit_sound: CommitSoundPreferences;
  melody: MelodyPreferences;
  music: MusicPreferences;
  achievements: AchievementPreferences;
  /** Installed command-table packs the `/` mode reads, in priority order: the first pack that defines a trigger wins. */
  command_tables: string[];
};

/** `client-core::plugins::DEFAULT_SOUND_PACK`. */
export const DEFAULT_SOUND_PACK = "default";
/** `client-core::plugins::DEFAULT_MELODY_PACK`. */
export const DEFAULT_MELODY_PACK = "twinkle";
/** `PluginPreferences::MAX_COMMAND_TABLES`. */
export const MAX_COMMAND_TABLES = 16;

export const defaultPluginPreferences: PluginPreferences = {
  key_sound: { enabled: false, mode: "keys", pack: DEFAULT_SOUND_PACK, volume: 50 },
  commit_sound: { enabled: false },
  melody: { pack: DEFAULT_MELODY_PACK },
  music: { enabled: false, pack: "", volume: 30 },
  achievements: { enabled: false },
  command_tables: [],
};

/** The plugin section a draft holds, each part filled in from the defaults where the document left it out. */
export function pluginPreferences(draft?: Pick<Preferences, "plugins">): PluginPreferences {
  const value = draft?.plugins;
  return {
    key_sound: { ...defaultPluginPreferences.key_sound, ...value?.key_sound },
    commit_sound: { ...defaultPluginPreferences.commit_sound, ...value?.commit_sound },
    melody: { ...defaultPluginPreferences.melody, ...value?.melody },
    music: { ...defaultPluginPreferences.music, ...value?.music },
    achievements: { ...defaultPluginPreferences.achievements, ...value?.achievements },
    command_tables: value?.command_tables ?? defaultPluginPreferences.command_tables,
  };
}

/**
 * The section after a pack was removed from disk: a selection naming it falls back to what a fresh profile selects, and an enabled command table naming it is dropped, so the document never points at a pack that is gone.
 */
export function withoutRemovedPack(
  preferences: PluginPreferences,
  kind: "sound" | "music" | "command_table",
  id: string,
): PluginPreferences {
  if (kind === "command_table") {
    return {
      ...preferences,
      command_tables: preferences.command_tables.filter((table) => table !== id),
    };
  }
  if (kind === "music") {
    return preferences.music.pack === id
      ? { ...preferences, music: { ...preferences.music, enabled: false, pack: "" } }
      : preferences;
  }
  return {
    ...preferences,
    key_sound:
      preferences.key_sound.pack === id
        ? { ...preferences.key_sound, pack: DEFAULT_SOUND_PACK }
        : preferences.key_sound,
    melody: preferences.melody.pack === id ? { pack: DEFAULT_MELODY_PACK } : preferences.melody,
  };
}
