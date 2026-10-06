//! Updating a live session, tearing it down, and freeing what the library returned.
//!
//! Part of the C ABI; see the parent module for what these shims guarantee.

use crate::*;

/// Queue a validated preference snapshot without interrupting composition.
/// # Safety
/// `snapshot` must point to `length` readable bytes for this call. Null is rejected.
#[no_mangle]
pub unsafe extern "C" fn msime_client_update_preferences(
    handle: u64,
    snapshot: *const u8,
    length: usize,
) -> *mut c_char {
    response(|| {
        if snapshot.is_null() || length > PREFERENCES_DOCUMENT_LIMIT {
            return Err("invalid preferences buffer".into());
        }
        // SAFETY: guaranteed by the caller's buffer contract.
        let bytes = unsafe { std::slice::from_raw_parts(snapshot, length) };
        let snapshot = serde_json::from_slice(bytes).map_err(|_| "invalid preferences document")?;
        with_session(handle, |session| session.update(snapshot))
    })
}

#[no_mangle]
pub extern "C" fn msime_client_destroy(handle: u64) -> *mut c_char {
    response(|| {
        SESSIONS.with(|sessions| {
            let mut session = sessions
                .try_borrow_mut()
                .map_err(|_| "reentrant host call")?
                .remove(&handle)
                .ok_or("unknown session or wrong thread")?;
            // A host may tear a session down without a focus-out first; what it counted is still written.
            session.flush_selections();
            // Dropping the engine session writes its own journal's delayed context learning; this also covers every other journal the process learned into, while this session still holds its dictionary access.
            msime_engine::flush_personal_learning();
            drop(session);
            Ok(Value::Null)
        })
    })
}

/// Write what would otherwise wait: the selection counts of every session on the calling thread and every queued personal-context transition of the process. The C++ Engine wrote its queue from `atexit` (personal_ngram_store.cpp:254-255); a host that can exit without a focus-out or a destroy, such as macOS `[NSApp terminate:]`, calls this from its will-terminate hook on the thread that owns its sessions. Sessions stay usable.
#[no_mangle]
pub extern "C" fn msime_client_flush_all() -> *mut c_char {
    response(|| {
        SESSIONS.with(|sessions| {
            // A reentrant call (from inside another host call on this thread) cannot reach the sessions; the Engine's queue is still written.
            if let Ok(mut sessions) = sessions.try_borrow_mut() {
                for session in sessions.values_mut() {
                    session.flush_selections();
                }
            }
        });
        msime_engine::flush_personal_learning();
        Ok(Value::Null)
    })
}

/// # Safety
/// `value` must be null or an allocation returned by this library, not yet freed.
#[no_mangle]
pub unsafe extern "C" fn msime_client_string_free(value: *mut c_char) {
    if !value.is_null() {
        // SAFETY: ownership is transferred back exactly once by the C caller.
        drop(unsafe { CString::from_raw(value) });
    }
}
