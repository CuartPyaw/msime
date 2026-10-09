import { deepEqual } from "../core/deep-equal";
import type { Preferences, Snapshot } from "../index";

export interface SettingsDirtyOptions {
  draft?: Preferences;
  snapshot?: Snapshot;
  macosShuangpinKeymap?: boolean;
  savedMacosShuangpinKeymap?: boolean;
}

/** Reports whether shared settings or a macOS native preference changed. */
export function settingsDirty({
  draft,
  snapshot,
  macosShuangpinKeymap,
  savedMacosShuangpinKeymap,
}: SettingsDirtyOptions): boolean {
  return (
    (!!draft && !!snapshot && !deepEqual(draft, snapshot.preferences)) ||
    (macosShuangpinKeymap !== undefined && macosShuangpinKeymap !== savedMacosShuangpinKeymap)
  );
}
