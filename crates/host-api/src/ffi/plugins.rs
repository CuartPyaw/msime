//! Key sounds, commit sounds, background music and sound-pack files for native hosts.
//!
//! Part of the C ABI; see the parent module for what these shims guarantee. The three sound calls sit on the key path, so they answer a plain bool instead of a JSON document the host would have to free: whether a request was queued. They never block, decode or touch the disk; `key_sound` says why.

use crate::*;
use key_sound::{KeyClass, PluginRoots, SessionSound};

/// Run `action` on the session's sound settings. False for an unknown handle, a wrong thread or a reentrant call, and after a panic, which must not cross the C boundary.
fn with_sound(handle: u64, action: impl FnOnce(&SessionSound) -> bool) -> bool {
    catch_unwind(AssertUnwindSafe(|| {
        SESSIONS.with(|sessions| {
            let Ok(sessions) = sessions.try_borrow() else {
                return false;
            };
            sessions
                .get(&handle)
                .is_some_and(|session| action(&session.sound))
        })
    }))
    .unwrap_or(false)
}

/// Queue the sound of one key press. `key_class` is 0 for any other key, 1 space, 2 enter, 3 backspace; anything else queues nothing.
#[no_mangle]
pub extern "C" fn msime_client_key_sound(handle: u64, key_class: u32) -> bool {
    let Some(class) = KeyClass::from_code(key_class) else {
        return false;
    };
    with_sound(handle, |sound| key_sound::key(sound, class))
}

/// Queue the sound of a commit: the key pack's commit sample, the melody's next note when it advances on commits, or both.
#[no_mangle]
pub extern "C" fn msime_client_commit_sound(handle: u64) -> bool {
    with_sound(handle, key_sound::commit)
}

/// Whether background music may play now: true while the input method is active in a field that is not a secure one, false otherwise.
#[no_mangle]
pub extern "C" fn msime_client_music_set_active(handle: u64, active: bool) -> bool {
    with_sound(handle, |sound| key_sound::music_active(sound, active))
}

/// The validated files of one sound pack, for a host that plays packs itself.
/// # Safety
/// `request` points to `length` readable UTF-8 JSON bytes. Null is rejected.
/// The returned response must be released with `msime_client_string_free`.
#[no_mangle]
pub unsafe extern "C" fn msime_client_key_sound_pack(
    request: *const u8,
    length: usize,
) -> *mut c_char {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Request {
        state_root: Option<String>,
        sound_packs: Option<String>,
        pack: String,
    }
    response(|| {
        // SAFETY: guaranteed by the documented caller contract; the length is bounded first.
        let request: Request = unsafe {
            with_bounded_bytes(
                request,
                length,
                65_536,
                "invalid sound pack request",
                |bytes| {
                    serde_json::from_slice(bytes)
                        .map_err(|_| "invalid sound pack request".to_owned())
                },
            )
        }?;
        let absolute = |path: &Option<String>| path.as_deref().is_none_or(absolute_path);
        if !absolute(&request.state_root) || !absolute(&request.sound_packs) {
            return Err("sound pack paths must be absolute".into());
        }
        let roots = PluginRoots::new(
            request.state_root.as_deref(),
            request.sound_packs.as_deref(),
            "",
        );
        key_sound::pack_files(&roots, &request.pack)
    })
}
