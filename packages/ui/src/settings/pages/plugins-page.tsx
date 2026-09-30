import * as settings from "../settings-style";
import { useSettingsForm } from "../settings-form-context";
import { PluginsSection } from "../plugins-section";
import { pluginPreferences } from "../plugin-preferences";
import { createSettingsDraftActions } from "../settings-draft-actions";

/** The 扩展 page of the settings form (route id `plugins`): sound packs, background music, command tables and the @ name list. */
export function PluginsSettingsPage() {
  const {
    client,
    draft,
    setDraft,
    busy,
    page,
    setError,
    confirm,
    showKeySound,
    showMusic,
    showPluginTriggers,
  } = useSettingsForm();
  const { onPreferencesChange } = createSettingsDraftActions({ setDraft });
  return (
    <fieldset disabled={busy} hidden={page !== "plugins"} aria-label="扩展">
      <div className={settings.groups}>
        <PluginsSection
          client={client.plugins}
          preferences={pluginPreferences(draft)}
          keySound={showKeySound}
          music={showMusic}
          triggers={showPluginTriggers}
          active={page === "plugins"}
          onChange={(plugins) => onPreferencesChange({ plugins })}
          onError={setError}
          confirm={confirm}
        />
      </div>
    </fieldset>
  );
}
