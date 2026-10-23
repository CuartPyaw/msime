import type { Preferences } from "../core/preferences-model";
import { Row } from "../core/platform-controls";
import { isComposingKeyEvent } from "../core/ime-key-event";
import { ActionButton } from "./action-button";
import { SettingsRowStack } from "./settings-row-stack";
import { SettingsGroupNote } from "./settings-group-note";
import { SettingsWarning } from "./settings-warning";
import { useVimModeApps, type UseVimModeAppsOptions } from "./use-vim-mode-apps";
import { SwitchRow } from "./switch-row";

export interface ChineseEnglishSpacingSectionProps {
  value?: boolean;
  onChange: (value: boolean) => void;
  crossCommit?: Preferences["cross_commit_spacing"];
  /** 只有接通跨次空格的 macOS 宿主提供这个回调。 */
  onCrossCommitChange?: (value: NonNullable<Preferences["cross_commit_spacing"]>) => void;
  pickApplication?: UseVimModeAppsOptions["pickApplication"];
  applicationNames?: UseVimModeAppsOptions["applicationNames"];
}

/** 「中英文之间加空格」开关：输入页「输出」组里的一行，缺省为关，与共享偏好的默认值一致。 */
export function ChineseEnglishSpacingSection({
  value,
  onChange,
  crossCommit,
  onCrossCommitChange,
  pickApplication,
  applicationNames,
}: ChineseEnglishSpacingSectionProps) {
  const config = crossCommit ?? { enabled: false, excluded_apps: [] };
  const apps = useVimModeApps({
    value: config.excluded_apps,
    windows: false,
    pickApplication,
    applicationNames,
    listName: "跨次空格排除名单",
    onChange: (excluded_apps) => onCrossCommitChange?.({ ...config, excluded_apps }),
  });
  return (
    <>
      <SwitchRow
        title="中英文之间加空格"
        description="一次上屏的文字里，汉字与英文字母、数字相邻处自动加一个空格，如「用Rust写」上屏为「用 Rust 写」。两次上屏之间不加；网址、日期等快捷模式和全角输入不受影响，译文和释义上屏、常用语、剪贴板和语音结果也不加"
        checked={value ?? false}
        onChange={onChange}
      />
      {onCrossCommitChange && (
        <>
          <SwitchRow
            title="跨次上屏中英文空格"
            description="新输入与前文的汉字和英文字母、数字相邻时补一个空格，如分次输入「用」「Rust」「写」得到「用 Rust 写」。仅 macOS；不改右侧已有文字，读不到文档时跳过。补空格可能增加一次撤销步骤。网址等快捷模式和全角输入不处理；自由输入的网址、邮箱和代码请关闭此项或排除应用"
            checked={config.enabled}
            onChange={(enabled) => onCrossCommitChange({ ...config, enabled })}
          />
          {config.enabled && (
            <SettingsRowStack role="group" aria-label="跨次空格排除应用">
              <SettingsGroupNote>
                以下应用不补跨次空格，可添加终端和代码编辑器。最多 32 个应用。
              </SettingsGroupNote>
              {apps.entries.map((id) => (
                <Row
                  key={id}
                  title={apps.names[id] ?? id}
                  description={apps.names[id] ? id : undefined}
                >
                  <ActionButton
                    action={() => apps.remove(id)}
                    ariaLabel={`把 ${apps.names[id] ?? id} 从跨次空格排除名单移除`}
                    label="移除"
                  />
                </Row>
              ))}
              <Row title="排除应用">
                {apps.pick ? (
                  <ActionButton
                    action={apps.pick}
                    disabled={apps.full || apps.picking}
                    label="添加应用…"
                  />
                ) : (
                  <>
                    <input
                      aria-label="应用 Bundle ID"
                      value={apps.draft}
                      autoCapitalize="off"
                      spellCheck={false}
                      placeholder="例如 com.apple.Terminal"
                      onChange={(event) => apps.edit(event.target.value)}
                      onKeyDown={(event) => {
                        if (event.key === "Enter" && !isComposingKeyEvent(event)) apps.addTyped();
                      }}
                    />
                    <ActionButton action={apps.addTyped} disabled={apps.full} label="添加" />
                  </>
                )}
              </Row>
              {apps.warning && <SettingsWarning role="alert">{apps.warning}</SettingsWarning>}
            </SettingsRowStack>
          )}
        </>
      )}
    </>
  );
}
