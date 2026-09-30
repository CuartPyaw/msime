/**
 * What the host does differently while the Engine composes Korean Hangul (scheme 4).
 *
 * Korean is a syllable automaton with no candidates. The view's `editing_text` holds only the key letters of the open syllable ("sud" for 녕), so it says whether a syllable is open but is never what the user should see: the syllable itself is `preedit`. A key the automaton does not take (Space, Return, a digit) commits the syllable and is left unhandled, and the host then does that key's normal work after the commit.
 */
export const KOREAN_SCHEME: number = 4;

export class KoreanCompositionPolicy {
  /** Whether letters are going to the Hangul automaton: the Korean scheme, with neither English nor a local utility mode taking the keys. */
  static active(scheme: number, dedicatedEnglish: boolean, localMode: string): boolean {
    return scheme === KOREAN_SCHEME && !dedicatedEnglish && localMode === "none";
  }

  /** Whether the keyboard's own scheme choice is Korean, before the Engine has reported anything. */
  static selected(schemeName: string, english: boolean, localMode: string): boolean {
    return schemeName === "korean" && !english && localMode === "none";
  }

  /** The composition to draw: the Hangul for Korean, the spelling for every other scheme. */
  static reading(korean: boolean, editing: string, preedit: string): string {
    return korean && editing.length > 0 ? preedit : editing;
  }

  /**
   * Whether a touch key the automaton leaves unhandled is typed by the keyboard after the commit: a space or a digit, the keys a phone has no application behind to type them.
   */
  static typesAfterCommit(character: number): boolean {
    return character === 0x20 || (character >= 0x30 && character <= 0x39);
  }
}
