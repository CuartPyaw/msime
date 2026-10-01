package app.msime.android;

import android.content.ContentProvider;
import android.content.ContentValues;
import android.content.Context;
import android.database.Cursor;
import android.net.Uri;
import android.os.Binder;
import android.os.Bundle;
import android.os.Process;

/**
 * Hands the signed-in account's access token to this app's other processes.
 *
 * <p>Declared in the main process and not exported, so only this uid can reach it. The token comes from {@link BackendAccount#owningSession}, which refreshes it when it is about to expire under the existing in-process lock; the `:ime` keyboard therefore never holds a refresh token and never refreshes, which is what keeps the rotating refresh token from being spent twice. The token is returned in the reply and nowhere else: nothing here logs it.
 */
public final class AccountSessionProvider extends ContentProvider {
    @Override public boolean onCreate() { return true; }

    @Override public Bundle call(String method, String arg, Bundle extras) {
        if (!AccountSessionRoutingPolicy.accepts(method, Binder.getCallingUid(), Process.myUid())) {
            throw new SecurityException("account session");
        }
        Context context = getContext();
        Bundle reply = new Bundle();
        reply.putString(AccountSessionRoutingPolicy.KEY_ACCESS_TOKEN,
            context == null ? "" : BackendAccount.owningSession(context).accessToken());
        return reply;
    }

    @Override public Cursor query(Uri uri, String[] projection, String selection,
                                  String[] selectionArgs, String sortOrder) {
        throw new UnsupportedOperationException();
    }

    @Override public String getType(Uri uri) { return null; }

    @Override public Uri insert(Uri uri, ContentValues values) {
        throw new UnsupportedOperationException();
    }

    @Override public int delete(Uri uri, String selection, String[] selectionArgs) {
        throw new UnsupportedOperationException();
    }

    @Override public int update(Uri uri, ContentValues values, String selection,
                                String[] selectionArgs) {
        throw new UnsupportedOperationException();
    }
}
