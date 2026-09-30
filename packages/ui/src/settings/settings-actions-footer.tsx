import * as settings from "./settings-style";

export interface SettingsActionsFooterProps {
  busy: boolean;
  dirty: boolean;
  canSave: boolean;
  showRestoreDefaults: boolean;
  onRestoreDefaults: () => void;
  /** Discards the draft and reads the saved settings again; drawn with the other secondary actions when the page allows it. */
  onReload?: () => void;
}

/** Shared save and restore controls at the bottom of settings forms: the secondary actions on the left, the status and 保存设置 on the right, in one row. */
export function SettingsActionsFooter({
  busy,
  dirty,
  canSave,
  showRestoreDefaults,
  onRestoreDefaults,
  onReload,
}: SettingsActionsFooterProps) {
  return (
    <footer className={settings.settingsActions}>
      {onReload && (
        <button type="button" className="secondary" disabled={busy} onClick={onReload}>
          重新读取
        </button>
      )}
      {showRestoreDefaults && (
        <button type="button" className="secondary" disabled={busy} onClick={onRestoreDefaults}>
          恢复默认设置
        </button>
      )}
      <span>{dirty ? "有未保存的修改" : ""}</span>
      <button type="submit" disabled={busy || !dirty || !canSave}>
        {busy ? "处理中…" : "保存设置"}
      </button>
    </footer>
  );
}
