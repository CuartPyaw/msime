#include "../../src/ipc/ReplyComposer.h"
#include "../core/TestHostOptions.h"
#include <cassert>
#include <chrono>
using namespace msime::windows;

namespace {
FanyImeNamedpipeData key(uint64_t request, uint32_t code, char16_t text, uint32_t modifiers = 0) {
  FanyImeNamedpipeData packet{};
  packet.client_id = 42;
  packet.event_type = FanyImePipeEventType::KeyEvent;
  packet.request_id = request;
  packet.keycode = code;
  packet.wch = text;
  packet.modifiers_down = modifiers;
  return packet;
}

// The Server side of the reference's Japanese key rules, run against a real Engine session in the Japanese scheme.
struct Fixture {
  ServerSession session;
  ReplyComposer composer{42, 1};
  uint64_t request = 0;
  NavigationBindings paging{true, false, false, false, false, false, false};

  explicit Fixture(const std::string &options) : session(42, options) { session.activate(1); }

  std::optional<PendingReply> press(uint32_t code, char16_t text,
                                    WordCharacterBinding binding = WordCharacterBinding::Disabled,
                                    uint32_t modifiers = 0) {
    const auto packet = key(++request, code, text, modifiers);
    auto reply = composer.configured_key(session, packet, 1, TsfPreeditStyle::Pinyin, paging,
                                         std::nullopt, binding);
    if (reply)
      composer.confirm_delivery(42, 1, packet.request_id);
    return reply;
  }
  std::string editing() const { return session.view().at("editing_text").get<std::string>(); }
};
} // namespace

int main() {
  const auto root = std::filesystem::temp_directory_path() /
                    ("msime-japanese-keys-" +
                     std::to_string(std::chrono::steady_clock::now().time_since_epoch().count()));
  std::filesystem::create_directory(root);
  struct Cleanup {
    std::filesystem::path root;
    ~Cleanup() {
      std::error_code ec;
      std::filesystem::remove_all(root, ec);
    }
  } cleanup{root};
  auto options = test_host_options(root);
  options["preferences"]["scheme"] = "japanese";
  options["preferences"]["traditional_chinese_output"] = true;
  const auto serialized = options.dump();

  // Traditional output is a Chinese projection; the reference's CandidateTextForOutput leaves Japanese text alone. The toggle itself survives.
  {
    Fixture japanese(serialized);
    assert(japanese.session.view().value("scheme", 0u) == 3u);
    const auto typed = japanese.press('K', u'k');
    assert(typed && !typed->traditional_output);
    assert(japanese.session.traditional_output());
  }

  // A bare '-' is the long-vowel mark even with minus/equals bound to word-to-character, and it starts a composition from an empty buffer.
  {
    Fixture japanese(serialized);
    const auto opened = japanese.press(0xBD, u'-', WordCharacterBinding::MinusEqual);
    assert(opened && !opened->committed_text);
    assert(japanese.editing() == "-");
    japanese.composer.cancel();
    japanese.session.cancel_composition(1);
    assert(japanese.press('K', u'k', WordCharacterBinding::MinusEqual));
    assert(japanese.press('A', u'a', WordCharacterBinding::MinusEqual));
    const auto vowel = japanese.press(0xBD, u'-', WordCharacterBinding::MinusEqual);
    assert(vowel && !vowel->committed_text);
    assert(japanese.editing() == "ka-");
  }

  // With minus/equals paging on, '=' still never pages in Japanese: it commits the highlighted candidate as punctuation.
  {
    Fixture japanese(serialized);
    assert(japanese.press('K', u'k'));
    assert(japanese.press('A', u'a'));
    const auto generation = japanese.session.view().at("generation");
    const auto equal = japanese.press(0xBB, u'=');
    assert(equal && equal->committed_text && !equal->committed_text->empty());
    assert(japanese.editing().empty());
    assert(japanese.session.view().at("generation") != generation);
  }

  // In V mode '-' is an operator the Engine spells, so minus/equals word-to-character must leave it to the composition; in a pinyin composition it still takes a character.
  {
    auto pinyin = test_host_options(root);
    pinyin["preferences"]["scheme"] = "quanpin";
    pinyin["preferences"]["local_modes"] = {
        {"unicode", true},          {"date_time", true},         {"quick_phrase", true},
        {"emoji", true},            {"kaomoji", true},           {"super_jianpin", true},
        {"temporary_english", true}, {"temporary_japanese", true}, {"expression", true}};
    Fixture expression(pinyin.dump());
    const auto opened = expression.press('V', u'V', WordCharacterBinding::MinusEqual, 1);
    assert(opened && !opened->committed_text);
    assert(expression.session.view().at("local_mode") == "expression");
    assert(expression.press('5', u'5', WordCharacterBinding::MinusEqual));
    const auto minus = expression.press(0xBD, u'-', WordCharacterBinding::MinusEqual);
    assert(minus && !minus->committed_text);
    assert(expression.editing() == "V5-");
    assert(expression.press('2', u'2', WordCharacterBinding::MinusEqual));
    assert(expression.editing() == "V5-2");

    Fixture word(pinyin.dump());
    assert(word.press('N', u'n', WordCharacterBinding::MinusEqual));
    assert(word.press('I', u'i', WordCharacterBinding::MinusEqual));
    // This fixture has no dictionary, so the highlighted row the key takes a character from is empty; the key still ends the composition with a commit.
    const auto first = word.press(0xBD, u'-', WordCharacterBinding::MinusEqual);
    assert(first && first->committed_text);
    assert(word.editing().empty());
  }
}
