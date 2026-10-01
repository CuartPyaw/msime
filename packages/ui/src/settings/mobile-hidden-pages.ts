export type MobileHiddenPageId = "shortcuts" | "floating-toolbar" | "helpcode" | "plugins";

export function mobileHiddenPageIds(options: {
  modeSwitchShortcuts: boolean;
  panelShortcuts: boolean;
  desktopMaintenanceShortcuts: boolean;
  helpcodeShiftEntry: boolean;
  android: boolean;
  harmony: boolean;
}): readonly MobileHiddenPageId[] {
  return [
    ...(options.modeSwitchShortcuts || options.panelShortcuts || options.desktopMaintenanceShortcuts
      ? []
      : (["shortcuts"] as const)),
    "floating-toolbar",
    // Sound packs, music and the / and @ modes belong to a physical keyboard; a phone keeps its own keyboard feedback settings.
    "plugins",
    ...(options.helpcodeShiftEntry || options.android || options.harmony
      ? []
      : (["helpcode"] as const)),
  ];
}
