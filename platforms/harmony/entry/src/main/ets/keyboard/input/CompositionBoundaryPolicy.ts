export enum CompositionBoundaryAction {
  NONE = "none",
  COMMIT_RAW = "commit-raw",
  FINISH_COMPOSITION = "finish-composition",
}

export enum CompositionBoundary {
  MODE_SWITCH = "mode-switch",
  DEACTIVATE = "deactivate",
}

/** How a mode or panel boundary preserves text already entered into the current scheme. */
export class CompositionBoundaryPolicy {
  static action(
    composing: boolean,
    japanese: boolean,
    boundary: CompositionBoundary,
    korean: boolean = false,
  ): CompositionBoundaryAction {
    if (!composing) {
      return CompositionBoundaryAction.NONE;
    }
    // A Korean syllable is text the user already wrote, like Japanese kana; its key letters are not something to commit instead.
    if (boundary === CompositionBoundary.DEACTIVATE || japanese || korean) {
      return CompositionBoundaryAction.FINISH_COMPOSITION;
    }
    return CompositionBoundaryAction.COMMIT_RAW;
  }
}
