import { describe, expect, test } from "vitest";
import {
  oobeComplete,
  oobeFlowComplete,
  oobeNextAction,
  oobeSystemReady,
  type OobeState,
} from "../../../../packages/ui/src/core/host-contracts";

const ready = (): OobeState => ({
  resourcesReady: true,
  systemEnabled: true,
  systemCurrent: true,
  preferencesSaved: true,
  onboardingSeen: true,
});

describe("跨平台 OOBE 状态契约", () => {
  test("完整快照才算系统和流程都完成", () => {
    const state = ready();
    expect(oobeFlowComplete(state)).toBe(true);
    expect(oobeSystemReady(state)).toBe(true);
    expect(oobeComplete(state)).toBe(true);
    expect(oobeNextAction(state)).toBeNull();
  });

  test("明确失败按资源、启用、默认、偏好顺序给出步骤", () => {
    const state = ready();
    state.resourcesReady = false;
    expect(oobeNextAction(state)).toBe("prepare_resources");
    state.resourcesReady = true;
    state.systemEnabled = false;
    expect(oobeNextAction(state)).toBe("enable_system_input_method");
    state.systemEnabled = true;
    state.systemCurrent = false;
    expect(oobeNextAction(state)).toBe("make_default_input_method");
    state.systemCurrent = true;
    state.preferencesSaved = false;
    expect(oobeNextAction(state)).toBe("save_preferences");
  });

  test("iOS 等未知系统能力既不误报失败也不误报完成", () => {
    const state = ready();
    state.systemEnabled = null;
    state.systemCurrent = null;
    expect(oobeFlowComplete(state)).toBe(true);
    expect(oobeSystemReady(state)).toBe(false);
    expect(oobeComplete(state)).toBe(false);
    expect(oobeNextAction(state)).toBeNull();
  });
});
