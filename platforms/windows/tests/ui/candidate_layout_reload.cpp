#include "../../src/candidate/CandidateCardSize.h"
#include "../../src/candidate/CandidateLayoutSettings.h"
#include <cassert>

int main() {
  using namespace msime::windows;
  const auto defaults = candidate_layout_settings(nlohmann::json::object());
  assert(defaults && !defaults->horizontal && defaults->show_preedit);
  // The shared `wubi_code_hint` is on when absent or null, and must survive the atomic encoding.
  assert(defaults->wubi_code_hint);
  assert(candidate_layout_settings({{"wubi_code_hint", nullptr}})->wubi_code_hint);
  const auto hint_off = candidate_layout_settings({{"wubi_code_hint", false}});
  assert(hint_off && !hint_off->wubi_code_hint);
  assert(!CandidateLayoutSettings::decode(hint_off->encode()).wubi_code_hint);
  assert(CandidateLayoutSettings::decode(defaults->encode()).wubi_code_hint);
  assert(!candidate_layout_settings({{"wubi_code_hint", "false"}}));
  // 共享的 `show_app_logo` 缺值或为 null 时按新装处理，不画 logo；打开后要经得起原子量编码。
  assert(!defaults->show_app_logo);
  assert(!candidate_layout_settings({{"show_app_logo", nullptr}})->show_app_logo);
  const auto logo_on = candidate_layout_settings({{"show_app_logo", true}});
  assert(logo_on && logo_on->show_app_logo);
  assert(CandidateLayoutSettings::decode(logo_on->encode()).show_app_logo);
  assert(!CandidateLayoutSettings::decode(defaults->encode()).show_app_logo);
  assert(!candidate_layout_settings({{"show_app_logo", "yes"}}));
  // 释义预留行数：翻译和离线英文释义都关时为 0；打开任一个时按目标语言数算，第二种语言为空或与第一种相同时只算一行。
  assert(defaults->reserved_gloss_lines == 0);
  assert(candidate_reserved_gloss_lines({{"candidate_english_gloss", true},
                                         {"translation_target_language", "en"}}) == 1);
  assert(candidate_reserved_gloss_lines({{"candidate_translations", true},
                                         {"translation_target_language", "en"},
                                         {"translation_secondary_language", nullptr}}) == 1);
  assert(candidate_reserved_gloss_lines({{"candidate_translations", true},
                                         {"translation_target_language", "en"},
                                         {"translation_secondary_language", "en"}}) == 1);
  const auto two = candidate_layout_settings({{"candidate_translations", true},
                                              {"translation_target_language", "en"},
                                              {"translation_secondary_language", "ja"}});
  assert(two && two->reserved_gloss_lines == 2);
  assert(CandidateLayoutSettings::decode(two->encode()).reserved_gloss_lines == 2);
  assert(CandidateLayoutSettings::decode(two->encode()).wubi_code_hint);
  assert(candidate_reserved_gloss_lines({{"candidate_translations", false},
                                         {"translation_secondary_language", "ja"}}) == 0);
  // 第一种语言缺值按英语；不支持的第二种语言不占行，和 macOS 取目标语言的规则相同。
  assert(candidate_reserved_gloss_lines({{"candidate_english_gloss", true},
                                         {"translation_secondary_language", "en"}}) == 1);
  assert(candidate_reserved_gloss_lines({{"candidate_english_gloss", true},
                                         {"translation_secondary_language", "fr"}}) == 2);
  assert(candidate_reserved_gloss_lines({{"candidate_translations", true},
                                         {"translation_target_language", "en"},
                                         {"translation_secondary_language", "zh"}}) == 1);
  for (bool horizontal : {false, true}) {
    for (bool preedit : {false, true}) {
      auto settings = candidate_layout_settings(
          {{"candidate_layout", horizontal ? "horizontal" : "vertical"},
           {"candidate_preedit_style", preedit ? "pinyin" : "empty"}});
      assert(settings);
      auto decoded = CandidateLayoutSettings::decode(settings->encode());
      assert(decoded.horizontal == horizontal &&
             decoded.show_preedit == preedit);
    }
  }
  for (auto invalid : {nlohmann::json{{"candidate_layout", "diagonal"}},
                       nlohmann::json{{"candidate_layout", 2}},
                       nlohmann::json{{"candidate_preedit_style", "raw"}},
                       nlohmann::json{{"candidate_preedit_style", nullptr}},
                       nlohmann::json::array()})
    assert(!candidate_layout_settings(invalid));
  CandidateCardInput input;
  input.items = {{90}, {90}, {90}};
  input.max_width = 1000;
  input.max_height = 1000;
  input.preedit_visible = true;
  input.horizontal = false;
  const auto vertical = candidate_card_size(input);
  input.horizontal = true;
  const auto horizontal = candidate_card_size(input);
  assert(horizontal.width > vertical.width);
  assert(horizontal.height < vertical.height);
  input.preedit_visible = false;
  const auto hidden_preedit = candidate_card_size(input);
  assert(hidden_preedit.height < horizontal.height);
}
