#pragma once
#include <cstdint>
#include <optional>

namespace msime::windows {
// Cross-application CN/EN authority.
//
// With input.ime_mode_scope set to "global" the user expects one Chinese or
// English state to follow them between applications. Each TSF client keeps its
// own mode, so the Server has to be the authority: when a different client
// takes focus it reports whatever mode it happens to hold, and the Server
// pushes its own back. Without this the choice was per-application only, which
// is the behaviour the "按应用记忆" option already describes.
//
// 应用例外（`app_input_mode_rules`）排在作用域前面，和 macOS 的 -englishMode 一样：焦点从别的应用进入一个有规则的应用时推规则里的模式，两种作用域下都是；用户在这次停留里手动切换后规则让位，离开再回来重新生效。规则是例外，不改全局状态。
struct ModeAuthorityState {
  bool chinese = true;
  bool seeded = false;
  // The focused client's session id, so a change of client is distinguishable
  // from the same client changing its own mode.
  uint64_t session = 0;
  // 当前这次停留所在应用的进程号。同一应用里换焦点（两个窗口、一个窗口里的几个输入框）不算新的停留。
  uint32_t app = 0;
  // 本次停留里用户已经手动切换过，规则让位到离开这个应用为止。
  bool rule_yielded = false;
  // 当前会话上一次报告的模式，用来认出同一会话里的模式变化。
  std::optional<bool> reported;
  // 推给当前会话、还没看到回报的模式。回报等于它时是推送落地，不是用户切换。
  std::optional<bool> pushed;
};
struct ModeAuthorityDecision {
  // Send a mode switch to the focused client.
  bool push = false;
  bool push_chinese = true;
  // 同一会话里模式变了且不是 Server 自己推的：用户切换了中英文（快捷键、工具栏、托盘或语言栏），中英文切换提示据此出现。
  bool user_changed = false;
  // The authority after this observation.
  ModeAuthorityState next;
};
// Decide what to do with one observation of the focused client's mode.
//
// `global` is the configured scope, `focused` whether a client is focused at
// all, and `reported` the mode that client says it is in. `app` 是焦点客户端的进程号，`rule` 是这个进程的应用例外（true 为中文），没有规则时为空。
inline ModeAuthorityDecision
mode_authority_step(const ModeAuthorityState &state, bool global, bool focused,
                    uint64_t session, bool reported, uint32_t app = 0,
                    std::optional<bool> rule = std::nullopt) {
  ModeAuthorityDecision decision;
  decision.next = state;
  if (!focused)
    return decision; // Nothing focused: keep the authority, push nothing.
  auto push = [&](bool chinese) {
    decision.next.pushed.reset();
    if (reported == chinese)
      return;
    decision.push = true;
    decision.push_chinese = chinese;
    decision.next.pushed = chinese;
  };
  if (session != state.session) {
    // 换了客户端。先记下它报告的模式；进程号变了就是一次新的停留，上次的手动让位作废。
    decision.next.session = session;
    decision.next.reported = reported;
    decision.next.pushed.reset();
    if (app != state.app) {
      decision.next.app = app;
      decision.next.rule_yielded = false;
    }
    if (rule && !decision.next.rule_yielded) {
      // 有规则的应用从规则里的模式开始，全局状态不动；按应用记忆时这就是它这次的模式。
      push(*rule);
      if (!global) {
        decision.next.chinese = *rule;
        decision.next.seeded = true;
      }
      return decision;
    }
    if (!global) {
      // Per-application memory. Track the session so switching the option on
      // later does not immediately treat the current client as a new one, but
      // never push: each application keeps its own mode, as the option says.
      decision.next.chinese = reported;
      decision.next.seeded = true;
      return decision;
    }
    if (!state.seeded) {
      // First observation seeds the authority rather than fighting the client.
      decision.next.chinese = reported;
      decision.next.seeded = true;
      return decision;
    }
    // A different client took focus. It reports its own mode; the authority
    // wins, and only differences are pushed so an already-correct client is
    // left alone.
    push(state.chinese);
    return decision;
  }
  if (state.reported == reported)
    return decision; // 同一会话，模式没变。
  decision.next.reported = reported;
  if (state.pushed == reported) {
    // Server 推的模式落地了，不是用户的选择。
    decision.next.pushed.reset();
    return decision;
  }
  decision.next.pushed.reset();
  // Same client, new mode: the user changed it deliberately, so it becomes the
  // authority and travels to the next application. 有规则的应用里手动切换，规则在这次停留里让位。
  decision.user_changed = state.reported.has_value();
  if (rule)
    decision.next.rule_yielded = true;
  decision.next.chinese = reported;
  decision.next.seeded = true;
  return decision;
}
} // namespace msime::windows
