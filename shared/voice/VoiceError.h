#pragma once
#include <stdexcept>

namespace msime::voice {
// What the shared recognizers and provider adapters throw for a request they cannot serve: a missing runtime or model, a malformed provider answer, a cancelled request. what() is a terse English diagnostic for logs; it never carries a token, audio or a response body.
class VoiceError : public std::runtime_error {
public:
  using std::runtime_error::runtime_error;
};
} // namespace msime::voice
