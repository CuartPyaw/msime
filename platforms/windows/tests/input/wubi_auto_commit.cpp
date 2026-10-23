#include "ipc/ReplyComposer.h"
#include "../../tsf/HostCharacterResult.h"
#include "../core/TestHostOptions.h"
#include "PipeMetadata.h"
#include <chrono>
#include <cstdio>
#include <cstdlib>

using namespace msime::windows;
namespace {
void require(bool value, const char *message) {
  if (!value) {
    std::fprintf(stderr, "FAIL: %s\n", message);
    std::exit(EXIT_FAILURE);
  }
}
struct Cleanup {
  std::filesystem::path root;
  ~Cleanup() {
    std::error_code error;
    std::filesystem::remove_all(root, error);
  }
};

void scenario(const std::filesystem::path &fixture, TsfPreeditStyle style,
              bool uiless, bool wubi98, bool unique, bool top,
              const std::string &input, const std::string &expected,
              const std::string &remainder, bool select = false,
              std::size_t expected_commits = 1,
              const std::string &expected_phrase = "") {
  const auto root = std::filesystem::temp_directory_path() /
      ("msime-wubi-commit-" + std::to_string(
          std::chrono::steady_clock::now().time_since_epoch().count()));
  Cleanup cleanup{root};
  auto options = test_host_options(root);
  auto &preferences = options["preferences"];
  preferences["scheme"] = "wubi";
  preferences["default_ime_mode"] = "chinese";
  preferences["wubi_profile"] = wubi98 ? "wubi98" : "wubi86";
  preferences["wubi_mixed_pinyin"] = false;
  preferences["wubi_auto_commit_unique"] = unique;
  preferences["wubi_fifth_code_top_commit"] = top;
  // 两份会话读相同的合成表；资源名同时满足五笔按需资源检查。
  for (const char *directory : {"resources", "dictionaries"}) {
    std::filesystem::copy_file(fixture, root / directory / "msime-pinyin.db");
    std::filesystem::copy_file(fixture, root / directory / "msime-wubi.db");
  }
  ServerSession server(42, options.dump());
  server.activate(1);
  ReplyComposer composer(42, 1);
  msime::tsf::EngineSessionAdapter tip;
  std::string raw, error, inserted, counted;
  require(tip.create(options.dump(), &error) && tip.focus(true, &raw, &error),
          "TIP session created");
  std::size_t insertions = 0, observations = 0;
  uint64_t request = 0;
  const auto check_views = [&] {
    require(tip.view(&raw, &error), "TIP view available");
    const auto view = nlohmann::json::parse(raw).at("value");
    require(view.at("editing_text") == server.view().at("editing_text") &&
            view.value("phrase_prefix", "") == server.view().value("phrase_prefix", ""),
            "TIP and Server retain the same remaining code");
  };
  for (unsigned char letter : input) {
    FanyImeNamedpipeData packet{};
    packet.client_id = 42;
    packet.event_type = FanyImePipeEventType::KeyEvent;
    packet.request_id = ++request;
    const bool shift = letter >= 'A' && letter <= 'Z';
    packet.keycode = shift ? letter : letter - 'a' + 'A';
    packet.wch = letter;
    packet.modifiers_down = (shift ? 1u : 0u) |
        (uiless ? FanyImePipeFlags::UiLess : 0u);
    packet.modifiers_down = PipeMetadata::local_character_commit_modifiers(
        packet, FanyImeProtocol::Capabilities | FanyImeProtocol::LocalCharacterCommit, true);
    const auto reply = composer.basic_key(server, packet, 1, style);
    require(reply.has_value(), "Server accepts the character");
    require(tip.character(letter, shift, &raw, &error), "TIP accepts the character");
    msime::tsf::EngineResult result;
    require(tip.parse_result(raw, &result, &error), "TIP character result parsed");
    require(msime::tsf::ApplyHostCharacterResult(result,
        [&](const std::string &text) { inserted += text; ++insertions; return true; },
        [] { return true; }, [] { return true; }) == msime::tsf::CharacterResultStatus::Applied,
        "production TIP result application succeeds");
    // 真正的 TIP 已插入自动提交。无论 worker 在本地结束组字前后送达，
    // 都不能再发送第二份文字；此处直接检查生产投影，避免靠时序掩盖重复。
    require(!reply->worker, "#9452: local Wubi commits must not also emit a commit worker");
    if (result.has_commit) {
      require(reply->committed_text == result.commit,
              "Server observes exactly the text TIP inserted");
      require(!reply->encoded || (static_cast<bool>(*reply->encoded) &&
          (reply->encoded->packet.msg_type == FanyImeReplyType::Preedit ||
           reply->encoded->packet.msg_type == FanyImeReplyType::UiLessComposition ||
           reply->encoded->packet.msg_type == FanyImeReplyType::NavigationIgnored)),
          "character reply only projects the remaining composition");
    } else {
      require(!reply->committed_text, "no commit means no statistics observation");
    }
    if (reply->committed_text) {
      counted += *reply->committed_text;
      ++observations;
    }
    if (reply->encoded) require(static_cast<bool>(*reply->encoded), "valid preedit reply");
    composer.confirm_delivery(42, 1, request);
    check_views();
  }
  if (inserted != expected || counted != expected)
    std::fprintf(stderr, "scenario input=%s style=%u uiless=%d wubi98=%d unique=%d top=%d inserted=%s counted=%s expected=%s\n",
        input.c_str(), static_cast<unsigned>(style), uiless, wubi98, unique, top,
        inserted.c_str(), counted.c_str(), expected.c_str());
  require(inserted == expected && counted == expected,
          "automatic commit text is inserted and counted exactly once");
  require(insertions == (expected.empty() ? 0u : expected_commits) && observations == insertions,
          "one insertion and one observation per completed code");
  require(server.view().at("editing_text") == remainder, "expected remaining code");
  require(server.view().value("phrase_prefix", "") == expected_phrase,
          "top commit keeps the selected phrase when another code remains");
  if (select) {
    // 关闭自动上屏之后，空格仍由正常候选选择回复写入，不走字符提交。
    FanyImeNamedpipeData packet{};
    packet.client_id = 42;
    packet.request_id = ++request;
    packet.keycode = 0x20;
    packet.wch = ' ';
    packet.modifiers_down = uiless ? FanyImePipeFlags::UiLess : 0u;
    const auto reply = composer.basic_key(server, packet, 1, style);
    require(reply && reply->committed_text == "合成甲" && reply->encoded &&
            static_cast<bool>(*reply->encoded), "ordinary candidate selection still commits");
    composer.confirm_delivery(42, 1, request);
  }
}
void fallback_scenario(const std::filesystem::path &fixture,
                       std::uint32_t capabilities, bool has_local_session) {
  const auto root = std::filesystem::temp_directory_path() /
      ("msime-wubi-fallback-" + std::to_string(
          std::chrono::steady_clock::now().time_since_epoch().count()));
  Cleanup cleanup{root};
  auto options = test_host_options(root);
  options["preferences"]["scheme"] = "wubi";
  options["preferences"]["default_ime_mode"] = "chinese";
  for (const char *directory : {"resources", "dictionaries"}) {
    std::filesystem::copy_file(fixture, root / directory / "msime-pinyin.db");
    std::filesystem::copy_file(fixture, root / directory / "msime-wubi.db");
  }
  ServerSession server(42, options.dump());
  server.activate(1);
  ReplyComposer composer(42, 1);
  uint64_t request = 0;
  std::size_t commits = 0;
  for (unsigned char letter : std::string("wqaa")) {
    FanyImeNamedpipeData packet{};
    packet.client_id = 42;
    packet.event_type = FanyImePipeEventType::KeyEvent;
    packet.request_id = ++request;
    packet.keycode = letter - 'a' + 'A';
    packet.wch = letter;
    packet.modifiers_down = PipeMetadata::local_character_commit_modifiers(
        packet, capabilities, has_local_session);
    require(!(packet.modifiers_down & FanyImePipeFlags::LocalCharacterCommit),
            "legacy or missing local session must not claim a local commit");
    const auto reply = composer.basic_key(server, packet, 1, TsfPreeditStyle::Local);
    require(reply.has_value(), "legacy Server accepts the character");
    if (reply->committed_text) {
      require(reply->worker && *reply->committed_text == "合成甲" &&
              reply->worker->at(0) == FanyImeWorkerReplyType::CommitCandidateAndContinue,
              "legacy character path still sends the actual commit worker");
      ++commits;
    } else require(!reply->worker, "unfinished legacy code has no commit worker");
    composer.confirm_delivery(42, 1, request);
  }
  require(commits == 1 && server.view().at("editing_text") == "",
          "legacy path commits once and finishes its code");
}

void negotiation_tests(const std::filesystem::path &fixture) {
  const auto modern = FanyImeProtocol::Capabilities | FanyImeProtocol::LocalCharacterCommit;
  const auto hello = FanyImeProtocol::Hello(42, 1, modern);
  const auto old_server = FanyImeProtocol::Negotiate(hello, FanyImeProtocol::Capabilities);
  const auto old_tip = FanyImeProtocol::Negotiate(FanyImeProtocol::Hello(42, 1), modern);
  const auto both = FanyImeProtocol::Negotiate(hello, modern);
  require(old_server.accepted && old_tip.accepted && both.accepted,
          "optional commit ownership keeps old peers compatible");
  fallback_scenario(fixture, old_server.capabilities, true);
  fallback_scenario(fixture, old_tip.capabilities, false);
  fallback_scenario(fixture, both.capabilities, false);
  FanyImeNamedpipeData packet{};
  packet.event_type = FanyImePipeEventType::KeyEvent;
  packet.modifiers_down = 1u | FanyImePipeFlags::UiLess | PipeMetadata::CandidateActive;
  packet.modifiers_down = PipeMetadata::local_character_commit_modifiers(packet, both.capabilities, true);
  require((packet.modifiers_down & FanyImePipeFlags::LocalCharacterCommit) &&
          PipeMetadata::key_modifiers(packet.modifiers_down) == 1u,
          "ownership metadata does not change keyboard modifiers");
  const auto previous = packet.modifiers_down & ~FanyImePipeFlags::LocalCharacterCommit;
  require(PipeMetadata::local_character_commit_modifiers(packet, old_server.capabilities, true) == previous &&
          PipeMetadata::local_character_commit_modifiers(packet, both.capabilities, false) == previous,
          "retries clear stale ownership after losing capability or session");
  packet.event_type = FanyImePipeEventType::StatusSnapshot;
  require(PipeMetadata::local_character_commit_modifiers(packet, both.capabilities, true) == previous,
          "non-key packets do not carry character commit ownership");
}
} // namespace

int main(int argc, char **argv) {
  require(argc == 2, "synthetic Wubi fixture required");
  try {
    negotiation_tests(argv[1]);
    for (auto style : {TsfPreeditStyle::Local, TsfPreeditStyle::Pinyin, TsfPreeditStyle::Empty})
      for (bool uiless : {false, true})
        for (bool wubi98 : {false, true}) {
          scenario(argv[1], style, uiless, wubi98, true, true, "wqaa", "合成甲", "");
          scenario(argv[1], style, uiless, wubi98, true, true, "wqaawqaa", "合成甲合成甲", "", false, 2);
          scenario(argv[1], style, uiless, wubi98, true, true, "wqabw", "", "w", false, 1, "合成乙");
          scenario(argv[1], style, uiless, wubi98, true, true, "wqabA", "合成乙A", "");
          scenario(argv[1], style, uiless, wubi98, false, true, "wqaaw", "", "w", false, 1, "合成甲");
          scenario(argv[1], style, uiless, wubi98, false, false, "wqaa", "", "wqaa", true);
        }
  } catch (const std::exception &error) {
    std::fprintf(stderr, "FAIL: %s\n", error.what());
    return EXIT_FAILURE;
  }
  std::puts("Wubi auto commit: TIP insertion, Server observation and continuation passed");
  return EXIT_SUCCESS;
}
