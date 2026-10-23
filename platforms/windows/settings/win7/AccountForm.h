#pragma once

#include "ControlText.h"
#include "PreferencesStore.h"
#include "SettingsWindow.h"
#include "../AccountPageModel.h"
#include <commctrl.h>
#include <functional>
#include <map>
#include <optional>

namespace msime::settings::win7 {

// 账号页只编排控件和稳定的 account action JSON；目录、会话迁移和网络全部由工作线程的 C ABI 调用完成。
class AccountForm {
public:
  struct Command {
    bool handled = false;
    std::optional<Json> request;
  };

  int create(const std::function<HWND(const wchar_t*, const wchar_t*, DWORD, int, int, int)>& row) {
    int top = 0;
    const auto add = [&](int id, const wchar_t* type, const wchar_t* title, DWORD style, int height = 28) {
      const auto control = row(type, title, style, id, top, height);
      if (id >= account_status) controls_[id] = control;
      top += (std::wstring_view(type) == WC_COMBOBOXW ? 28 : height) + 12;
      return control;
    };
    add(account_status, L"STATIC", L"正在读取账号状态…", SS_LEFT, 64);
    add(account_refresh, L"BUTTON", L"刷新登录方式", WS_TABSTOP | BS_PUSHBUTTON, 32);
    add(account_email, L"BUTTON", L"邮箱验证码登录", WS_TABSTOP | BS_PUSHBUTTON, 32);
    add(account_phone, L"BUTTON", L"手机号验证码登录", WS_TABSTOP | BS_PUSHBUTTON, 32);
    add(account_target, L"EDIT", L"", WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL);
    SendMessageW(get(account_target), EM_LIMITTEXT, 320, 0);
    add(account_request_code, L"BUTTON", L"获取验证码", WS_TABSTOP | BS_PUSHBUTTON, 32);
    add(account_code, L"EDIT", L"", WS_TABSTOP | WS_BORDER | ES_NUMBER | ES_AUTOHSCROLL);
    SendMessageW(get(account_code), EM_LIMITTEXT, 6, 0);
    add(account_login, L"BUTTON", L"登录", WS_TABSTOP | BS_PUSHBUTTON, 32);
    add(account_google, L"BUTTON", L"使用 Google 登录", WS_TABSTOP | BS_PUSHBUTTON, 32);
    add(account_logout, L"BUTTON", L"退出登录", WS_TABSTOP | BS_PUSHBUTTON, 32);
    add(account_logout_all, L"BUTTON", L"退出所有设备", WS_TABSTOP | BS_PUSHBUTTON, 32);
    add(account_notice, L"STATIC", L"", SS_LEFT, 100);
    return top;
  }

  void refresh() {
    updating_ = true;
    SetWindowTextW(get(account_status), signed_in_ ? (L"已登录：" + user_name_) .c_str() : L"当前未登录");
    SetWindowTextW(get(account_notice), notice_.c_str());
    SetWindowTextW(get(account_target), target_.c_str());
    SetWindowTextW(get(account_code), code_.c_str());
    updating_ = false;
    update();
  }

  void enable(bool enabled) {
    enabled_ = enabled;
    update();
  }

  Command command(WPARAM event) {
    const int id = LOWORD(event), notification = HIWORD(event);
    if (id < account_status || id > account_notice) return {};
    Command result{true, std::nullopt};
    if (!enabled_ || updating_) return result;
    if (id == account_email && notification == BN_CLICKED) { channel_ = L"email"; notice_.clear(); update(); return result; }
    if (id == account_phone && notification == BN_CLICKED) { channel_ = L"phone"; notice_.clear(); update(); return result; }
    if (id == account_target && notification == EN_CHANGE) {
      if (const auto value = control_text::read(get(account_target), 320)) target_ = control_text::wide(*value);
      return result;
    }
    if (id == account_code && notification == EN_CHANGE) {
      if (const auto value = control_text::read(get(account_code), 16)) code_ = account::code_digits(control_text::wide(*value));
      return result;
    }
    if (notification != BN_CLICKED || !IsWindowEnabled(get(id))) return result;
    if (id == account_refresh) result.request = Json{{"operation", "providers"}};
    else if (id == account_request_code) {
      const auto target = account::trim_login_target(target_);
      if (channel_.empty() || target.empty()) { notice_ = L"请先选择邮箱或手机号并填写登录目标。"; refresh(); return result; }
      result.request = Json{{"operation", "request_code"}, {"provider", channel_}, {"target", narrow(target)}};
    } else if (id == account_login) {
      if (challenge_.empty() || !account::six_digits(code_)) { notice_ = L"请先获取验证码并填写 6 位数字验证码。"; refresh(); return result; }
      result.request = Json{{"operation", "login"}, {"challenge_id", challenge_}, {"code", narrow(code_)}};
    } else if (id == account_google) result.request = Json{{"operation", "google_login"}};
    else if (id == account_logout) result.request = Json{{"operation", "logout"}, {"all", false}};
    else if (id == account_logout_all) result.request = Json{{"operation", "logout"}, {"all", true}};
    return result;
  }

  void completed(const Json& request, const Json& response) {
    notice_.clear();
    if (!response.is_object() || !response.value("ok", false)) {
      notice_ = account::error_message(control_text::wide(response.value("error", std::string{"account_unavailable"})));
      refresh();
      return;
    }
    const auto value = response.value("value", Json{});
    const auto operation = request.value("operation", std::string{});
    if (operation == "providers") {
      provider_email_ = value.value("email", false);
      provider_phone_ = value.value("phone", false);
      provider_google_ = value.value("google", false);
      notice_ = L"登录方式已刷新。";
    } else if (operation == "request_code") {
      challenge_ = value.value("challengeId", std::string{});
      notice_ = challenge_.empty() ? L"无法取得验证码，请稍后重试。" : L"验证码已发送，请查收。";
    } else if (operation == "status" || operation == "login" || operation == "google_login") {
      const auto user = value.value("user", Json{});
      signed_in_ = user.is_object() && user.contains("id") && user.at("id").is_string();
      user_name_ = signed_in_ ? control_text::wide(user.value("displayName", std::string{})) : std::wstring{};
      if (operation != "status") notice_ = signed_in_ ? L"登录成功。" : L"当前未登录。";
      if (operation == "login" || operation == "google_login") { challenge_.clear(); code_.clear(); }
    } else if (operation == "logout") {
      signed_in_ = false; user_name_.clear(); challenge_.clear(); code_.clear(); notice_ = L"已退出登录。";
    }
    refresh();
  }

  Json status_request() const { return Json{{"operation", "status"}}; }

private:
  HWND get(int id) const { const auto it = controls_.find(id); return it == controls_.end() ? nullptr : it->second; }
  static std::string narrow(std::wstring_view value) {
    const auto utf8 = control_text::utf8(value);
    return utf8.value_or(std::string{});
  }
  void update() {
    if (!enabled_) {
      for (const auto& entry : controls_) EnableWindow(entry.second, FALSE);
      return;
    }
    EnableWindow(get(account_refresh), TRUE);
    EnableWindow(get(account_email), !signed_in_ && provider_email_);
    EnableWindow(get(account_phone), !signed_in_ && provider_phone_);
    EnableWindow(get(account_target), !signed_in_ && !channel_.empty());
    EnableWindow(get(account_request_code), !signed_in_ && !channel_.empty() && !target_.empty());
    EnableWindow(get(account_code), !signed_in_ && !challenge_.empty());
    EnableWindow(get(account_login), !signed_in_ && !challenge_.empty());
    EnableWindow(get(account_google), !signed_in_ && provider_google_);
    EnableWindow(get(account_logout), signed_in_);
    EnableWindow(get(account_logout_all), signed_in_);
  }

  std::map<int, HWND> controls_;
  std::wstring channel_, target_, code_, user_name_, notice_;
  std::string challenge_;
  bool enabled_ = false, updating_ = false, signed_in_ = false;
  bool provider_email_ = false, provider_phone_ = false, provider_google_ = false;
};

} // namespace msime::settings::win7
