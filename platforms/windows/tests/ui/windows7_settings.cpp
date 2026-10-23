#include "../../settings/win7/SettingsWindow.h"
#include "../../settings/win7/PreferencesStore.h"
#include "../../settings/InputSettingsModel.h"
#include "../../../../shared/contracts/msime_edition.h"

#include <commctrl.h>
#include <cassert>
#include <cstdio>
#include <cstdlib>
#include <filesystem>
#include <fstream>
#include <functional>
#include <string>

namespace {
using namespace msime::settings::win7;

struct TemporaryDirectory {
  TemporaryDirectory() {
    wchar_t root[MAX_PATH]{};
    wchar_t unique[MAX_PATH]{};
    const DWORD length = GetTempPathW(MAX_PATH, root);
    assert(length && length < MAX_PATH);
    assert(GetTempFileNameW(root, L"m7s", 0, unique));
    assert(DeleteFileW(unique));
    assert(CreateDirectoryW(unique, nullptr));
    path = unique;
  }
  ~TemporaryDirectory() {
    std::error_code error;
    std::filesystem::remove_all(path, error);
  }
  std::filesystem::path path;
};

struct InjectedDirectory {
  explicit InjectedDirectory(const std::filesystem::path& path, const wchar_t* variable = L"MSIME_CLIENT_STATE_DIR") : name(variable) {
    const DWORD required = GetEnvironmentVariableW(name, nullptr, 0);
    present = required != 0;
    if (present) {
      previous.resize(required);
      const DWORD length = GetEnvironmentVariableW(name, previous.data(), required);
      assert(length < required);
      previous.resize(length);
    }
    assert(SetEnvironmentVariableW(name, path.c_str()));
  }
  ~InjectedDirectory() {
    SetEnvironmentVariableW(name, present ? previous.c_str() : nullptr);
  }
  const wchar_t* name;
  bool present = false;
  std::wstring previous;
};

void pump_until(const std::function<bool()>& condition) {
  const ULONGLONG deadline = GetTickCount64() + 15000;
  for (;;) {
    MSG message{};
    while (PeekMessageW(&message, nullptr, 0, 0, PM_REMOVE)) {
      assert(message.message != WM_QUIT);
      TranslateMessage(&message);
      DispatchMessageW(&message);
    }
    if (condition()) return;
    assert(GetTickCount64() < deadline);
    MsgWaitForMultipleObjects(0, nullptr, FALSE, 25, QS_ALLINPUT);
  }
}

std::wstring caption(HWND window) {
  std::wstring value(static_cast<size_t>(GetWindowTextLengthW(window)) + 1, L'\0');
  const int copied = GetWindowTextW(window, value.data(), static_cast<int>(value.size()));
  value.resize(static_cast<size_t>(copied));
  return value;
}

} // namespace

int main() {
#ifdef _MSC_VER
  _set_error_mode(_OUT_TO_STDERR);
#endif
  using namespace msime::settings::win7;
  TemporaryDirectory temporary;
  InjectedDirectory injected(temporary.path);
#if !MSIME_EDITION_IS_FULL
  // 模拟安装准备阶段写入的状态目录身份；偏好按目录记录取默认值，不按进程包声明取。
  {
    std::ofstream record(temporary.path / L"edition", std::ios::binary);
    record << MSIME_EDITION_ID << '\n';
    assert(record.good());
  }
#endif
  PreferencesStore store(temporary.path.u8string());
  auto loaded = store.load();
  assert(loaded.outcome == Outcome::loaded && loaded.snapshot);
  // Rust 按状态目录的版本记录选默认值，必须与原生编译期产品身份一致。
  assert(loaded.snapshot->preferences().at("scheme") == MSIME_EDITION_DEFAULT_SCHEME);
  auto tokens = loaded.snapshot->preferences().at("ai_assistant").at("tokens");
  tokens["https://compat.example.invalid"] = "synthetic-token-not-a-real-credential";
  auto seeded = store.save(*loaded.snapshot, {
      {{"ai_assistant", "tokens"}, tokens},
      {{"navigation", "tab"}, false}, {{"navigation", "page_up_down"}, false}});
  assert(seeded.outcome == Outcome::saved && seeded.snapshot);
  auto peer = store.load();
  assert(peer.snapshot && peer.snapshot->revision() == seeded.snapshot->revision());
  const auto unchanged = seeded.snapshot->document;
  auto changed = store.save(*seeded.snapshot, {{{"keybindings", "switch_language_caps_lock"}, true}});
  assert(changed.outcome == Outcome::saved && changed.snapshot);
  assert(seeded.snapshot->document == unchanged);
  auto conflict = store.save(*peer.snapshot, {{{"keybindings", "switch_language_caps_lock"}, false}});
  assert(conflict.outcome == Outcome::conflict && !conflict.snapshot);
  assert(store.load().snapshot->revision() == changed.snapshot->revision());
  const auto exported = store.export_document();
  assert(exported && exported->find("synthetic-token-not-a-real-credential") == std::string::npos);
  auto local_only = store.save(*changed.snapshot, {{{"keybindings", "switch_language_caps_lock"}, false},
      {{"diagnostic_log", "server"}, true}});
  assert(local_only.outcome == Outcome::saved && local_only.snapshot);
  auto restored = store.import_document(*local_only.snapshot, *exported);
  assert(restored.outcome == Outcome::saved && restored.snapshot);
  assert(restored.snapshot->preferences().at("keybindings").at("switch_language_caps_lock") == true);
  assert(restored.snapshot->preferences().at("diagnostic_log").at("server") == true);
  assert(restored.snapshot->preferences().at("ai_assistant").at("tokens") == tokens);
  const auto import_conflict = store.import_document(*changed.snapshot, *exported);
  assert(import_conflict.outcome == Outcome::conflict && import_conflict.error == "settings_conflict");
  assert(store.import_document(*restored.snapshot, "{}").outcome == Outcome::storage);
  assert(store.load().snapshot->revision() == restored.snapshot->revision());
  assert(store.load().snapshot->preferences().at("ai_assistant").at("tokens") == tokens);
  auto invalid = store.save(*restored.snapshot, {{{"keybindings", "synthetic_unknown_key"}, true}});
  assert(invalid.outcome == Outcome::invalid_change);
  auto rejected = store.save(*restored.snapshot, {{{"keybindings", "switch_language_caps_lock"}, "invalid"}});
  assert(rejected.outcome == Outcome::storage);
  assert(store.load().snapshot->revision() == restored.snapshot->revision());

  // 真实 HWND、系统控件通知和工作线程贯通到同源 Rust，不触碰测试目录之外的数据。
  const auto file = temporary.path / L"synthetic-settings.json";
  const auto resources = temporary.path / L"resources";
  const auto custom_directory = resources / L"helpcodes" / L"custom";
  const auto plugin_directory = temporary.path / L"plugins" / L"helpcode" / L"synthetic-codes";
  std::filesystem::create_directories(custom_directory);
  std::filesystem::create_directories(plugin_directory);
  {
    std::ofstream custom(custom_directory / L"synthetic.txt", std::ios::binary);
    custom << "# name: 合成辅助码\n# name_en: Synthetic helper\n字=ab\n";
    std::ofstream manifest(plugin_directory / L"plugin.toml", std::ios::binary);
    manifest << "schema_version = 1\nkind = 'helpcode'\nid = 'synthetic-codes'\nname = '合成插件'\nversion = '1'\nlicense = 'CC0-1.0'\n[helpcode]\ntable = 'table.txt'\n";
    std::ofstream table(plugin_directory / L"table.txt", std::ios::binary);
    table << "字=cd\n";
    std::ofstream options(temporary.path / L"runtime-options.json", std::ios::binary);
    options << Json{{"resources", resources.u8string()}}.dump();
    assert(custom.good() && manifest.good() && table.good() && options.good());
  }
  InjectedDirectory host_options(temporary.path / L"runtime-options.json", L"MSIME_CLIENT_HOST_OPTIONS");
  unsigned selections = 0;
  bool cancel_selection = false;
  bool allow_fuzzy_reset = false;
  unsigned reset_confirmations = 0;
  SettingsWindow window([&](HWND owner, bool save) -> std::optional<std::filesystem::path> {
    DWORD process = 0;
    GetWindowThreadProcessId(owner, &process);
    assert(process == GetCurrentProcessId());
    if (cancel_selection) return std::nullopt;
    assert(save == (selections == 0));
    ++selections;
    return file;
  }, [&](HWND owner, const wchar_t* title, const wchar_t* text) {
    assert(IsWindow(owner));
    assert(std::wstring_view(title) == L"关闭模糊音");
    assert(std::wstring_view(text).find(L"清空") != std::wstring_view::npos);
    ++reset_confirmations;
    return allow_fuzzy_reset;
  });
  assert(window.create(GetModuleHandleW(nullptr)));
  const HWND host = window.handle();
  const HWND body = GetDlgItem(host, page_body);
  const HWND language = GetDlgItem(body, language_switch);
  const HWND caps = GetDlgItem(body, caps_lock);
  assert(body && language && caps);
#if MSIME_EDITION_IS_FULL
  assert(GetDlgItem(body, japanese_toggle) != nullptr);
#else
  assert(GetDlgItem(body, japanese_toggle) == nullptr);
#endif
  pump_until([&] { return IsWindowEnabled(language) != FALSE; });
  assert(SendMessageW(caps, BM_GETCHECK, 0, 0) == BST_CHECKED);
  const auto before_controls = store.load().snapshot->preferences();
  const auto select_page = [&](int index) {
    const HWND selector = GetDlgItem(host, page_selector);
    assert(selector);
    SendMessageW(selector, CB_SETCURSEL, index, 0);
    SendMessageW(host, WM_COMMAND, MAKEWPARAM(page_selector, CBN_SELCHANGE), reinterpret_cast<LPARAM>(selector));
    pump_until([&] { return IsWindowEnabled(language) != FALSE; });
  };
  SendMessageW(language, CB_SETCURSEL, 3, 0);
  SendMessageW(body, WM_COMMAND, MAKEWPARAM(language_switch, CBN_SELCHANGE), reinterpret_cast<LPARAM>(language));
  assert(!IsWindowEnabled(language));
  pump_until([&] { return IsWindowEnabled(language) != FALSE; });
  auto saved = store.load();
  assert(saved.snapshot);
  const auto& keys = saved.snapshot->preferences().at("keybindings");
  assert(!keys.at("switch_language_shift").get<bool>());
  assert(!keys.at("switch_language_ctrl").get<bool>());
  assert(!keys.at("switch_language_ctrl_alt_space").get<bool>());
  assert(saved.snapshot->preferences().at("ai_assistant") == before_controls.at("ai_assistant"));

  SendMessageW(caps, BM_CLICK, 0, 0);
  pump_until([&] { return IsWindowEnabled(caps) != FALSE; });
  saved = store.load();
  assert(!saved.snapshot->preferences().at("keybindings").at("switch_language_caps_lock").get<bool>());
  const bool previous_width = saved.snapshot->preferences().at("keybindings").at("toggle_fullwidth_option_shift_h").get<bool>();
  const auto external = store.save(*saved.snapshot, {{{"keybindings", "toggle_fullwidth_option_shift_h"}, !previous_width}});
  assert(external.outcome == Outcome::saved);
  SendMessageW(caps, BM_CLICK, 0, 0);
  pump_until([&] { return IsWindowEnabled(caps) != FALSE; });
  assert(caption(GetDlgItem(host, status_text)).find(L"其他窗口") != std::wstring::npos);
  assert(SendMessageW(caps, BM_GETCHECK, 0, 0) == BST_UNCHECKED);
  saved = store.load();
  assert(saved.snapshot->revision() == external.snapshot->revision());
  assert(saved.snapshot->preferences().at("keybindings").at("toggle_fullwidth_option_shift_h").get<bool>() != previous_width);

  const HWND tabs = GetDlgItem(host, page_tabs);
  const auto select_tab = [&](const wchar_t* title) {
    for (int index = 0; index < TabCtrl_GetItemCount(tabs); ++index) {
      wchar_t text[128]{};
      TCITEMW item{};
      item.mask = TCIF_TEXT;
      item.pszText = text;
      item.cchTextMax = 128;
      assert(TabCtrl_GetItem(tabs, index, &item));
      if (std::wstring_view(text) != title) continue;
      TabCtrl_SetCurSel(tabs, index);
      NMHDR changed_tab{tabs, page_tabs, TCN_SELCHANGE};
      SendMessageW(host, WM_NOTIFY, page_tabs, reinterpret_cast<LPARAM>(&changed_tab));
      return;
    }
    assert(false && "Requested settings tab does not exist");
  };
  TabCtrl_SetCurSel(tabs, 1);
  NMHDR notification{tabs, page_tabs, TCN_SELCHANGE};
  SendMessageW(host, WM_NOTIFY, page_tabs, reinterpret_cast<LPARAM>(&notification));
  const HWND reference = GetDlgItem(body, reference_text);
  assert((GetWindowLongPtrW(reference, GWL_STYLE) & ES_READONLY) != 0);
  assert(caption(reference).find(L"Page Up / Page Down") != std::wstring::npos);
  assert(caption(reference).find(L"Shift+Tab / Tab") != std::wstring::npos);
  assert(store.load().snapshot->revision() == saved.snapshot->revision());
  // 以词定字与翻页共用键组时，复合修改必须一次落盘，不能先保存出非法中间态。
  select_page(3);
  select_tab(L"选词与翻页");
  const HWND word = GetDlgItem(body, word_character_keys);
  const HWND brackets = GetDlgItem(body, navigation_brackets);
  const HWND backspace = GetDlgItem(body, backspace_reverts);
  assert(word && brackets && backspace);
  assert((GetWindowLongPtrW(word, GWL_STYLE) & WS_VISIBLE) != 0);
  assert(store.load().snapshot->revision() == saved.snapshot->revision());
  auto selection_seed = store.save(*saved.snapshot, {{{"word_character", "enabled"}, false},
      {{"navigation", "brackets"}, false}, {{"navigation", "minus_equal"}, false}});
  assert(selection_seed.outcome == Outcome::saved);
  SendMessageW(host, WM_COMMAND, MAKEWPARAM(reload_settings, BN_CLICKED), 0);
  pump_until([&] { return IsWindowEnabled(word) != FALSE; });
  const auto select_word = [&](const wchar_t* title) {
    const auto index = SendMessageW(word, CB_FINDSTRINGEXACT, static_cast<WPARAM>(-1), reinterpret_cast<LPARAM>(title));
    assert(index != CB_ERR);
    SendMessageW(word, CB_SETCURSEL, index, 0);
    SendMessageW(body, WM_COMMAND, MAKEWPARAM(word_character_keys, CBN_SELCHANGE), reinterpret_cast<LPARAM>(word));
    pump_until([&] { return IsWindowEnabled(word) != FALSE; });
  };
  select_word(L"[ / ]");
  saved = store.load();
  assert(saved.snapshot->revision() == selection_seed.snapshot->revision() + 1);
  assert(saved.snapshot->preferences().at("word_character").at("enabled") == true);
  assert(saved.snapshot->preferences().at("word_character").at("keys") == "brackets");
  const auto word_revision = saved.snapshot->revision();
  SendMessageW(brackets, BM_CLICK, 0, 0);
  pump_until([&] { return IsWindowEnabled(brackets) != FALSE; });
  saved = store.load();
  assert(saved.snapshot->revision() == word_revision + 1);
  assert(saved.snapshot->preferences().at("navigation").at("brackets") == true);
  assert(saved.snapshot->preferences().at("word_character").at("enabled") == false);
  assert(SendMessageW(word, CB_FINDSTRINGEXACT, static_cast<WPARAM>(-1), reinterpret_cast<LPARAM>(L"[ / ]")) == CB_ERR);
  select_word(L"- / =");
  saved = store.load();
  assert(saved.snapshot->preferences().at("word_character").at("keys") == "minus_equal");
  assert(saved.snapshot->preferences().at("word_character").at("enabled") == true);
  const bool previous_backspace = saved.snapshot->preferences().value("backspace_reverts_selection", true);
  SendMessageW(backspace, BM_CLICK, 0, 0);
  pump_until([&] { return IsWindowEnabled(backspace) != FALSE; });
  saved = store.load();
  assert(saved.snapshot->preferences().value("backspace_reverts_selection", true) != previous_backspace);
  assert(saved.snapshot->preferences().at("word_character").at("enabled") == true);
  const HWND mouse = GetDlgItem(body, navigation_mouse_wheel);
  assert(mouse && SendMessageW(mouse, BM_GETCHECK, 0, 0) == BST_UNCHECKED);
  SendMessageW(mouse, BM_CLICK, 0, 0);
  pump_until([&] { return IsWindowEnabled(mouse) != FALSE; });
  assert(store.load().snapshot->preferences().at("navigation").at("mouse_wheel") == true);
  SendMessageW(mouse, BM_CLICK, 0, 0);
  pump_until([&] { return IsWindowEnabled(mouse) != FALSE; });
  assert(!store.load().snapshot->preferences().at("navigation").contains("mouse_wheel"));
  select_word(L"关闭");
  saved = store.load();
  assert(saved.snapshot->preferences().at("word_character").at("keys") == "minus_equal");
  assert(saved.snapshot->preferences().at("word_character").at("enabled") == false);
  assert(saved.snapshot->preferences().at("navigation").at("tab") == false);
  assert(saved.snapshot->preferences().at("navigation").at("page_up_down") == false);
  assert(saved.snapshot->preferences().at("ai_assistant").at("tokens") == tokens);
  const auto select_value = [&](int id, int index) {
    const HWND control = GetDlgItem(body, id);
    assert(control && IsWindowEnabled(control));
    SendMessageW(control, CB_SETCURSEL, index, 0);
    SendMessageW(body, WM_COMMAND, MAKEWPARAM(id, CBN_SELCHANGE), reinterpret_cast<LPARAM>(control));
    pump_until([&] { return IsWindowEnabled(control) != FALSE; });
  };
  const auto click = [&](int id) {
    const HWND control = GetDlgItem(body, id);
    assert(control && IsWindowEnabled(control));
    SendMessageW(control, BM_CLICK, 0, 0);
    pump_until([&] { return IsWindowEnabled(GetDlgItem(body, punctuation_mode)) != FALSE; });
  };
  // 输入页的标签切换不落盘；繁简只修改对应开关，不改其他宿主的繁体变体。
  select_tab(L"输出设置");
  assert((GetWindowLongPtrW(GetDlgItem(body, traditional_output), GWL_STYLE) & WS_VISIBLE) != 0);
  assert(store.load().snapshot->revision() == saved.snapshot->revision());
  const auto before_output = saved.snapshot->preferences();
  const bool old_traditional = before_output.at("traditional_chinese_output").get<bool>();
  select_value(traditional_output, old_traditional ? 0 : 1);
  saved = store.load();
  assert(saved.snapshot->preferences().at("traditional_chinese_output").get<bool>() != old_traditional);
  assert(saved.snapshot->preferences().value("traditional_chinese_variant", "s2t") ==
      before_output.value("traditional_chinese_variant", "s2t"));
  assert(caption(GetDlgItem(body, output_description)).find(old_traditional ? L"简体" : L"繁体") != std::wstring::npos);
  const bool old_spacing = saved.snapshot->preferences().value("chinese_english_spacing", false);
  click(chinese_english_spacing);
  assert(store.load().snapshot->preferences().value("chinese_english_spacing", false) != old_spacing);
  click(chinese_english_spacing);
  assert(store.load().snapshot->preferences().value("chinese_english_spacing", false) == old_spacing);
  const auto old_width = store.load().snapshot->preferences().at("character_width");
  click(fullwidth_output);
  assert(store.load().snapshot->preferences().at("character_width") != old_width);
  saved = store.load();
  select_page(4);
  assert((GetWindowLongPtrW(GetDlgItem(body, punctuation_mode), GWL_STYLE) & WS_VISIBLE) != 0);
  assert(store.load().snapshot->revision() == saved.snapshot->revision());
  select_value(punctuation_mode, 0);
  saved = store.load();
  const auto punctuation_revision = saved.snapshot->revision();
  select_value(punctuation_mode, 3);
  saved = store.load();
  assert(saved.snapshot->revision() == punctuation_revision + 1);
  assert(saved.snapshot->preferences().at("chinese_punctuation") == true);
  assert(saved.snapshot->preferences().at("punctuation_lock") == "english");
  select_value(punctuation_mode, 1);
  saved = store.load();
  assert(saved.snapshot->revision() == punctuation_revision + 2);
  assert(saved.snapshot->preferences().at("chinese_punctuation") == false);
  assert(saved.snapshot->preferences().at("punctuation_lock") == "follow");
  select_value(quote_style, 1);
  assert(store.load().snapshot->preferences().at("quote_style") == "corner");
  select_value(quote_style, 0);
  assert(!store.load().snapshot->preferences().contains("quote_style"));
  select_value(bracket_style, 1);
  assert(store.load().snapshot->preferences().at("bracket_style") == "hollow");
  select_value(bracket_style, 0);
  assert(!store.load().snapshot->preferences().contains("bracket_style"));
  saved = store.load();
  const auto bad_style = store.save(*saved.snapshot, {{{"quote_style"}, "synthetic-invalid-style"}});
  assert(bad_style.outcome == Outcome::storage);
  assert(store.load().snapshot->revision() == saved.snapshot->revision());
  const auto bool_style = store.save(*saved.snapshot, {{{"quote_style"}, true}});
  assert(bool_style.outcome == Outcome::invalid_change);
  const auto repeat = saved.snapshot->preferences().at("smart_punctuation_repeat");
  const bool old_smart = saved.snapshot->preferences().at("smart_punctuation").get<bool>();
  click(smart_punctuation);
  saved = store.load();
  assert(saved.snapshot->revision() == punctuation_revision + 7);
  for (const auto* field : {"smart_punctuation", "smart_punctuation_direct_digit", "smart_punctuation_direct_letter"})
    assert(saved.snapshot->preferences().at(field).get<bool>() != old_smart);
  assert(saved.snapshot->preferences().at("smart_punctuation_repeat") == repeat);
  assert((IsWindowEnabled(GetDlgItem(body, smart_repeat)) != FALSE) == !old_smart);
  if (old_smart) click(smart_punctuation);
  click(smart_direct_digit);
  click(smart_direct_letter);
  assert(caption(GetDlgItem(body, smart_description)).find(L"暂不生效") != std::wstring::npos);
  click(smart_punctuation);
  assert(!IsWindowEnabled(GetDlgItem(body, smart_direct_digit)));
  assert(!IsWindowEnabled(GetDlgItem(body, smart_direct_letter)));
  click(smart_punctuation);
  saved = store.load();
  assert(saved.snapshot->preferences().at("smart_punctuation_direct_digit") == true);
  assert(saved.snapshot->preferences().at("smart_punctuation_direct_letter") == true);
  assert(saved.snapshot->preferences().at("smart_punctuation_repeat") == repeat);
  assert(saved.snapshot->preferences().at("ai_assistant").at("tokens") == tokens);
  // 混输总开关仅改变可用状态，保留子项与其他宿主使用的最短触发长度。
  select_page(3);
  select_tab(L"候选与联想");
  assert((GetWindowLongPtrW(GetDlgItem(body, mixed_enabled), GWL_STYLE) & WS_VISIBLE) != 0);
  assert(store.load().snapshot->revision() == saved.snapshot->revision());
#if MSIME_EDITION_TEMPORARY_JAPANESE
  assert(GetDlgItem(body, mixed_japanese));
#else
  assert(!GetDlgItem(body, mixed_japanese));
#endif
  const auto before_mixed = saved.snapshot->preferences().at("mixed_input");
  const bool was_mixed = before_mixed.value("enabled", true);
  click(mixed_enabled);
  saved = store.load();
  assert(saved.snapshot->preferences().at("mixed_input").value("enabled", true) != was_mixed);
  assert(saved.snapshot->preferences().at("mixed_input").at("english") == before_mixed.at("english"));
  assert(saved.snapshot->preferences().at("mixed_input").at("emoji") == before_mixed.at("emoji"));
  assert(saved.snapshot->preferences().at("mixed_input").at("minimum_prefix") == before_mixed.at("minimum_prefix"));
  assert((IsWindowEnabled(GetDlgItem(body, mixed_english)) != FALSE) == !was_mixed);
  if (was_mixed) click(mixed_enabled);
  select_value(mixed_first_page_count, 2);
  assert(store.load().snapshot->preferences().at("mixed_input").at("first_page_count") == 3);
  select_value(mixed_first_page_count, 0);
  assert(!store.load().snapshot->preferences().at("mixed_input").contains("first_page_count"));
  const bool old_symbols = store.load().snapshot->preferences().at("mixed_input").value("symbols", true);
  click(mixed_symbols);
  assert(store.load().snapshot->preferences().at("mixed_input").value("symbols", true) != old_symbols);
  click(mixed_symbols);
  assert(store.load().snapshot->preferences().at("mixed_input").value("symbols", true) == old_symbols);
  // 选择来源和编辑文本只改草稿；失败后无关偏好保存也不能把草稿抹掉。
  saved = store.load();
  const auto before_draft = saved.snapshot->revision();
  select_value(cloud_source, 1);
  assert(store.load().snapshot->revision() == before_draft);
  assert(IsWindowEnabled(GetDlgItem(body, cloud_endpoint)));
  assert(!IsWindowEnabled(GetDlgItem(body, cloud_body)));
  SetWindowTextW(GetDlgItem(body, cloud_endpoint), L"https://synthetic.example.invalid/candidates");
  SetWindowTextW(GetDlgItem(body, cloud_headers), L"{");
  click(cloud_save);
  assert(store.load().snapshot->revision() == before_draft);
  assert(caption(GetDlgItem(host, status_text)).find(L"未保存的内容已保留") != std::wstring::npos);
  assert(caption(GetDlgItem(body, cloud_headers)) == L"{");
  click(learn_candidates);
  assert(caption(GetDlgItem(body, cloud_headers)) == L"{");
  assert(SendMessageW(GetDlgItem(body, cloud_source), CB_GETCURSEL, 0, 0) == 1);
  select_value(cloud_method, 1);
  select_value(cloud_response_format, 2);
  assert(IsWindowEnabled(GetDlgItem(body, cloud_body)));
  assert(IsWindowEnabled(GetDlgItem(body, cloud_response_path)));
  SetWindowTextW(GetDlgItem(body, cloud_headers), L"{\"X-Synthetic\":\"synthetic-value\"}");
  SetWindowTextW(GetDlgItem(body, cloud_body), L"{\"query\":\"{query}\",\"test\":\"合成\"}");
  SetWindowTextW(GetDlgItem(body, cloud_response_path), L"choices.0.message.content");
  SetWindowTextW(GetDlgItem(body, cloud_timeout), L"2500");
  const auto before_cloud_save = store.load().snapshot->revision();
  click(cloud_save);
  saved = store.load();
  assert(saved.snapshot->revision() == before_cloud_save + 1);
  const auto cloud_config = saved.snapshot->preferences().at("cloud_candidate_provider");
  assert(cloud_config.at("source") == "custom" && cloud_config.at("method") == "POST");
  assert(cloud_config.at("headers_json") == "{\"X-Synthetic\":\"synthetic-value\"}");
  assert(cloud_config.at("body_template") == "{\"query\":\"{query}\",\"test\":\"合成\"}");
  assert(cloud_config.at("response_path") == "choices.0.message.content");
  assert(cloud_config.at("timeout_ms") == 2500);
  assert(saved.snapshot->preferences().at("ai_assistant").at("tokens") == tokens);
  SetWindowTextW(GetDlgItem(body, cloud_timeout), L"invalid");
  click(cloud_save);
  assert(store.load().snapshot->revision() == saved.snapshot->revision());
  assert(caption(GetDlgItem(body, cloud_timeout)) == L"invalid");
  // 新的服务快照胜过过期草稿，CAS 冲突不能静默覆盖别处的选择。
  auto external_cloud = cloud_config;
  external_cloud["timeout_ms"] = 3500;
  const auto cloud_peer = store.save(*saved.snapshot, {{{"cloud_candidate_provider"}, external_cloud}});
  assert(cloud_peer.outcome == Outcome::saved);
  SetWindowTextW(GetDlgItem(body, cloud_timeout), L"3000");
  click(cloud_save);
  saved = store.load();
  assert(saved.snapshot->revision() == cloud_peer.snapshot->revision());
  assert(saved.snapshot->preferences().at("cloud_candidate_provider").at("timeout_ms") == 3500);
  assert(caption(GetDlgItem(body, cloud_timeout)) == L"3500");
  assert(caption(GetDlgItem(host, status_text)).find(L"其他窗口") != std::wstring::npos);
  select_tab(L"中英模式");
  const auto mode_revision = saved.snapshot->revision();
  assert(store.load().snapshot->revision() == mode_revision);
  const auto old_mode = saved.snapshot->preferences().at("default_ime_mode");
  select_value(default_ime_mode, old_mode == "english" ? 0 : 1);
  assert(store.load().snapshot->preferences().at("default_ime_mode") != old_mode);
  select_value(ime_mode_scope, 1);
  assert(store.load().snapshot->preferences().at("ime_mode_scope") == "global");
  const bool old_hud = store.load().snapshot->preferences().at("input_mode_hud").get<bool>();
  click(input_mode_hud);
  assert(store.load().snapshot->preferences().at("input_mode_hud").get<bool>() != old_hud);
  saved = store.load();
  // 同一张规则表里其他平台的标识必须保留，新增 Windows 规则沿用现有路径与大小写规范化。
  auto rule_seed = store.save(*saved.snapshot, {{{"app_input_mode_rules"}, Json{{"com.synthetic.editor", "english"}}}});
  assert(rule_seed.outcome == Outcome::saved);
  SendMessageW(host, WM_COMMAND, MAKEWPARAM(reload_settings, BN_CLICKED), 0);
  pump_until([&] { return IsWindowEnabled(GetDlgItem(body, app_rule_add)) != FALSE; });
  assert(!IsWindowEnabled(GetDlgItem(body, app_rule_mode)));
  SetWindowTextW(GetDlgItem(body, app_rule_name), L"  \"C:\\synthetic\\合成EDITOR.EXE\"  ");
  click(app_rule_add);
  saved = store.load();
  assert(saved.snapshot->revision() == rule_seed.snapshot->revision() + 1);
  assert(saved.snapshot->preferences().at("app_input_mode_rules").at("合成editor.exe") == "chinese");
  assert(saved.snapshot->preferences().at("app_input_mode_rules").at("com.synthetic.editor") == "english");
  assert(saved.snapshot->preferences().at("ime_mode_scope") == "global");
  SetWindowTextW(GetDlgItem(body, app_rule_name), L"合成Editor.EXE");
  click(app_rule_add);
  assert(store.load().snapshot->revision() == saved.snapshot->revision());
  assert(caption(GetDlgItem(host, status_text)).find(L"已经有例外") != std::wstring::npos);
  const auto select_rule = [&](const wchar_t* prefix) {
    const HWND list = GetDlgItem(body, app_rule_list);
    const auto index = SendMessageW(list, LB_FINDSTRING, static_cast<WPARAM>(-1), reinterpret_cast<LPARAM>(prefix));
    assert(index != LB_ERR);
    SendMessageW(list, LB_SETCURSEL, index, 0);
    SendMessageW(body, WM_COMMAND, MAKEWPARAM(app_rule_list, LBN_SELCHANGE), reinterpret_cast<LPARAM>(list));
  };
  select_rule(L"合成editor.exe");
  select_value(app_rule_mode, 1);
  saved = store.load();
  assert(saved.snapshot->preferences().at("app_input_mode_rules").at("合成editor.exe") == "english");
  assert(saved.snapshot->preferences().at("app_input_mode_rules").at("com.synthetic.editor") == "english");
  SetWindowTextW(GetDlgItem(body, app_rule_name), L"synthetic.txt");
  click(app_rule_add);
  assert(store.load().snapshot->revision() == saved.snapshot->revision());
  assert(caption(GetDlgItem(host, status_text)).find(L".exe") != std::wstring::npos);
  click(app_rule_remove);
  saved = store.load();
  assert(!saved.snapshot->preferences().at("app_input_mode_rules").contains("合成editor.exe"));
  assert(saved.snapshot->preferences().at("app_input_mode_rules").at("com.synthetic.editor") == "english");
  select_rule(L"com.synthetic.editor");
  click(app_rule_remove);
  saved = store.load();
  assert(!saved.snapshot->preferences().contains("app_input_mode_rules"));
  assert(!IsWindowEnabled(GetDlgItem(body, app_rule_remove)));
  assert(!IsWindowEnabled(GetDlgItem(body, app_rule_mode)));
  assert(saved.snapshot->preferences().at("ai_assistant").at("tokens") == tokens);
  // 快捷模式的总开关不清除单项；/ 和 @ 独立，地名只从属于 @。
  select_tab(L"高级选项");
  assert(store.load().snapshot->revision() == saved.snapshot->revision());
  const auto advanced_seed = store.save(*saved.snapshot, {
      {{"local_modes", "shift_letter_entry"}, true}, {{"local_modes", "quick_phrase"}, true},
      {{"local_modes", "mention"}, false}, {{"local_modes", "mention_places"}, false},
      {{"fuzzy_pinyin", "enabled"}, false}, {{"fuzzy_pinyin", "rules"}, Json::array()},
      {{"fuzzy_pinyin", "seeded"}, false}, {{"learning"}, true},
      {{"frequency", "mode"}, "promote"}, {{"frequency", "trigger_count"}, 9}, {{"frequency", "linear_step"}, 8}});
  assert(advanced_seed.outcome == Outcome::saved);
  SendMessageW(host, WM_COMMAND, MAKEWPARAM(reload_settings, BN_CLICKED), 0);
  pump_until([&] { return IsWindowEnabled(GetDlgItem(body, shift_letter_modes)) != FALSE; });
  const auto local_control = [&](std::wstring_view key) {
    for (size_t index = 0; index < msime::settings::input::local_modes.size(); ++index)
      if (msime::settings::input::local_modes[index].id == key) return local_mode_first + static_cast<int>(index);
    assert(false && "Requested local mode does not exist");
    return -1;
  };
#if MSIME_EDITION_TEMPORARY_JAPANESE
  assert(GetDlgItem(body, local_control(L"temporary_japanese")));
#else
  assert(!GetDlgItem(body, local_control(L"temporary_japanese")));
#endif
  click(shift_letter_modes);
  assert(!IsWindowEnabled(GetDlgItem(body, local_control(L"quick_phrase"))));
  assert(IsWindowEnabled(GetDlgItem(body, local_control(L"command"))));
  assert(IsWindowEnabled(GetDlgItem(body, local_control(L"mention"))));
  assert(store.load().snapshot->preferences().at("local_modes").at("quick_phrase") == true);
  click(shift_letter_modes);
  click(local_control(L"chaizi"));
  assert(store.load().snapshot->preferences().at("local_modes").at("chaizi") == true);
  assert(!IsWindowEnabled(GetDlgItem(body, local_control(L"mention_places"))));
  click(local_control(L"mention"));
  assert(IsWindowEnabled(GetDlgItem(body, local_control(L"mention_places"))));
  click(local_control(L"mention_places"));
  click(local_control(L"mention"));
  assert(!IsWindowEnabled(GetDlgItem(body, local_control(L"mention_places"))));
  assert(store.load().snapshot->preferences().at("local_modes").at("mention_places") == true);
  const auto before_fuzzy = store.load().snapshot->revision();
  click(fuzzy_enabled);
  saved = store.load();
  assert(saved.snapshot->revision() == before_fuzzy + 1);
  assert(saved.snapshot->preferences().at("fuzzy_pinyin").at("seeded") == true);
  assert(saved.snapshot->preferences().at("fuzzy_pinyin").at("rules").size() == 11);
  click(fuzzy_rule_first);
  click(fuzzy_enabled);
  assert(!IsWindowEnabled(GetDlgItem(body, fuzzy_rule_first)));
  assert(store.load().snapshot->preferences().at("fuzzy_pinyin").at("rules").size() == 10);
  click(fuzzy_enabled);
  assert(store.load().snapshot->preferences().at("fuzzy_pinyin").at("rules").size() == 10);
  saved = store.load();
  click(fuzzy_reset);
  assert(reset_confirmations == 1 && store.load().snapshot->revision() == saved.snapshot->revision());
  allow_fuzzy_reset = true;
  click(fuzzy_reset);
  assert(reset_confirmations == 2);
  saved = store.load();
  assert(saved.snapshot->preferences().at("fuzzy_pinyin").at("enabled") == false);
  assert(saved.snapshot->preferences().at("fuzzy_pinyin").at("rules").empty());
  assert(saved.snapshot->preferences().at("fuzzy_pinyin").at("seeded") == true);
  click(fuzzy_enabled);
  assert(store.load().snapshot->preferences().at("fuzzy_pinyin").at("rules").empty());
  // 超出界面常用档但仍合法的既有调频值照实展示，不因刷新归一化。
  assert(SendMessageW(GetDlgItem(body, frequency_trigger), CB_GETCOUNT, 0, 0) == 7);
  assert(SendMessageW(GetDlgItem(body, frequency_trigger), CB_GETCURSEL, 0, 0) == 6);
  assert(store.load().snapshot->preferences().at("frequency").at("trigger_count") == 9);
  select_value(frequency_mode, 0);
  assert(!IsWindowEnabled(GetDlgItem(body, frequency_trigger)));
  assert(!IsWindowEnabled(GetDlgItem(body, frequency_step)));
  select_value(frequency_mode, 3);
  assert(IsWindowEnabled(GetDlgItem(body, frequency_trigger)));
  assert(IsWindowEnabled(GetDlgItem(body, frequency_step)));
  select_value(frequency_trigger, 2);
  select_value(frequency_step, 4);
  assert(store.load().snapshot->preferences().at("frequency").at("trigger_count") == 3);
  assert(store.load().snapshot->preferences().at("frequency").at("linear_step") == 5);
  select_value(frequency_mode, 4);
  assert(!IsWindowEnabled(GetDlgItem(body, frequency_step)));
  select_tab(L"候选与联想");
  click(learn_candidates);
  select_tab(L"高级选项");
  assert(!IsWindowEnabled(GetDlgItem(body, frequency_mode)));
  saved = store.load();
  assert(saved.snapshot->preferences().at("frequency").at("linear_step") == 5);
  assert(saved.snapshot->preferences().at("ai_assistant").at("tokens") == tokens);
  bool offers_helpcode = false;
  for (std::string_view scheme : {MSIME_EDITION_INPUT_SCHEMES})
    offers_helpcode = offers_helpcode || scheme == "quanpin" || scheme == "shuangpin";
  assert((GetDlgItem(body, helpcode_first) != nullptr) == offers_helpcode);
  if (offers_helpcode) {
    // 当前未安装的插件仍可见；换回内置方案时同次清除插件，其他插件配置与另一族不动。
    auto plugins = saved.snapshot->preferences().value("plugins", Json::object());
    plugins["helpcode_pack_quanpin"] = "synthetic-missing";
    plugins["helpcode_pack_shuangpin"] = "synthetic-other";
    plugins["combo_counter"] = true;
    const auto before_helpcode = store.save(*saved.snapshot, {{{"plugins"}, plugins},
        {{"quanpin_helpcode", "enabled"}, true}, {{"quanpin_helpcode", "schema"}, "custom/synthetic-missing"}});
    assert(before_helpcode.outcome == Outcome::saved);
    SendMessageW(host, WM_COMMAND, MAKEWPARAM(reload_settings, BN_CLICKED), 0);
    pump_until([&] { return IsWindowEnabled(language) != FALSE; });
    const HWND schema = GetDlgItem(body, helpcode_first + 1);
    const auto find_option = [&](const wchar_t* label) {
      return static_cast<int>(SendMessageW(schema, CB_FINDSTRINGEXACT, static_cast<WPARAM>(-1), reinterpret_cast<LPARAM>(label)));
    };
    const auto selected_text = [&] {
      const auto selected = SendMessageW(schema, CB_GETCURSEL, 0, 0);
      const auto length = SendMessageW(schema, CB_GETLBTEXTLEN, selected, 0);
      assert(length >= 0);
      std::wstring value(static_cast<size_t>(length) + 1, L'\0');
      SendMessageW(schema, CB_GETLBTEXT, selected, reinterpret_cast<LPARAM>(value.data()));
      value.resize(static_cast<size_t>(length));
      return value;
    };
    assert(selected_text() == L"synthetic-missing（插件，未找到）");
    assert(find_option(L"custom/synthetic-missing") >= 0);
    assert(find_option(L"合成辅助码 (Synthetic helper)") >= 0);
    assert(find_option(L"合成插件（插件）") >= 0);
    assert(store.load().snapshot->revision() == before_helpcode.snapshot->revision());
    select_value(helpcode_first + 1, find_option(L"小鹤"));
    saved = store.load();
    assert(saved.snapshot->revision() == before_helpcode.snapshot->revision() + 1);
    assert(saved.snapshot->preferences().at("quanpin_helpcode").at("schema") == "xiaohe");
    assert(!saved.snapshot->preferences().at("plugins").contains("helpcode_pack_quanpin"));
    assert(saved.snapshot->preferences().at("plugins").at("helpcode_pack_shuangpin") == "synthetic-other");
    assert(saved.snapshot->preferences().at("plugins").at("combo_counter") == true);
    select_value(helpcode_first + 1, find_option(L"合成插件（插件）"));
    saved = store.load();
    assert(saved.snapshot->preferences().at("plugins").at("helpcode_pack_quanpin") == "synthetic-codes");
    assert(saved.snapshot->preferences().at("quanpin_helpcode").at("schema") == "xiaohe");
    click(helpcode_first);
    assert(!IsWindowEnabled(schema));
    assert(IsWindowEnabled(GetDlgItem(body, helpcode_first + 2)));
    click(helpcode_first + 2);
    assert(store.load().snapshot->preferences().at("plugins").at("helpcode_pack_quanpin") == "synthetic-codes");
    click(helpcode_first);
    select_value(helpcode_first + 1, find_option(L"合成辅助码 (Synthetic helper)"));
    saved = store.load();
    assert(saved.snapshot->preferences().at("quanpin_helpcode").at("schema") == "custom/synthetic");
    assert(!saved.snapshot->preferences().at("plugins").contains("helpcode_pack_quanpin"));
    const auto revision = saved.snapshot->revision();
    std::filesystem::remove_all(custom_directory);
    std::filesystem::remove_all(plugin_directory);
    select_page(0);
    select_page(3);
    assert(selected_text() == L"custom/synthetic");
    assert(find_option(L"合成插件（插件）") == CB_ERR);
    assert(store.load().snapshot->revision() == revision);
    assert(store.load().snapshot->preferences().at("ai_assistant").at("tokens") == tokens);
  }
  // 原有自定义尺寸只在用户选择标准档位后才改变；两个尺寸字段原子保存。
  select_page(1);
  const HWND size = GetDlgItem(body, toolbar_size);
  assert((GetWindowLongPtrW(size, GWL_STYLE) & WS_VISIBLE) != 0);
  assert((GetWindowLongPtrW(tabs, GWL_STYLE) & WS_VISIBLE) == 0);
#if MSIME_EDITION_HANDWRITING
  assert(GetDlgItem(body, toolbar_handwriting) != nullptr);
#else
  assert(GetDlgItem(body, toolbar_handwriting) == nullptr);
#endif
  auto custom = store.save(*saved.snapshot, {{{"floating_toolbar", "scale_percent"}, 110},
      {{"floating_toolbar", "font_size"}, 20}});
  assert(custom.outcome == Outcome::saved);
  SendMessageW(host, WM_COMMAND, MAKEWPARAM(reload_settings, BN_CLICKED), 0);
  pump_until([&] { return IsWindowEnabled(size) != FALSE; });
  assert(SendMessageW(size, CB_GETCOUNT, 0, 0) == 6);
  assert(SendMessageW(size, CB_GETCURSEL, 0, 0) == 5);
  assert(store.load().snapshot->revision() == custom.snapshot->revision());
  SendMessageW(size, CB_SETCURSEL, 4, 0);
  SendMessageW(body, WM_COMMAND, MAKEWPARAM(toolbar_size, CBN_SELCHANGE), reinterpret_cast<LPARAM>(size));
  pump_until([&] { return IsWindowEnabled(size) != FALSE; });
  saved = store.load();
  assert(saved.snapshot->revision() == custom.snapshot->revision() + 1);
  assert(saved.snapshot->preferences().at("floating_toolbar").at("scale_percent") == 150);
  assert(saved.snapshot->preferences().at("floating_toolbar").at("font_size") == 28);
  assert(SendMessageW(size, CB_GETCOUNT, 0, 0) == 5);
  const HWND emoji = GetDlgItem(body, toolbar_emoji);
  const bool previous_emoji = saved.snapshot->preferences().at("floating_toolbar").at("emoji").get<bool>();
  SendMessageW(emoji, BM_CLICK, 0, 0);
  pump_until([&] { return IsWindowEnabled(emoji) != FALSE; });
  saved = store.load();
  assert(saved.snapshot->preferences().at("floating_toolbar").at("emoji").get<bool>() != previous_emoji);
  assert(saved.snapshot->preferences().at("ai_assistant").at("tokens") == tokens);
  select_page(2);
  const HWND server_log = GetDlgItem(body, diagnostic_server);
  const HWND tsf_log = GetDlgItem(body, diagnostic_tsf);
  assert(server_log && tsf_log);
  assert((GetWindowLongPtrW(server_log, GWL_STYLE) & WS_VISIBLE) != 0);
  assert(caption(GetDlgItem(body, data_directory_text)) == temporary.path.wstring());
  const bool previous_server = saved.snapshot->preferences().at("diagnostic_log").at("server").get<bool>();
  SendMessageW(server_log, BM_CLICK, 0, 0);
  pump_until([&] { return IsWindowEnabled(server_log) != FALSE; });
  saved = store.load();
  assert(saved.snapshot->preferences().at("diagnostic_log").at("server").get<bool>() != previous_server);
  SendMessageW(body, WM_COMMAND, MAKEWPARAM(export_settings, BN_CLICKED),
      reinterpret_cast<LPARAM>(GetDlgItem(body, export_settings)));
  pump_until([&] { return IsWindowEnabled(server_log) != FALSE; });
  if (!std::filesystem::is_regular_file(file)) {
    std::fwprintf(stderr, L"Export status: %ls\n", caption(GetDlgItem(host, status_text)).c_str());
    std::abort();
  }
  const auto exported_revision = saved.snapshot->revision();
  assert(store.load().snapshot->revision() == exported_revision);
  const bool exported_caps = saved.snapshot->preferences().at("keybindings").at("switch_language_caps_lock").get<bool>();
  select_page(0);
  SendMessageW(caps, BM_CLICK, 0, 0);
  pump_until([&] { return IsWindowEnabled(caps) != FALSE; });
  assert(store.load().snapshot->preferences().at("keybindings").at("switch_language_caps_lock").get<bool>() != exported_caps);
  select_page(2);
  // 文件选择边界使用合成路径；实际导入不能恢复文件中不应携带的本机日志开关。
  SendMessageW(tsf_log, BM_CLICK, 0, 0);
  pump_until([&] { return IsWindowEnabled(tsf_log) != FALSE; });
  saved = store.load();
  const bool current_tsf = saved.snapshot->preferences().at("diagnostic_log").at("tsf").get<bool>();
  SendMessageW(body, WM_COMMAND, MAKEWPARAM(import_settings, BN_CLICKED),
      reinterpret_cast<LPARAM>(GetDlgItem(body, import_settings)));
  pump_until([&] { return IsWindowEnabled(tsf_log) != FALSE; });
  saved = store.load();
  assert(saved.snapshot->revision() == exported_revision + 3);
  assert(saved.snapshot->preferences().at("keybindings").at("switch_language_caps_lock").get<bool>() == exported_caps);
  assert(saved.snapshot->preferences().at("diagnostic_log").at("tsf").get<bool>() == current_tsf);
  assert(saved.snapshot->preferences().at("ai_assistant").at("tokens") == tokens);
  assert(selections == 2);
  cancel_selection = true;
  const auto before_cancel = saved.snapshot->revision();
  SendMessageW(body, WM_COMMAND, MAKEWPARAM(import_settings, BN_CLICKED),
      reinterpret_cast<LPARAM>(GetDlgItem(body, import_settings)));
  assert(selections == 2 && IsWindowEnabled(tsf_log));
  assert(store.load().snapshot->revision() == before_cancel);
  select_page(0);
  TabCtrl_SetCurSel(tabs, 0);
  SendMessageW(host, WM_NOTIFY, page_tabs, reinterpret_cast<LPARAM>(&notification));
  SendMessageW(language, CB_SETCURSEL, 1, 0);
  SendMessageW(body, WM_COMMAND, MAKEWPARAM(language_switch, CBN_SELCHANGE), reinterpret_cast<LPARAM>(language));
  assert(!IsWindowEnabled(language));
  // 关闭会完成刚受理的修改并等待工作线程退出，不丢掉排队写入。
  DestroyWindow(host);
  assert(window.handle() == nullptr);
  saved = store.load();
  assert(saved.snapshot->preferences().at("keybindings").at("switch_language_ctrl").get<bool>());
  assert(!saved.snapshot->preferences().at("keybindings").at("switch_language_shift").get<bool>());
  assert(!saved.snapshot->preferences().at("keybindings").at("switch_language_ctrl_alt_space").get<bool>());
  std::puts("Windows 7 settings: native controls, CAS, file roundtrip, local state retention and cancellation passed");
}
