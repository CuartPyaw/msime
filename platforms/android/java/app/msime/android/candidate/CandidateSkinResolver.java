package app.msime.android;

import java.io.File;
import java.util.HashMap;
import java.util.Map;
import org.json.JSONException;
import org.json.JSONObject;

/** 在偏好工作线程读取候选包，输入线程只取与当前请求相符的配色。 */
public final class CandidateSkinResolver {
    private record Resolved(String request, String id, Map<String, String> colours) {}
    private record Modes(String directory, Resolved light, Resolved dark) {}
    private volatile Modes modes;

    /** 与偏好重读共用工作线程；每次重新校验包，资源更新不要求同时修改偏好或清单。 */
    public String read(String directory) {
        String response = NativeClient.loadPreferences(directory);
        Modes next = null;
        try {
            JSONObject envelope = new JSONObject(response);
            JSONObject value = JsonPolicy.strictTrue(envelope.opt("ok"))
                ? envelope.optJSONObject("value") : null;
            JSONObject preferences = value == null ? null : value.optJSONObject("preferences");
            if (preferences != null && new File(directory).isAbsolute()
                    && "custom".equals(preferences.optString("global_theme"))) {
                String root = new File(directory, "skins").getAbsolutePath();
                next = new Modes(directory, resolve(preferences, root, false),
                    resolve(preferences, root, true));
            }
        } catch (JSONException | RuntimeException | LinkageError ignored) {
            // 候选包失效只恢复现有配色，不阻止偏好和引擎会话更新。
        }
        modes = next;
        return response;
    }

    private static String request(JSONObject preferences, boolean dark) throws JSONException {
        return KeyboardSkin.themeRequest(preferences.optString("global_theme", "system"),
            preferences.optJSONObject("custom_theme"), dark);
    }

    private static Resolved resolve(JSONObject preferences, String root, boolean dark)
            throws JSONException {
        String key = request(preferences, dark);
        JSONObject query = new JSONObject(key).put("skins_directory", root);
        JSONObject envelope = new JSONObject(NativeClient.resolveTheme(query.toString()));
        JSONObject value = JsonPolicy.strictTrue(envelope.opt("ok"))
            ? envelope.optJSONObject("value") : null;
        if (value == null) return null;
        String id = JsonPolicy.strictString(value.opt("candidate_skin"));
        JSONObject palette = value.optJSONObject("candidate");
        if (id == null || id.isEmpty() || palette == null) return null;
        Map<String, String> colours = new HashMap<>();
        for (String name : new String[] {"surface", "border", "text", "number", "secondary",
                "accent", "selected", "selected_text", "selected_number", "hover"}) {
            String colour = JsonPolicy.strictString(palette.opt(name));
            if (colour != null) colours.put(name, colour);
        }
        return new Resolved(key, id, Map.copyOf(colours));
    }

    /** 不触碰文件系统，也不把旧请求的包颜色套到刚切换的主题上。 */
    public CandidateAppearance.Palette paletteFor(String directory, JSONObject preferences,
            boolean dark, CandidateAppearance.Palette fallback) {
        Modes current = modes;
        if (current == null || preferences == null || !current.directory().equals(directory))
            return fallback;
        Resolved selected = dark ? current.dark() : current.light();
        if (selected == null) return fallback;
        try {
            if (selected.request().equals(request(preferences, dark)))
                return CandidateAppearance.fromResolved(fallback, selected.id(), selected.colours());
        } catch (JSONException ignored) {
            // 与键盘相同，非法主题请求不保留另一份主题的颜色。
        }
        return fallback;
    }
}
