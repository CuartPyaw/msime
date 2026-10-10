#pragma once

#include <algorithm>
#include <cctype>
#include <chrono>
#include <optional>
#include <string>
#include <string_view>
#include <unordered_set>
#include <utility>
#include <vector>

namespace msime::windows {
// 水杉账号的候选释义接口，和 macOS 的 BackendChatClient.translate、Linux provider 的 anonymous_translation 是同一个：一次最多 32 个中文词、一种目标语言，源语言固定是中文。
inline constexpr std::string_view account_gloss_url = "https://api.msime.app/v1/translate";
inline constexpr size_t account_gloss_maximum_words = 32;
inline constexpr size_t account_gloss_maximum_word_bytes = 2048;
inline constexpr size_t account_gloss_maximum_value_bytes = 4096;
// 取不到任何会话时补注册本机匿名账号（macOS 的 BackendAnonymousAccount.ensureSignedIn），失败后隔多久再试。注册会阻塞翻译线程最多约一分钟，离线时不能每一页都试一次。
inline constexpr auto account_registration_retry = std::chrono::minutes(10);

// 一个候选是否可以问账号：共享层给每个候选算好的 online_gloss（只有中文候选，且不是表情、颜文字）。
struct AccountGlossCandidate {
  std::string text;
  bool online_gloss = false;
};

// 这一页要发给账号的词：可以问账号、本机词典还没有答上（answered 里带非空释义的不算）、也不在缓存里（known 返回真）的候选，按页上的顺序，同一个词只问一次。和 macOS synchronizeAccountGloss 一样，本机词典先答，账号只补它们留下的空。
template <typename Known>
std::vector<std::string> account_gloss_words(
    const std::vector<AccountGlossCandidate> &candidates,
    const std::vector<std::pair<std::string, std::string>> &answered, Known &&known) {
  std::vector<std::string> words;
  std::unordered_set<std::string> seen;
  for (const auto &candidate : candidates) {
    if (!candidate.online_gloss || candidate.text.empty() ||
        candidate.text.size() > account_gloss_maximum_word_bytes)
      continue;
    const bool local = std::any_of(answered.begin(), answered.end(), [&](const auto &entry) {
      return entry.first == candidate.text && !entry.second.empty();
    });
    if (local || known(candidate.text) || !seen.insert(candidate.text).second)
      continue;
    words.push_back(candidate.text);
    if (words.size() == account_gloss_maximum_words)
      break;
  }
  return words;
}

// 接口的 target_lang 是大写的语言代码（EN、JA）。代码只能是 ASCII 字母和连字符，空的或超过 16 字节的不发。
inline std::optional<std::string> account_gloss_target(std::string_view language) {
  if (language.empty() || language.size() > 16 || language == "zh")
    return std::nullopt;
  std::string target;
  target.reserve(language.size());
  for (const unsigned char ch : language) {
    if (!std::isalpha(ch) && ch != '-')
      return std::nullopt;
    target.push_back(static_cast<char>(std::toupper(ch)));
  }
  return target;
}

// 校验一次回复的 data 并换成每个词的释义，和 macOS BackendChatClient.translate 的判据相同：条数必须和问的词一样多，每条不超过 4096 字节，不含换行、回车和制表符以外的控制字符（另外拒收 DEL：共享会话收释义时遇到控制字符会把整批拒掉），任何一条不合格整批作废（返回空），这一页的词留到下次再问。释义和词本身一样时等于没有释义，换成空字符串；空字符串表示账号对这个词没有释义，调用方据此记一条否定缓存。
inline std::optional<std::vector<std::string>> account_gloss_values(
    const std::vector<std::string> &words, std::vector<std::string> values) {
  if (values.size() != words.size())
    return std::nullopt;
  for (size_t index = 0; index < values.size(); ++index) {
    const auto &value = values[index];
    if (value.size() > account_gloss_maximum_value_bytes ||
        std::any_of(value.begin(), value.end(), [](unsigned char ch) {
          return ch == '\n' || ch == '\r' || (ch < 0x20 && ch != '\t') || ch == 0x7f;
        }))
      return std::nullopt;
    if (value == words[index])
      values[index].clear();
  }
  return values;
}
} // namespace msime::windows
