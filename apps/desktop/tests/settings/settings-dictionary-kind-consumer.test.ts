import { expect, test } from "vitest";
import { localDictionaryKinds as dictionaryKinds } from "../../../../packages/ui/src/dictionary/dictionary-kinds";
import { localDictionaryKinds as settingsKinds } from "../../../../packages/ui/src/settings/settings-options";

test("settings options re-export the dictionary kind catalog", () => {
  expect(settingsKinds).toBe(dictionaryKinds);
});
