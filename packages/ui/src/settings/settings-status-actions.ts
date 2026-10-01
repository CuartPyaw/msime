export interface CreateSettingsStatusActionsOptions {
  recoverPreferences: () => Promise<void>;
  inputSourceStartup?: { openSettings: () => Promise<void>; enable?: () => Promise<void> };
  /** Hides the startup notice for the rest of this window, including later focus refreshes. */
  dismissInputSourceStartup: () => void;
}

/** Creates recovery and startup-notice actions for the settings status surface. */
export function createSettingsStatusActions({
  recoverPreferences,
  inputSourceStartup,
  dismissInputSourceStartup,
}: CreateSettingsStatusActionsOptions) {
  const enable = inputSourceStartup?.enable;
  return {
    onRecover: () => void recoverPreferences(),
    onOpenSettings: () => inputSourceStartup?.openSettings(),
    onEnable: enable ? () => enable() : undefined,
    onDismiss: dismissInputSourceStartup,
  } as const;
}
