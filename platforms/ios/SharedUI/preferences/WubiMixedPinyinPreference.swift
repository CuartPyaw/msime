import Foundation

// Answers a wubi code the table cannot spell with quanpin candidates for the same letters. A code
// the table does answer keeps its own candidates, so wubi as typed is unchanged.
//
// The choice lives in the shared preference document `wubi_mixed_pinyin`, which the settings page
// writes and the engine reads through host-api; `Preferences::for_edition` seeds a fresh wubi
// edition with it on, so no App Group mirror is needed.
@MainActor
enum WubiMixedPinyinPreference {
  static let documentKey = "wubi_mixed_pinyin"
  /// 文档里没有这个键时的值：随版本，full 是关、五笔版是开。
  static var documentDefault: Bool { MSIMEAppEdition.wubiMixedPinyinDefault }

  /// 文档里的值；缺这个键时取 `documentDefault`。
  static func isEnabled(in document: [String: Any]?) -> Bool {
    document?[documentKey] as? Bool ?? documentDefault
  }
}
