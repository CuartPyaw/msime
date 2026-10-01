import app.msime.android.CommunityRequest;
import app.msime.android.CommunityRequest.Kind;

public final class CommunityRequestSmoke {
    public static void main(String[] args) {
        check(CommunityRequest.kinds().size() == 3, "skins, dictionaries and replies");
        check("皮肤".equals(Kind.SKIN.title()) && !Kind.SKIN.searchHint().isEmpty(),
            "every kind is titled and says what its search covers");

        check("/v1/community/skins?offset=0&q=".equals(
            CommunityRequest.path(Kind.SKIN, "", "", 0)), "the skin catalogue has its own endpoint");
        check("/v1/community/resources?kind=dictionary&scope=mine&q=&offset=40".equals(
            CommunityRequest.path(Kind.DICTIONARY, "mine", "", 40)),
            "dictionaries and replies share the resource endpoint, separated by kind");
        check(CommunityRequest.path(Kind.REPLY, "", "", -5).endsWith("offset=0"),
            "a negative offset is the first page, not a server error");
        check(CommunityRequest.path(Kind.SKIN, "", "  海盐  ", 0).endsWith("q=%E6%B5%B7%E7%9B%90"),
            "a search term is trimmed and percent-encoded");

        // Form encoding would send this as two spaces: `+` is a literal here, not a space.
        check("C%2B%2B".equals(CommunityRequest.encode("C++")), "a plus stays a plus");
        check("a%20b".equals(CommunityRequest.encode("a b")), "a space is %20, not +");
        check("-_.~".equals(CommunityRequest.encode("-_.~")), "unreserved characters pass through");
        check(CommunityRequest.encode("").isEmpty() && CommunityRequest.encode(null).isEmpty(),
            "nothing to encode encodes to nothing");

        check("最多发布 50 款皮肤，请先下架部分作品。".equals(
            CommunityRequest.message("skin_publish_limit", 409)),
            "a named code answers before the status");
        check("登录已过期，请重新登录。".equals(CommunityRequest.message("", 401)),
            "an unnamed 401 falls back to the status");
        check("连不上社区，请检查网络后重试。".equals(CommunityRequest.message(null, 0)),
            "a request that never reached the server says so");
        check(!CommunityRequest.message("something new", 599).isEmpty(),
            "an unknown failure still says something");

        check("skins".equals(CommunityRequest.reportKind(Kind.SKIN))
            && "dictionaries".equals(CommunityRequest.reportKind(Kind.DICTIONARY))
            && "replies".equals(CommunityRequest.reportKind(Kind.REPLY)),
            "reports name each kind the way the server does");
        check(CommunityRequest.REPORT_REASONS.equals(java.util.List.of(
            "侵权/抄袭", "色情低俗", "违法违规", "垃圾广告", "恶意插件", "其他")),
            "the fixed report reasons, in order");
        check(CommunityRequest.validReport("其他", null) && CommunityRequest.validReport("其他", "😀".repeat(1000)),
            "a detail of up to 1000 characters is accepted");
        check(!CommunityRequest.validReport("其他", "a".repeat(1001)), "a longer detail is refused");
        check(!CommunityRequest.validReport("不喜欢", ""), "only the fixed reasons are sent");
        check("内容包含不允许发布的词语，请修改后再提交".equals(CommunityRequest.message("blocked_content", 422)),
            "a refused word asks for an edit");
        check("审核服务暂时不可用，请稍后重试".equals(CommunityRequest.message("screening_unavailable", 503)),
            "screening outage asks for a retry");
        check(CommunityRequest.message("account_banned", 403).contains("封禁"), "a ban is named");
        System.out.println("Android community requests: paths, query encoding, reports and failures passed");
    }

    private static void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }
}
