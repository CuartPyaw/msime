package app.msime.android;

import java.util.concurrent.atomic.AtomicInteger;

public final class AccountSessionRoutingSmoke {
    private static final String PACKAGE = "app.msime.android";
    private static final String TOKEN = "e".repeat(64);

    public static void main(String[] args) throws Exception {
        check(AccountSessionRoutingPolicy.ownsSession(PACKAGE, PACKAGE), "the main process owns the session");
        check(!AccountSessionRoutingPolicy.ownsSession(PACKAGE + ":ime", PACKAGE), "the keyboard process does not own the session");
        check(!AccountSessionRoutingPolicy.ownsSession(null, PACKAGE), "an unknown process asks the owner");
        check(!AccountSessionRoutingPolicy.ownsSession(PACKAGE, ""), "a missing package name asks the owner");
        check(!AccountSessionRoutingPolicy.ownsSession(PACKAGE, null), "a null package name asks the owner");
        check(AccountSessionRoutingPolicy.authority(PACKAGE).equals(PACKAGE + ".account-session"), "the authority matches the manifests");
        boolean rejected = false;
        try {
            AccountSessionRoutingPolicy.authority("");
        } catch (IllegalArgumentException expected) {
            rejected = true;
        }
        check(rejected, "an empty package name has no authority");

        check(AccountSessionRoutingPolicy.accepts("access_token", 10123, 10123), "our own uid gets the token");
        check(!AccountSessionRoutingPolicy.accepts("access_token", 10124, 10123), "another uid is refused");
        check(!AccountSessionRoutingPolicy.accepts("refresh_token", 10123, 10123), "no other method is answered");
        check(!AccountSessionRoutingPolicy.accepts(null, 10123, 10123), "a missing method is refused");

        // A process that does not own the session neither reads nor writes the store and never calls the service; the token comes from the owner alone.
        AtomicInteger storeTouches = new AtomicInteger();
        AtomicInteger requests = new AtomicInteger();
        BackendAccount.SessionStore store = new BackendAccount.SessionStore() {
            @Override public String load() { storeTouches.incrementAndGet(); return null; }
            @Override public void save(String value) { storeTouches.incrementAndGet(); }
            @Override public void clear() { storeTouches.incrementAndGet(); }
        };
        BackendAccount.Requester requester = (method, path, body, token) -> {
            requests.incrementAndGet();
            throw new IllegalStateException("no network in this smoke");
        };
        check(new BackendAccount(store, requester, () -> TOKEN).accessToken().equals(TOKEN), "the owner's token is used");
        check(new BackendAccount(store, requester, () -> TOKEN).signedIn(), "an owner token means signed in");
        check(new BackendAccount(store, requester, () -> "").accessToken().isEmpty(), "a signed-out owner means signed out");
        check(new BackendAccount(store, requester, () -> null).accessToken().isEmpty(), "a missing reply means signed out");
        check(new BackendAccount(store, requester, () -> "not-a-token").accessToken().isEmpty(), "a malformed reply is not used as a token");
        check(new BackendAccount(store, requester, () -> {
            throw new IllegalArgumentException("Unknown authority");
        }).accessToken().isEmpty(), "an unreachable owner means signed out rather than a local refresh");
        check(storeTouches.get() == 0, "a non-owning process never touches the session store");
        check(requests.get() == 0, "a non-owning process never refreshes");
        System.out.println("Android account session routing: single refreshing process passed");
    }

    private static void check(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }
}
