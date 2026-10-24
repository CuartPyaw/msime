import app.msime.android.EighteenKeyLayout;
import app.msime.android.FourteenKeyLayout;
import app.msime.android.KeyPressIds;
import app.msime.android.KeyboardActionRow;
import app.msime.android.KeyboardLayout;
import app.msime.android.NineKeyPanelPolicy;
import java.util.HashSet;
import java.util.List;
import java.util.Set;

/** 18 键双拼的附件分组、可达字母和宿主呈现边界。 */
public final class EighteenKeyLayoutSmoke {
    private static void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }

    public static void main(String[] args) {
        List<List<FourteenKeyLayout.Key>> rows = EighteenKeyLayout.rows();
        check(rows.size() == 3 && rows.get(0).size() == 7 && rows.get(1).size() == 6
            && rows.get(2).size() == 5, "附件的三行是 7＋6＋5 键");
        List<String> expected = List.of("q", "we", "rt", "y", "u", "io", "p", "a", "sd",
            "fg", "h", "jk", "l", "z", "xc", "v", "bn", "m");
        Set<Character> letters = new HashSet<>();
        int index = 0;
        for (List<FourteenKeyLayout.Key> row : rows) {
            for (FourteenKeyLayout.Key key : row) {
                check(key.letters().equals(expected.get(index++)), "分组顺序与附件一致");
                for (char letter : key.letters().toCharArray())
                    check(letters.add(letter), "每个字母只能出现一次");
                check(KeyPressIds.isKnown(EighteenKeyLayout.keyId(key)), "统计存储接受新键位");
                check(EighteenKeyLayout.keyId(key).startsWith("Eighteen"), "统计独立于其他布局");
            }
        }
        check(letters.size() == 26, "26 个字母完整可达");
        FourteenKeyLayout.Key l = rows.get(1).get(5);
        check(FourteenKeyLayout.face(l, true).equals("L;")
            && FourteenKeyLayout.accessibilityLabel(l, true).contains("分号"), "分号入口可见可读");
        check(EighteenKeyLayout.holdLetters(l).equals("l;"), "长按可以直接输入 L 或分号");
        int layout = KeyboardLayout.EIGHTEEN_KEY_LAYOUT;
        check(KeyboardLayout.drawsKeyGrid(layout), "拼音选择与候选展开使用网格");
        check(KeyboardLayout.drawsNumberPad(KeyboardLayout.Layer.SYMBOLS, layout, true, false),
            "数字层往返保持原布局");
        check(KeyboardActionRow.layerTitle(layout, true).equals("18键"), "数字层返回键标明布局");
        check(!KeyboardLayout.carriesLetterCase(layout)
            && !KeyboardLayout.drawsNumberRow(true, KeyboardLayout.Layer.LETTERS, layout, 760),
            "组码没有 Shift 和额外数字行");
        check(NineKeyPanelPolicy.threeColumn(layout, true, false, true), "组字可展开三栏候选");
        check(KeyboardLayout.resolveTouchLayout(false, false, false, true, 1, "twenty_six_key") == layout,
            "实际 18 键网格决定键面");
        check(KeyboardLayout.resolveTouchLayout(false, false, false, false, 1, "twenty_six_key")
            == KeyboardLayout.STANDARD_TOUCH_LAYOUT, "本地模式与直输回到确定字母");
        System.out.println("Android eighteen-key layout: rows, letters, semicolon, ids and gates passed");
    }
}
