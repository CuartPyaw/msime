package app.msime.android;

/**
 * 引擎不收的键，宿主自己上屏之前该不该先把组合结束掉。
 *
 * <p>Android draws the preedit as a real composing region, so committing a declined mark straight into it replaces the pinyin rather than following it: "nihao" then "@" left only "@". Apple's `handleSymbol` resolves the same case with finish_composition — the leading candidate — so the mark lands after 你好.
 *
 * <p>标点和数字走这条路。数字行（#6022）的数字经 `type()` 交给 Engine：当前页有对应候选的 1–9 由 runtime 选词，翻页外的 1–9 被 runtime 吞掉（handled、什么也不变）；剩下被拒的是 `0` 和没有候选时的 1–9。它们和被拒的标点一样，直接上屏会把组字区里的拼音换成这个数字（nihao 再按 0 只剩 0，候选栏还挂着 nihao 的候选），所以也先按首选结束组合，再上屏数字。
 */
public final class DeclinedKeyPolicy {
    private DeclinedKeyPolicy() {}

    /**
     * Whether a declined key must finish the open composition before the host commits it.
     *
     * @param punctuation 这个键是不是按标点交给 Engine 的
     * @param key 宿主要上屏的字符
     * @param composing Engine 处理完这个键之后是否还在组字
     */
    public static boolean finishesComposition(boolean punctuation, char key, boolean composing) {
        return composing && (punctuation || (key >= '0' && key <= '9'));
    }
}
