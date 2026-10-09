package app.msime.android;

import android.app.Activity;
import android.app.Instrumentation;
import android.os.Bundle;
import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.Future;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;
import org.json.JSONObject;

/** Regression coverage for refresh-token rotation in the native Android account client. */
public final class BackendAccountRefreshDeviceSmoke extends Instrumentation {
    private static final String ACCESS = "a".repeat(64);
    private static final String REFRESH = "b".repeat(64);
    private static final String NEXT_ACCESS = "c".repeat(64);
    private static final String NEXT_REFRESH = "d".repeat(64);

    private static final class MemoryStore implements BackendAccount.SessionStore {
        private String value;

        MemoryStore(String value) { this.value = value; }

        @Override public synchronized String load() { return value; }
        @Override public synchronized void save(String next) { value = next; }
        @Override public synchronized void clear() { value = null; }
    }

    private static JSONObject tokens(String access, String refresh) throws Exception {
        return new JSONObject().put("access_token", access).put("refresh_token", refresh)
            .put("token_type", "Bearer").put("expires_in", 900);
    }

    private static String expiredSession() throws Exception {
        return new JSONObject().put("tokens", tokens(ACCESS, REFRESH))
            .put("expires_at_unix_ms", System.currentTimeMillis() - 1_000L).toString();
    }

    private static String activeSession() throws Exception {
        return new JSONObject().put("tokens", tokens(ACCESS, REFRESH))
            .put("expires_at_unix_ms", System.currentTimeMillis() + 900_000L).toString();
    }

    private static void check(boolean value, String message) {
        if (!value) throw new AssertionError(message);
    }

    private static void refreshesExpiredSessionAndRotatesCredentials() throws Exception {
        MemoryStore store = new MemoryStore(expiredSession());
        AtomicInteger calls = new AtomicInteger();
        BackendAccount account = new BackendAccount(store, (method, path, body, token) -> {
            calls.incrementAndGet();
            check("POST".equals(method) && "/v1/auth/refresh".equals(path), "refresh endpoint");
            check(REFRESH.equals(body.optString("refresh_token")), "refresh token sent");
            return tokens(NEXT_ACCESS, NEXT_REFRESH);
        });

        check(NEXT_ACCESS.equals(account.accessToken()), "refreshed access token returned");
        JSONObject saved = new JSONObject(store.load());
        JSONObject savedTokens = saved.getJSONObject("tokens");
        check(NEXT_ACCESS.equals(savedTokens.optString("access_token")), "access token rotated");
        check(NEXT_REFRESH.equals(savedTokens.optString("refresh_token")), "refresh token rotated");
        check(!saved.optString("session_id").isEmpty(), "legacy session receives a login identity");
        check(saved.optLong("expires_at_unix_ms") > System.currentTimeMillis(), "expiry persisted");
        check(calls.get() == 1, "one refresh request");
    }

    private static void concurrentCallersShareOneRefresh() throws Exception {
        MemoryStore store = new MemoryStore(expiredSession());
        CountDownLatch started = new CountDownLatch(1);
        CountDownLatch release = new CountDownLatch(1);
        AtomicInteger calls = new AtomicInteger();
        BackendAccount.Requester requester = (method, path, body, token) -> {
            calls.incrementAndGet();
            started.countDown();
            check(release.await(5, TimeUnit.SECONDS), "refresh released");
            return tokens(NEXT_ACCESS, NEXT_REFRESH);
        };
        ExecutorService executor = Executors.newFixedThreadPool(2);
        try {
            Future<String> first = executor.submit(() -> new BackendAccount(store, requester).accessToken());
            check(started.await(5, TimeUnit.SECONDS), "refresh started");
            Future<String> second = executor.submit(() -> new BackendAccount(store, requester).accessToken());
            release.countDown();
            check(NEXT_ACCESS.equals(first.get(5, TimeUnit.SECONDS)), "first caller token");
            check(NEXT_ACCESS.equals(second.get(5, TimeUnit.SECONDS)), "second caller token");
            check(calls.get() == 1, "single-flight refresh");
        } finally {
            executor.shutdownNow();
        }
    }

    private static void refreshesAnUnexpiredRejectedToken() throws Exception {
        MemoryStore store = new MemoryStore(activeSession());
        AtomicInteger calls = new AtomicInteger();
        BackendAccount account = new BackendAccount(store, (method, path, body, token) -> {
            calls.incrementAndGet();
            check("POST".equals(method) && "/v1/auth/refresh".equals(path), "rejected token refresh endpoint");
            check(REFRESH.equals(body.optString("refresh_token")), "rejected token refresh credential");
            return tokens(NEXT_ACCESS, NEXT_REFRESH);
        });

        check(NEXT_ACCESS.equals(account.currentAccessToken(ACCESS)),
            "an unexpired rejected token must be refreshed");
        check(calls.get() == 1, "a rejected token triggers one refresh");
    }

    private static void retriesAccountRequestsAfter401() throws Exception {
        MemoryStore store = new MemoryStore(activeSession());
        AtomicInteger calls = new AtomicInteger();
        BackendAccount account = new BackendAccount(store, (method, path, body, token) -> {
            int call = calls.incrementAndGet();
            if ("/v1/auth/refresh".equals(path)) return tokens(NEXT_ACCESS, NEXT_REFRESH);
            check("/v1/models".equals(path), "account retry keeps the original endpoint");
            if (call == 1) throw new BackendAccount.RequestException(401);
            check(NEXT_ACCESS.equals(token), "account retry uses the refreshed access token");
            return new JSONObject().put("data", new org.json.JSONArray()
                .put(new JSONObject().put("id", "synthetic-model")))
                .put("default_model", "synthetic-model");
        });

        check(account.chatModels().size() == 1, "an account 401 retries after refresh");
        check(calls.get() == 3, "account request, refresh and retry are each issued once");
        check(!new JSONObject(store.load()).optString("session_id").isEmpty(),
            "refresh retains a persisted login identity");
    }

    private static JSONObject models() throws Exception {
        return new JSONObject().put("data", new org.json.JSONArray()
            .put(new JSONObject().put("id", "synthetic-model")))
            .put("default_model", "synthetic-model");
    }

    private static void oldRequestDoesNotRetryAfterNewLogin() throws Exception {
        MemoryStore store = new MemoryStore(activeSession());
        String oldId = new BackendAccount(store, (method, path, body, token) -> models()).currentSession().sessionId();
        BackendAccount replacement = new BackendAccount(store,
            (method, path, body, token) -> tokens(NEXT_ACCESS, NEXT_REFRESH));
        AtomicInteger modelCalls = new AtomicInteger();
        BackendAccount old = new BackendAccount(store, (method, path, body, token) -> {
            if ("/v1/models".equals(path)) {
                if (modelCalls.incrementAndGet() == 1) {
                    replacement.login(new BackendAccount.Challenge("synthetic-challenge", "synthetic-nonce"),
                        "synthetic-credential");
                    throw new BackendAccount.RequestException(401);
                }
                return models();
            }
            throw new AssertionError("unexpected account request");
        });

        boolean cancelled = false;
        try {
            old.chatModels();
        } catch (java.util.concurrent.CancellationException expected) {
            cancelled = true;
        }
        check(cancelled, "a rejected old request must not retry with a new login");
        check(modelCalls.get() == 1, "the new login must not receive the old request");
        check(!oldId.equals(new JSONObject(store.load()).optString("session_id")),
            "new login has a new identity");
    }

    private static void oldRequestDiscardsSuccessAfterNewLogin() throws Exception {
        MemoryStore store = new MemoryStore(activeSession());
        BackendAccount replacement = new BackendAccount(store,
            (method, path, body, token) -> tokens(NEXT_ACCESS, NEXT_REFRESH));
        BackendAccount old = new BackendAccount(store, (method, path, body, token) -> {
            check("/v1/models".equals(path), "models endpoint");
            replacement.login(new BackendAccount.Challenge("synthetic-challenge", "synthetic-nonce"),
                "synthetic-credential");
            return models();
        });

        boolean cancelled = false;
        try {
            old.chatModels();
        } catch (java.util.concurrent.CancellationException expected) {
            cancelled = true;
        }
        check(cancelled, "an old response must not be delivered after a new login");
    }

    private static void secondaryProcessRejectsChangedOwnerLogin() throws Exception {
        AtomicReference<String> login = new AtomicReference<>("synthetic-login-a");
        BackendAccount.TokenSource owner = new BackendAccount.TokenSource() {
            @Override public String accessToken() { return ACCESS; }
            @Override public BackendAccount.SessionCredential session(String rejectedToken) {
                return new BackendAccount.SessionCredential(ACCESS, login.get());
            }
        };
        AtomicInteger calls = new AtomicInteger();
        BackendAccount secondary = new BackendAccount(new MemoryStore(null), (method, path, body, token) -> {
            calls.incrementAndGet();
            login.set("synthetic-login-b");
            throw new BackendAccount.RequestException(401);
        }, owner);

        boolean cancelled = false;
        try {
            secondary.chatModels();
        } catch (java.util.concurrent.CancellationException expected) {
            cancelled = true;
        }
        check(cancelled, "a secondary process must reject an owner login replacement");
        check(calls.get() == 1, "a secondary process must not replay an old request");
    }

    private static void oldRequestDiscardsSuccessAfterSignOut() throws Exception {
        MemoryStore store = new MemoryStore(activeSession());
        BackendAccount account = new BackendAccount(store, (method, path, body, token) -> {
            check("/v1/models".equals(path), "models endpoint");
            new BackendAccount(store, (m, p, b, t) -> models()).signOut();
            return models();
        });

        boolean cancelled = false;
        try {
            account.chatModels();
        } catch (java.util.concurrent.CancellationException expected) {
            cancelled = true;
        }
        check(cancelled, "a signed-out request must not deliver its old response");
    }

    private static List<BackendAccount.ChatMessage> chatRequest() {
        return java.util.List.of(new BackendAccount.ChatMessage("user", "synthetic prompt"));
    }

    private static void oldChatStreamDoesNotRetryAfterNewLogin() throws Exception {
        MemoryStore store = new MemoryStore(activeSession());
        BackendAccount replacement = new BackendAccount(store,
            (method, path, body, token) -> tokens(NEXT_ACCESS, NEXT_REFRESH));
        AtomicInteger calls = new AtomicInteger();
        BackendAccount old = new BackendAccount(store, (method, path, body, token) -> models(), null,
            (body, token, call, listener) -> {
                calls.incrementAndGet();
                replacement.login(new BackendAccount.Challenge("synthetic-challenge", "synthetic-nonce"),
                    "synthetic-credential");
                throw new BackendAccount.RequestException(401);
            });
        boolean cancelled = false;
        try {
            old.chatStream(chatRequest(), "synthetic-model", new BackendAccount.ChatCall(), delta -> {});
        } catch (java.util.concurrent.CancellationException expected) {
            cancelled = true;
        }
        check(cancelled, "an old chat stream must not retry after a new login");
        check(calls.get() == 1, "a new login receives no old chat stream retry");
    }

    private static void oldChatStreamDiscardsDeltaAfterNewLogin() throws Exception {
        MemoryStore store = new MemoryStore(activeSession());
        BackendAccount replacement = new BackendAccount(store,
            (method, path, body, token) -> tokens(NEXT_ACCESS, NEXT_REFRESH));
        AtomicInteger deltas = new AtomicInteger();
        BackendAccount old = new BackendAccount(store, (method, path, body, token) -> models(), null,
            (body, token, call, listener) -> {
                replacement.login(new BackendAccount.Challenge("synthetic-challenge", "synthetic-nonce"),
                    "synthetic-credential");
                listener.onDelta("old delta");
                return "old delta";
            });
        boolean cancelled = false;
        try {
            old.chatStream(chatRequest(), "synthetic-model", new BackendAccount.ChatCall(),
                delta -> deltas.incrementAndGet());
        } catch (java.util.concurrent.CancellationException expected) {
            cancelled = true;
        }
        check(cancelled, "an old chat stream must stop after a new login");
        check(deltas.get() == 0, "an old chat delta must not reach the listener");
    }

    private static void oldChatStreamDoesNotFallbackAfterNewLogin() throws Exception {
        MemoryStore store = new MemoryStore(activeSession());
        BackendAccount replacement = new BackendAccount(store,
            (method, path, body, token) -> tokens(NEXT_ACCESS, NEXT_REFRESH));
        AtomicInteger fallbackCalls = new AtomicInteger();
        BackendAccount old = new BackendAccount(store, (method, path, body, token) -> {
            fallbackCalls.incrementAndGet();
            return models();
        }, null, (body, token, call, listener) -> {
            replacement.login(new BackendAccount.Challenge("synthetic-challenge", "synthetic-nonce"),
                "synthetic-credential");
            throw new BackendAccount.RequestException(400);
        });
        boolean cancelled = false;
        try {
            old.chatStream(chatRequest(), "synthetic-model", new BackendAccount.ChatCall(), delta -> {});
        } catch (java.util.concurrent.CancellationException expected) {
            cancelled = true;
        }
        check(cancelled, "an old chat stream must not fallback under a new login");
        check(fallbackCalls.get() == 0, "a new login receives no old fallback request");
    }

    private static void chatStreamRetriesWithinTheSameLogin() throws Exception {
        MemoryStore store = new MemoryStore(activeSession());
        AtomicInteger calls = new AtomicInteger();
        BackendAccount account = new BackendAccount(store, (method, path, body, token) -> {
            check("/v1/auth/refresh".equals(path), "chat stream refresh endpoint");
            return tokens(NEXT_ACCESS, NEXT_REFRESH);
        }, null, (body, token, call, listener) -> {
            calls.incrementAndGet();
            if (ACCESS.equals(token)) throw new BackendAccount.RequestException(401);
            check(NEXT_ACCESS.equals(token), "chat stream retry uses refreshed token");
            listener.onDelta("synthetic reply");
            return "synthetic reply";
        });
        AtomicInteger deltas = new AtomicInteger();
        String reply = account.chatStream(chatRequest(), "synthetic-model", new BackendAccount.ChatCall(),
            delta -> deltas.incrementAndGet());
        check("synthetic reply".equals(reply), "same login chat stream returns reply");
        check(calls.get() == 2 && deltas.get() == 1, "same login chat stream retries once and delivers delta");
    }

    private static void chatStreamFallsBackWithinTheSameLogin() throws Exception {
        MemoryStore store = new MemoryStore(activeSession());
        BackendAccount account = new BackendAccount(store, (method, path, body, token) -> {
            check(ACCESS.equals(token) && "/v1/chat/completions".equals(path), "fallback keeps request identity");
            check(!body.optBoolean("stream", true), "fallback disables streaming");
            return new JSONObject().put("choices", new org.json.JSONArray().put(new JSONObject()
                .put("message", new JSONObject().put("role", "assistant").put("content", "synthetic reply"))));
        }, null, (body, token, call, listener) -> {
            throw new BackendAccount.RequestException(400);
        });
        AtomicInteger deltas = new AtomicInteger();
        String reply = account.chatStream(chatRequest(), "synthetic-model", new BackendAccount.ChatCall(),
            delta -> deltas.incrementAndGet());
        check("synthetic reply".equals(reply) && deltas.get() == 1,
            "same login fallback delivers one complete reply");
    }

    private static void unauthorizedRefreshClearsSession() throws Exception {
        MemoryStore store = new MemoryStore(expiredSession());
        BackendAccount account = new BackendAccount(store, (method, path, body, token) -> {
            throw new BackendAccount.RequestException(401);
        });
        check(account.accessToken().isEmpty(), "unauthorized refresh has no token");
        check(store.load() == null, "unauthorized refresh clears session");
    }

    private static void unboundedPersistedExpiryIsRejected() throws Exception {
        MemoryStore store = new MemoryStore(new JSONObject().put("tokens", tokens(ACCESS, REFRESH))
            .put("expires_at_unix_ms", Long.MAX_VALUE).toString());
        AtomicInteger calls = new AtomicInteger();
        BackendAccount account = new BackendAccount(store, (method, path, body, token) -> {
            calls.incrementAndGet();
            return tokens(NEXT_ACCESS, NEXT_REFRESH);
        });
        check(account.accessToken().isEmpty(), "unbounded persisted expiry rejected");
        check(calls.get() == 0, "unbounded expiry does not use token or refresh");
    }

    @Override public void onCreate(Bundle arguments) {
        super.onCreate(arguments);
        start();
    }

    @Override public void onStart() {
        Bundle result = new Bundle();
        String stage = "refresh";
        try {
            refreshesExpiredSessionAndRotatesCredentials();
            concurrentCallersShareOneRefresh();
            refreshesAnUnexpiredRejectedToken();
            retriesAccountRequestsAfter401();
            oldRequestDoesNotRetryAfterNewLogin();
            oldRequestDiscardsSuccessAfterNewLogin();
            secondaryProcessRejectsChangedOwnerLogin();
            oldRequestDiscardsSuccessAfterSignOut();
            stage = "old stream retry";
            oldChatStreamDoesNotRetryAfterNewLogin();
            stage = "old stream delta";
            oldChatStreamDiscardsDeltaAfterNewLogin();
            stage = "old stream fallback";
            oldChatStreamDoesNotFallbackAfterNewLogin();
            stage = "same login stream refresh";
            chatStreamRetriesWithinTheSameLogin();
            stage = "same login stream fallback";
            chatStreamFallsBackWithinTheSameLogin();
            unauthorizedRefreshClearsSession();
            unboundedPersistedExpiryIsRejected();
            result.putString("stream", "MSIME_DEVICE_SMOKE_PASSED: account refresh, login lineage, chat stream and unauthorized clearing\n");
            finish(Activity.RESULT_OK, result);
        } catch (Exception | AssertionError error) {
            result.putString("stream", "MSIME_DEVICE_SMOKE_FAILED: account refresh " + stage + " ("
                + error.getClass().getSimpleName() + ": " + error.getMessage() + ")\n");
            finish(Activity.RESULT_CANCELED, result);
        }
    }
}
