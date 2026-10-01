package app.msime.android;

/**
 * Which process owns the signed-in account session, and how the others reach it.
 *
 * <p>The service rotates the refresh token on every refresh and revokes the whole session when a used one is presented again. Two processes that each refresh from their own copy of the session can therefore sign the user out, and the settings app and the isolated `:ime` keyboard process both need a token. So exactly one process refreshes: the app's main process, the one sign-in and sign-out already run in. Every other process asks it for the current access token through a non-exported provider and never reads or writes the session store itself.
 */
public final class AccountSessionRoutingPolicy {
    /** The provider authority, after the package name; the manifests declare `${applicationId}` + this. */
    public static final String AUTHORITY_SUFFIX = ".account-session";
    /** The one `ContentProvider.call` method the provider answers. */
    public static final String METHOD_ACCESS_TOKEN = "access_token";
    /** The reply key carrying the token, or an empty string when the device is not signed in. */
    public static final String KEY_ACCESS_TOKEN = "access_token";

    private AccountSessionRoutingPolicy() {}

    /**
     * Whether this process reads, refreshes and writes the session itself.
     *
     * <p>Only the main process, whose name is the package name. A process whose name cannot be read is treated as a secondary one: asking the provider is correct from any process, including the main one, whereas refreshing from the wrong one is what revokes the session.
     */
    public static boolean ownsSession(String processName, String packageName) {
        return processName != null && packageName != null && !packageName.isEmpty()
            && processName.equals(packageName);
    }

    public static String authority(String packageName) {
        if (packageName == null || packageName.isEmpty()) throw new IllegalArgumentException("No package name");
        return packageName + AUTHORITY_SUFFIX;
    }

    /** The provider answers only our own uid and only the access token method; `exported="false"` is the first line, this is the second. */
    public static boolean accepts(String method, int callingUid, int ownUid) {
        return METHOD_ACCESS_TOKEN.equals(method) && callingUid == ownUid;
    }
}
