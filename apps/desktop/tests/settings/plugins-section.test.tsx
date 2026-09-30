// @vitest-environment jsdom
import { saveSettingsNow, settingsFormReady } from "../support/settings-form";
import { afterEach, expect, test, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import {
  LocalModesSection,
  PluginsSection,
  SettingsPage,
  defaultLocalModes,
  defaultPluginPreferences,
  mentionListIssue,
  pluginErrorMessage,
  pluginPreferences,
  withoutRemovedPack,
  type MentionEntry,
  type PluginCatalogResult,
  type PluginClient,
  type PluginPackage,
  type PluginPreferences,
  type Preferences,
  type Snapshot,
} from "@msime/ui";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const pack = (overrides: Partial<PluginPackage> & Pick<PluginPackage, "id" | "kind">) =>
  ({
    name: overrides.id,
    version: "1.0.0",
    license: "CC0-1.0",
    author: null,
    description: null,
    builtin: false,
    ...overrides,
  }) as PluginPackage;

const catalog: PluginCatalogResult = {
  packages: [
    pack({ id: "default", kind: "sound", name: "默认", builtin: true, mode: "keys" }),
    pack({ id: "twinkle", kind: "sound", name: "小星星", builtin: true, mode: "sequence" }),
    pack({ id: "typewriter", kind: "sound", name: "打字机", author: "测试者", mode: "keys" }),
    pack({ id: "rain", kind: "music", name: "雨声", license: "CC-BY-4.0", tracks: ["a.ogg"] }),
    pack({
      id: "signature",
      kind: "command_table",
      name: "签名",
      commands: [{ trigger: "sig", title: "签名", template: "{date} 测试" }],
    }),
  ],
  issues: [{ kind: "sound", folder: "broken", reason: "missing plugin.toml" }],
};

function fakeClient(overrides: Partial<PluginClient> = {}): PluginClient {
  return {
    catalog: vi.fn(async () => catalog),
    importPack: vi.fn(async () => null),
    remove: vi.fn(async () => undefined),
    loadMentions: vi.fn(async (): Promise<MentionEntry[]> => [{ text: "张三", key: "zhang'san" }]),
    saveMentions: vi.fn(async () => undefined),
    ...overrides,
  };
}

function renderSection(
  props: Partial<Parameters<typeof PluginsSection>[0]> = {},
  preferences: PluginPreferences = defaultPluginPreferences,
) {
  const onChange = vi.fn();
  const onError = vi.fn();
  const confirm = vi.fn(async () => true);
  const client = props.client === undefined && !("client" in props) ? fakeClient() : props.client;
  render(
    <PluginsSection
      preferences={preferences}
      keySound
      music
      triggers
      active
      onChange={onChange}
      onError={onError}
      confirm={confirm}
      {...props}
      client={client}
    />,
  );
  return { onChange, onError, confirm, client };
}

test("switches key sounds and keeps the other effect settings", () => {
  const { onChange } = renderSection({ client: undefined });

  fireEvent.click(screen.getByRole("switch", { name: "按键音" }));
  expect(onChange).toHaveBeenLastCalledWith({
    ...defaultPluginPreferences,
    key_sound: { ...defaultPluginPreferences.key_sound, enabled: true },
  });

  fireEvent.click(screen.getByRole("switch", { name: "上屏音" }));
  expect(onChange).toHaveBeenLastCalledWith({
    ...defaultPluginPreferences,
    commit_sound: { enabled: true },
  });

  fireEvent.click(screen.getByRole("switch", { name: "成就音效" }));
  expect(onChange).toHaveBeenLastCalledWith({
    ...defaultPluginPreferences,
    achievements: { enabled: true },
  });

  fireEvent.change(screen.getByRole("slider", { name: "音效音量" }), { target: { value: "80" } });
  expect(onChange).toHaveBeenLastCalledWith({
    ...defaultPluginPreferences,
    key_sound: { ...defaultPluginPreferences.key_sound, volume: 80 },
  });
});

test("offers the melody pack only in melody mode, and the mode only while key sounds are on", async () => {
  const on = {
    ...defaultPluginPreferences,
    key_sound: { ...defaultPluginPreferences.key_sound, enabled: true },
  };
  const { onChange } = renderSection({}, on);
  await screen.findByText("打字机 1.0.0");

  expect(screen.queryByRole("combobox", { name: "旋律" })).toBeNull();
  fireEvent.click(screen.getByRole("radio", { name: "按键旋律" }));
  expect(onChange).toHaveBeenLastCalledWith({
    ...on,
    key_sound: { ...on.key_sound, mode: "melody" },
  });

  cleanup();
  renderSection({}, { ...on, key_sound: { ...on.key_sound, mode: "melody" } });
  await screen.findByText("打字机 1.0.0");
  const melody = screen.getByRole("combobox", { name: "旋律" });
  expect(
    within(melody)
      .getAllByRole("option")
      .map((option) => option.textContent),
  ).toEqual(["小星星（内置）"]);
  const packs = screen.getByRole("combobox", { name: "音效包" });
  expect(
    within(packs)
      .getAllByRole("option")
      .map((option) => option.textContent),
  ).toEqual(["默认（内置）", "打字机"]);

  cleanup();
  renderSection({ client: undefined });
  expect(
    screen.getByRole("radiogroup", { name: "发声方式" }).querySelector("input")?.disabled,
  ).toBe(true);
});

test("names a selected pack that is no longer installed instead of showing another", async () => {
  renderSection(
    {},
    {
      ...defaultPluginPreferences,
      key_sound: { ...defaultPluginPreferences.key_sound, pack: "gone" },
    },
  );
  await screen.findByText("打字机 1.0.0");
  const packs = screen.getByRole("combobox", { name: "音效包" }) as HTMLSelectElement;
  expect(packs.value).toBe("gone");
  expect(within(packs).getByRole("option", { name: "gone（未找到）" })).toBeTruthy();

  cleanup();
  // A host with no pack store lists nothing, so the selection is shown as it is rather than as missing.
  renderSection({ client: undefined });
  const selected = screen.getByRole("combobox", { name: "音效包" });
  expect(
    within(selected)
      .getAllByRole("option")
      .map((option) => option.textContent),
  ).toEqual(["default"]);
});

test("chooses a music pack, which starts unselected", async () => {
  const { onChange } = renderSection();
  await screen.findByText("雨声 1.0.0");
  const select = screen.getByRole("combobox", { name: "音乐包" }) as HTMLSelectElement;
  expect(select.value).toBe("");

  fireEvent.change(select, { target: { value: "rain" } });
  expect(onChange).toHaveBeenLastCalledWith({
    ...defaultPluginPreferences,
    music: { ...defaultPluginPreferences.music, pack: "rain" },
  });
});

test("hides the groups a host does not back", () => {
  renderSection({ client: undefined, keySound: false, music: false, triggers: false });
  expect(screen.queryByRole("switch", { name: "按键音" })).toBeNull();
  expect(screen.queryByRole("switch", { name: "背景音乐" })).toBeNull();
  expect(screen.queryByText("指令表")).toBeNull();
  expect(screen.queryByText("@ 名单")).toBeNull();
  expect(screen.queryByRole("button", { name: "导入文件夹" })).toBeNull();
});

test("lists every pack with its licence and removes only installed ones", async () => {
  const selected = {
    ...defaultPluginPreferences,
    key_sound: { ...defaultPluginPreferences.key_sound, pack: "typewriter" },
  };
  const { client, confirm, onChange } = renderSection({}, selected);
  const list = await screen.findByLabelText("已安装的扩展包");

  await waitFor(() => expect(within(list).getByText("打字机 1.0.0")).toBeTruthy());
  expect(within(list).getByText("音效包 · 作者 测试者 · 许可证 CC0-1.0")).toBeTruthy();
  expect(within(list).getAllByText("音效包 · 内置 · 许可证 CC0-1.0")).toHaveLength(2);
  expect(within(list).getByText("音乐包 · 许可证 CC-BY-4.0")).toBeTruthy();
  expect(within(list).queryByRole("button", { name: "删除默认" })).toBeNull();
  expect(screen.getByText("音效包 broken 无法载入：missing plugin.toml")).toBeTruthy();

  fireEvent.click(within(list).getByRole("button", { name: "删除打字机" }));
  await waitFor(() => expect(client!.remove).toHaveBeenCalledWith("sound", "typewriter"));
  expect(confirm).toHaveBeenCalledWith(expect.objectContaining({ danger: true }));
  // The selection falls back to the built-in pack rather than naming one that is gone.
  expect(onChange).toHaveBeenCalledWith(defaultPluginPreferences);
});

test("keeps a pack when the removal is not confirmed", async () => {
  const client = fakeClient();
  render(
    <PluginsSection
      client={client}
      preferences={defaultPluginPreferences}
      keySound
      music
      triggers
      active
      onChange={vi.fn()}
      onError={vi.fn()}
      confirm={async () => false}
    />,
  );
  fireEvent.click(await screen.findByRole("button", { name: "删除打字机" }));
  await waitFor(() => expect(client.remove).not.toHaveBeenCalled());
});

test("imports through the host picker and reports what was installed", async () => {
  const imported = pack({ id: "piano", kind: "sound", name: "钢琴", version: "2.0.0" });
  const client = fakeClient({ importPack: vi.fn(async () => imported) });
  renderSection({ client });
  await screen.findByText("打字机 1.0.0");

  fireEvent.click(screen.getByRole("button", { name: "导入 .zip" }));
  await waitFor(() => expect(client.importPack).toHaveBeenCalledWith("archive"));
  expect(await screen.findByText("已导入音效包「钢琴」2.0.0。")).toBeTruthy();
  expect(client.catalog).toHaveBeenCalledTimes(2);

  fireEvent.click(screen.getByRole("button", { name: "导入文件夹" }));
  await waitFor(() => expect(client.importPack).toHaveBeenLastCalledWith("folder"));
});

test("says which rule a refused pack broke", async () => {
  const client = fakeClient({
    importPack: vi.fn(async () => {
      throw { code: "plugin_invalid", detail: "plugins may not request permissions" };
    }),
  });
  const { onError } = renderSection({ client });
  await screen.findByText("打字机 1.0.0");

  fireEvent.click(screen.getByRole("button", { name: "导入文件夹" }));
  await waitFor(() =>
    expect(onError).toHaveBeenCalledWith(
      "扩展包不符合要求（plugins may not request permissions）。",
    ),
  );
});

test("a cancelled picker changes nothing", async () => {
  const client = fakeClient();
  renderSection({ client });
  await screen.findByText("打字机 1.0.0");
  fireEvent.click(screen.getByRole("button", { name: "导入文件夹" }));
  await waitFor(() => expect(client.importPack).toHaveBeenCalled());
  expect(client.catalog).toHaveBeenCalledTimes(1);
  expect(screen.queryByRole("status")).toBeNull();
});

test("enables command tables in order and lists enabled tables that are gone", async () => {
  const { onChange } = renderSection(
    {},
    { ...defaultPluginPreferences, command_tables: ["removed"] },
  );
  const table = await screen.findByRole("switch", { name: "签名" });
  expect(screen.getByText("1 条指令：/sig 签名")).toBeTruthy();

  fireEvent.click(table);
  expect(onChange).toHaveBeenLastCalledWith({
    ...defaultPluginPreferences,
    command_tables: ["removed", "signature"],
  });

  fireEvent.click(screen.getByRole("switch", { name: "removed（未找到）" }));
  expect(onChange).toHaveBeenLastCalledWith({ ...defaultPluginPreferences, command_tables: [] });
});

test("edits the local @ list and refuses a malformed reading before saving", async () => {
  const client = fakeClient();
  renderSection({ client });
  expect(await screen.findByDisplayValue("张三")).toBeTruthy();
  expect(screen.getByText(/名单只保存在本机，不随账号同步/)).toBeTruthy();
  const save = screen.getByRole("button", { name: "保存名单" }) as HTMLButtonElement;
  expect(save.disabled).toBe(true);

  fireEvent.click(screen.getByRole("button", { name: "添加" }));
  const names = screen.getAllByLabelText("名字或地点");
  const keys = screen.getAllByLabelText("拼音");
  fireEvent.change(names[1], { target: { value: " 北京 " } });
  fireEvent.change(keys[1], { target: { value: "Bei Jing" } });
  expect(screen.getByRole("alert").textContent).toContain("第 2 行的拼音只能是小写字母");
  expect(save.disabled).toBe(true);

  fireEvent.change(keys[1], { target: { value: "bei'jing" } });
  expect(screen.queryByRole("alert")).toBeNull();
  fireEvent.click(save);
  await waitFor(() =>
    expect(client.saveMentions).toHaveBeenCalledWith([
      { text: "张三", key: "zhang'san" },
      { text: "北京", key: "bei'jing" },
    ]),
  );
  await waitFor(() => expect(save.disabled).toBe(true));

  fireEvent.click(screen.getByRole("button", { name: "删除第 1 行" }));
  expect(screen.queryByDisplayValue("张三")).toBeNull();
});

test("checks a name list the way the host does", () => {
  expect(mentionListIssue([{ text: "张三", key: "" }])).toBeNull();
  expect(mentionListIssue([{ text: "Alice", key: "" }])).toBeNull();
  expect(mentionListIssue([{ text: " ", key: "" }])).toBe("第 1 行还没有填写名字或地点。");
  expect(mentionListIssue([{ text: "x".repeat(200), key: "" }])).toBe("第 1 行太长了。");
  expect(mentionListIssue([{ text: "a\u0007", key: "" }])).toBe("第 1 行含有控制字符。");
  expect(mentionListIssue([{ text: "张三", key: "zhang''san" }])).toContain("第 1 行的拼音");
  expect(mentionListIssue([{ text: "张三", key: "a".repeat(65) }])).toContain("第 1 行的拼音");
  expect(
    mentionListIssue([
      { text: "张三", key: "" },
      { text: "张三", key: "zs" },
    ]),
  ).toBe("「张三」重复了。");
  expect(
    mentionListIssue(Array.from({ length: 1001 }, (_, index) => ({ text: `${index}`, key: "" }))),
  ).toBe("名单最多 1000 条。");
});

test("decodes host failures, falling back for unknown ones", () => {
  expect(pluginErrorMessage({ code: "plugin_reserved" }, "失败")).toBe(
    "这个 id 属于内置音效包，不能覆盖或删除。",
  );
  expect(pluginErrorMessage({ code: "plugin_archive", detail: "too many members" }, "失败")).toBe(
    "压缩包无法读取（too many members）。",
  );
  expect(pluginErrorMessage({ code: "mention_invalid", detail: null }, "失败")).toBe("名单有误。");
  expect(pluginErrorMessage(new Error("boom"), "失败")).toBe("失败");
});

test("fills the defaults the document leaves out and forgets removed packs", () => {
  expect(pluginPreferences({})).toEqual(defaultPluginPreferences);
  expect(
    pluginPreferences({
      plugins: { music: { enabled: true } } as unknown as Preferences["plugins"],
    }).music,
  ).toEqual({ enabled: true, pack: "", volume: 30 });

  const selected: PluginPreferences = {
    ...defaultPluginPreferences,
    key_sound: { ...defaultPluginPreferences.key_sound, pack: "custom" },
    melody: { pack: "custom" },
    music: { enabled: true, pack: "rain", volume: 40 },
    command_tables: ["a", "b"],
  };
  expect(withoutRemovedPack(selected, "sound", "custom")).toEqual({
    ...selected,
    key_sound: { ...selected.key_sound, pack: "default" },
    melody: { pack: "twinkle" },
  });
  expect(withoutRemovedPack(selected, "music", "rain").music).toEqual({
    enabled: false,
    pack: "",
    volume: 40,
  });
  expect(withoutRemovedPack(selected, "music", "other")).toBe(selected);
  expect(withoutRemovedPack(selected, "command_table", "a").command_tables).toEqual(["b"]);
});

test("shows the V, / and @ switches only where the host routes them", () => {
  const onChange = vi.fn();
  render(<LocalModesSection preferences={defaultLocalModes} ios={false} onChange={onChange} />);
  expect(screen.queryByRole("switch", { name: /V 模式/ })).toBeNull();

  cleanup();
  // A document written while the three were off leaves them out, as the defaults do.
  const stored = defaultLocalModes;
  expect("expression" in stored).toBe(false);
  render(<LocalModesSection preferences={stored} ios={false} triggers onChange={onChange} />);
  const expression = screen.getByRole("switch", { name: /V 模式/ }) as HTMLInputElement;
  expect(expression.checked).toBe(false);
  expect(screen.getByRole("switch", { name: /\/ 模式/ })).toBeTruthy();
  expect(screen.getByRole("switch", { name: /@ 模式/ })).toBeTruthy();

  fireEvent.click(screen.getByRole("switch", { name: /@ 模式/ }));
  expect(onChange).toHaveBeenCalledWith({ ...stored, mention: true });
});

const snapshot: Snapshot = {
  format_version: 1,
  revision: 3,
  preferences: {
    scheme: "quanpin",
    shuangpin_profile: "xiaohe",
    candidate_page_size: 5,
    learning: true,
    chinese_punctuation: true,
  },
};

test("the 扩展 page saves plugin settings into the preferences document", async () => {
  const save = vi.fn(async (_revision: number, preferences: Preferences) => ({
    ...snapshot,
    revision: 4,
    preferences,
  }));
  render(
    <SettingsPage
      client={{
        load: async () => snapshot,
        save,
        host: { platform: "macos", key_sound: true, music: true, plugin_triggers: true } as never,
        plugins: fakeClient(),
      }}
    />,
  );
  await settingsFormReady();
  fireEvent.click(screen.getByRole("button", { name: "扩展" }));
  const form = screen.getByRole("group", { name: "扩展" });
  await within(form).findByText("打字机 1.0.0");

  fireEvent.click(within(form).getByRole("switch", { name: "按键音" }));
  saveSettingsNow();
  await waitFor(() => expect(save).toHaveBeenCalled());
  expect(save.mock.calls.at(-1)?.[1].plugins?.key_sound.enabled).toBe(true);
});

test("the 扩展 page is not offered on a phone or a host that backs none of it", async () => {
  render(
    <SettingsPage
      client={{
        load: async () => snapshot,
        save: vi.fn(),
        host: { platform: "macos" } as never,
      }}
    />,
  );
  await settingsFormReady();
  expect(screen.queryByRole("button", { name: "扩展" })).toBeNull();

  cleanup();
  render(
    <SettingsPage
      client={{
        load: async () => snapshot,
        save: vi.fn(),
        host: { platform: "android", key_sound: true, plugin_triggers: true } as never,
        plugins: fakeClient(),
      }}
    />,
  );
  await settingsFormReady();
  expect(screen.queryByRole("button", { name: "扩展" })).toBeNull();
});
