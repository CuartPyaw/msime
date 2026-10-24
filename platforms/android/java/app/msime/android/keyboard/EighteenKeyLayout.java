package app.msime.android;

import java.util.List;

/** Android 双拼 18 键；字母组复用已有的键面值类型，解码由共享 Rust 负责。 */
public final class EighteenKeyLayout {
    private static final List<List<FourteenKeyLayout.Key>> ROWS = List.of(
        List.of(key("q"), key("we"), key("rt"), key("y"), key("u"), key("io"), key("p")),
        List.of(key("a"), key("sd"), key("fg"), key("h"), key("jk"), key("l")),
        List.of(key("z"), key("xc"), key("v"), key("bn"), key("m")));

    public static final String LAYER_TITLE = "18键";

    private EighteenKeyLayout() {}

    private static FourteenKeyLayout.Key key(String letters) {
        return new FourteenKeyLayout.Key(letters);
    }

    /** 附件指定的三行 7＋6＋5 分组；L 同时承载微软双拼的分号。 */
    public static List<List<FourteenKeyLayout.Key>> rows() { return ROWS; }

    /** 独立记录 18 键热力图，不与 26 键或 14 键混合。 */
    public static String keyId(FourteenKeyLayout.Key key) {
        return "Eighteen" + FourteenKeyLayout.face(key);
    }

    /** 长按组内字母可直接输入，L 还提供分号。 */
    public static String holdLetters(FourteenKeyLayout.Key key) {
        return "l".equals(key.letters()) ? "l;" : FourteenKeyLayout.holdLetters(key);
    }
}
