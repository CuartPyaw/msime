import Foundation

/// The Dubeolsik (두벌식) faces of the 26 letter keys while the Korean scheme is active.
///
/// The Engine composes syllables from ASCII letters, so the keyboard only relabels the keys and sends the letter printed under each jamo. Shift gives the five tense consonants and the two extra vowels on Q W E R T O P and sends those letters in upper case; every other key keeps its jamo under Shift and sends its lower-case letter.
enum DubeolsikKeyLayout {
  private static let plainJamo: [String: String] = [
    "q": "ㅂ", "w": "ㅈ", "e": "ㄷ", "r": "ㄱ", "t": "ㅅ", "y": "ㅛ", "u": "ㅕ", "i": "ㅑ", "o": "ㅐ", "p": "ㅔ",
    "a": "ㅁ", "s": "ㄴ", "d": "ㅇ", "f": "ㄹ", "g": "ㅎ", "h": "ㅗ", "j": "ㅓ", "k": "ㅏ", "l": "ㅣ",
    "z": "ㅋ", "x": "ㅌ", "c": "ㅊ", "v": "ㅍ", "b": "ㅠ", "n": "ㅜ", "m": "ㅡ",
  ]
  private static let shiftedJamo: [String: String] = [
    "q": "ㅃ", "w": "ㅉ", "e": "ㄸ", "r": "ㄲ", "t": "ㅆ", "o": "ㅒ", "p": "ㅖ",
  ]

  /// Whether Shift changes what the key types.
  static func hasShiftedJamo(_ letter: String) -> Bool {
    shiftedJamo[letter.lowercased()] != nil
  }

  /// The jamo drawn on the key for `letter`, or nil for a key that is not one of the 26 letters.
  static func keycap(for letter: String, shifted: Bool) -> String? {
    let key = letter.lowercased()
    if shifted, let jamo = shiftedJamo[key] { return jamo }
    return plainJamo[key]
  }

  /// The ASCII letter the key sends to the Engine: upper case only where Shift gives a different jamo, so the case the Engine sees is always the case that decides the jamo. Nil for a key that is not one of the 26 letters.
  static func keyInput(for letter: String, shifted: Bool) -> String? {
    let key = letter.lowercased()
    guard plainJamo[key] != nil else { return nil }
    return shifted && shiftedJamo[key] != nil ? key.uppercased() : key
  }
}
