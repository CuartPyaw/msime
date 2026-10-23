#pragma once

#include <windows.h>
#include <functional>
#include <memory>
#include <string_view>

namespace msime::windows::win7 {

enum HandwritingControl {
  handwriting_canvas = 100, handwriting_undo, handwriting_clear, handwriting_status,
  handwriting_candidate = 200,
};

class HandwritingWindow final {
public:
  // 测试只在输入提交边界注入；笔划、窗口消息和识别均使用实际实现。
  using SubmitCandidate = std::function<bool(std::wstring_view)>;
  explicit HandwritingWindow(SubmitCandidate submit = {});
  ~HandwritingWindow();
  HandwritingWindow(const HandwritingWindow&) = delete;
  HandwritingWindow& operator=(const HandwritingWindow&) = delete;
  bool create(HINSTANCE instance);
  int run(int show);
  HWND handle() const;

private:
  struct State;
  std::unique_ptr<State> state_;
};

} // namespace msime::windows::win7
