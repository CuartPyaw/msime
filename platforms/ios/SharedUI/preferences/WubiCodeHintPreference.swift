import Foundation

/// Controls the remaining-code hint beside Wubi candidates.
///
/// The choice lives in the shared preference document (`wubi_code_hint`), which the settings page
/// writes and the keyboard reads through the session, so nothing is kept in the App Group. A
/// bounded pure helper keeps fallback or unrelated candidate codes from suggesting invalid keys.
@MainActor
enum WubiCodeHintPreference {
  static let documentKey = "wubi_code_hint"
  /// 文档里没有这个键时的值：接入共享偏好之前它只有一种实际行为，就是开。
  nonisolated static let documentDefault = true
  nonisolated static let maxCodeLength = 64

  /// 文档里的值；缺这个键时取 `documentDefault`。
  static func isEnabled(in document: [String: Any]?) -> Bool {
    document?[documentKey] as? Bool ?? documentDefault
  }

  nonisolated static func hint(
    code: String, typed: String, answeredByPinyinFallback: Bool
  ) -> String {
    guard !answeredByPinyinFallback, !typed.isEmpty,
          code.count <= maxCodeLength, typed.count <= maxCodeLength,
          code.count > typed.count, code.hasPrefix(typed) else { return "" }
    return String(code.dropFirst(typed.count))
  }
}
