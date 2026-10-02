/**
 * The Korean Dubeolsik (2-beolsik) touch layout: which jamo each letter key wears, and which ASCII letter a tap sends.
 *
 * The Engine owns the syllable automaton and reads the letter's case, so the keyboard only labels the keys and picks the case. Shift gives the five double consonants and ㅒ ㅖ on Q W E R T O P, as on the standard layout; every other shifted key types the same jamo as without Shift, so its cap does not change and its letter still goes out in upper case, which the Engine reads as the same jamo.
 */

const JAMO: Map<string, string> = new Map<string, string>([
  ["q", "ㅂ"],
  ["w", "ㅈ"],
  ["e", "ㄷ"],
  ["r", "ㄱ"],
  ["t", "ㅅ"],
  ["y", "ㅛ"],
  ["u", "ㅕ"],
  ["i", "ㅑ"],
  ["o", "ㅐ"],
  ["p", "ㅔ"],
  ["a", "ㅁ"],
  ["s", "ㄴ"],
  ["d", "ㅇ"],
  ["f", "ㄹ"],
  ["g", "ㅎ"],
  ["h", "ㅗ"],
  ["j", "ㅓ"],
  ["k", "ㅏ"],
  ["l", "ㅣ"],
  ["z", "ㅋ"],
  ["x", "ㅌ"],
  ["c", "ㅊ"],
  ["v", "ㅍ"],
  ["b", "ㅠ"],
  ["n", "ㅜ"],
  ["m", "ㅡ"],
]);

const SHIFTED_JAMO: Map<string, string> = new Map<string, string>([
  ["q", "ㅃ"],
  ["w", "ㅉ"],
  ["e", "ㄸ"],
  ["r", "ㄲ"],
  ["t", "ㅆ"],
  ["o", "ㅒ"],
  ["p", "ㅖ"],
]);

export class DubeolsikLayout {
  /** The jamo a letter key wears, or the letter itself for a key the layout does not cover. */
  static jamo(letter: string, shifted: boolean): string {
    const lower: string = letter.toLowerCase();
    if (shifted) {
      const doubled: string | undefined = SHIFTED_JAMO.get(lower);
      if (doubled !== undefined) {
        return doubled;
      }
    }
    return JAMO.get(lower) ?? letter;
  }

  /** The ASCII letter a tap sends: upper case under Shift, which is how the Engine tells ㄲ from ㄱ. */
  static input(letter: string, shifted: boolean): string {
    return shifted ? letter.toUpperCase() : letter.toLowerCase();
  }
}
